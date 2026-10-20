//! Bounded Pratt parser. Decimal arithmetic stays decimal; transcendental results
//! carry an approximation marker all the way to the caller.
use rust_decimal::{Decimal, prelude::*};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Angle {
    #[default]
    Degrees,
    Radians,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    pub number: Decimal,
    pub approximate: bool,
}

impl Value {
    pub fn exact(number: Decimal) -> Self {
        Self {
            number,
            approximate: false,
        }
    }
    pub fn text(self) -> String {
        self.number.normalize().to_string()
    }
    fn float(n: f64) -> Result<Self, CalcError> {
        if !n.is_finite() {
            return Err(CalcError::new("The result is not a finite real number"));
        }
        // IEEE-754 operations provide ~15 significant decimal digits.
        let s = format!("{n:.14e}");
        let number = Decimal::from_scientific(&s)
            .map_err(|_| CalcError::new("The result is outside the supported number range"))?;
        if n != 0.0 && number.is_zero() {
            return Err(CalcError::new(
                "The result is smaller than the supported precision",
            ));
        }
        Ok(Self {
            number,
            approximate: true,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalcError {
    pub message: String,
}
impl CalcError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for CalcError {}

#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
    pub angle: Angle,
    pub ans: Decimal,
    pub memory: Decimal,
    pub x: Decimal,
    pub ans_approximate: bool,
    pub memory_approximate: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(Decimal),
    Name(String),
    Op(char),
    Left,
    Right,
    End,
}

fn lex(input: &str) -> Result<Vec<Token>, CalcError> {
    if input.len() > 4096 {
        return Err(CalcError::new("Expression too long (maximum 4096 bytes)"));
    }
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' || c == ',' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',')
            {
                i += 1;
            }
            // Only an e followed by an exponent belongs to a numeric literal.
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                let mut j = i + 1;
                if j < chars.len() && matches!(chars[j], '+' | '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j + 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let literal: String = chars[start..i].iter().collect::<String>().replace(',', ".");
            let number = if literal.contains(['e', 'E']) {
                Decimal::from_scientific(&literal)
            } else {
                Decimal::from_str_exact(&literal)
            }
            .map_err(|_| CalcError::new("Invalid number or unsupported precision"))?;
            if number.is_zero()
                && literal
                    .chars()
                    .take_while(|c| !matches!(c, 'e' | 'E'))
                    .any(|c| matches!(c, '1'..='9'))
            {
                return Err(CalcError::new(
                    "The number is smaller than the supported precision",
                ));
            }
            out.push(Token::Number(number));
        } else if c.is_ascii_alphabetic() || c == 'π' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            out.push(Token::Name(
                chars[start..i].iter().collect::<String>().to_lowercase(),
            ));
        } else {
            out.push(match c {
                '(' => Token::Left,
                ')' => Token::Right,
                '+' | '-' | '*' | '/' | '^' | '%' | '!' => Token::Op(c),
                '×' | '·' => Token::Op('*'),
                '÷' => Token::Op('/'),
                '−' => Token::Op('-'),
                _ => return Err(CalcError::new(format!("Unrecognized character: {c}"))),
            });
            i += 1;
        }
    }
    out.push(Token::End);
    Ok(out)
}

pub fn evaluate(input: &str, context: Context) -> Result<Value, CalcError> {
    let tokens = lex(input)?;
    if tokens.len() == 1 {
        return Err(CalcError::new("Enter an expression"));
    }
    let mut parser = Parser {
        tokens,
        at: 0,
        context,
        depth: 0,
    };
    let value = parser.expression(0)?;
    if parser.peek() != &Token::End {
        return Err(CalcError::new("Unexpected token or unmatched parenthesis"));
    }
    Ok(value)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    context: Context,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn take(&mut self) -> Token {
        let t = self.tokens[self.at].clone();
        self.at += 1;
        t
    }
    fn expression(&mut self, min_bp: u8) -> Result<Value, CalcError> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(CalcError::new("Expression too deep (maximum 64 levels)"));
        }
        let mut left = match self.take() {
            Token::Number(n) => Value::exact(n),
            Token::Op('+') => self.expression(25)?,
            Token::Op('-') => {
                let v = self.expression(25)?;
                Value {
                    number: -v.number,
                    ..v
                }
            }
            Token::Left => {
                let v = self.expression(0)?;
                self.close()?;
                v
            }
            Token::Name(name) => match name.as_str() {
                "pi" | "π" => Value::float(std::f64::consts::PI)?,
                "e" => Value::float(std::f64::consts::E)?,
                "ans" => Value {
                    number: self.context.ans,
                    approximate: self.context.ans_approximate,
                },
                "mem" => Value {
                    number: self.context.memory,
                    approximate: self.context.memory_approximate,
                },
                "x" => Value::exact(self.context.x),
                _ => {
                    if self.take() != Token::Left {
                        return Err(CalcError::new(format!("Parentheses required for {name}")));
                    }
                    let v = self.expression(0)?;
                    self.close()?;
                    self.function(&name, v)?
                }
            },
            _ => return Err(CalcError::new("Expected a number, constant or function")),
        };
        loop {
            if matches!(self.peek(), Token::Op('!' | '%')) && min_bp <= 40 {
                let op = self.take();
                left = if op == Token::Op('%') {
                    let number = checked(left.number.checked_div(Decimal::from(100)))?;
                    reject_underflow(number, !left.number.is_zero())?;
                    Value { number, ..left }
                } else {
                    if !left.number.fract().is_zero() || left.number.is_sign_negative() {
                        return Err(CalcError::new("Factorial requires a non-negative integer"));
                    }
                    let n = left
                        .number
                        .to_u32()
                        .filter(|n| *n <= 27)
                        .ok_or_else(|| CalcError::new("Factorial limit: 27"))?;
                    let mut result = Decimal::ONE;
                    for i in 2..=n {
                        result = checked(result.checked_mul(Decimal::from(i)))?;
                    }
                    Value {
                        number: result,
                        ..left
                    }
                };
                continue;
            }
            let (op, lbp, rbp, implicit) = match self.peek() {
                Token::Op('+') => ('+', 10, 11, false),
                Token::Op('-') => ('-', 10, 11, false),
                Token::Op('*') => ('*', 20, 21, false),
                Token::Op('/') => ('/', 20, 21, false),
                Token::Op('^') => ('^', 30, 30, false),
                Token::Name(_) | Token::Left => ('*', 20, 21, true),
                _ => break,
            };
            if lbp < min_bp {
                break;
            }
            if !implicit {
                self.take();
            }
            let right = self.expression(rbp)?;
            let approximate = left.approximate || right.approximate;
            left = if op == '^' {
                power(left, right)?
            } else {
                let n = match op {
                    '+' => left.number.checked_add(right.number),
                    '-' => left.number.checked_sub(right.number),
                    '*' => left.number.checked_mul(right.number),
                    '/' => {
                        if right.number.is_zero() {
                            return Err(CalcError::new("Division by zero is undefined"));
                        }
                        left.number.checked_div(right.number)
                    }
                    _ => unreachable!(),
                };
                let number = checked(n)?;
                reject_underflow(
                    number,
                    !left.number.is_zero() && !right.number.is_zero() && matches!(op, '*' | '/'),
                )?;
                Value {
                    number,
                    approximate,
                }
            };
        }
        self.depth -= 1;
        Ok(left)
    }
    fn close(&mut self) -> Result<(), CalcError> {
        if self.take() == Token::Right {
            Ok(())
        } else {
            Err(CalcError::new("Missing closing parenthesis"))
        }
    }
    fn function(&self, name: &str, v: Value) -> Result<Value, CalcError> {
        let n = v
            .number
            .to_f64()
            .ok_or_else(|| CalcError::new("Number outside the supported range"))?;
        let angle = if self.context.angle == Angle::Degrees {
            n.to_radians()
        } else {
            n
        };
        let inverse = |x: f64| {
            if self.context.angle == Angle::Degrees {
                x.to_degrees()
            } else {
                x
            }
        };
        let result = match name {
            "abs" => {
                return Ok(Value {
                    number: v.number.abs(),
                    ..v
                });
            }
            "floor" => {
                return Ok(Value {
                    number: v.number.floor(),
                    ..v
                });
            }
            "ceil" => {
                return Ok(Value {
                    number: v.number.ceil(),
                    ..v
                });
            }
            "round" => {
                return Ok(Value {
                    number: v.number.round(),
                    ..v
                });
            }
            "sqrt" => n.sqrt(),
            "cbrt" => n.cbrt(),
            "sin" => angle.sin(),
            "cos" => angle.cos(),
            "tan" => {
                if angle.cos().abs() < 1e-14 {
                    return Err(CalcError::new("Tangent is undefined at this angle"));
                }
                angle.tan()
            }
            "asin" => inverse(n.asin()),
            "acos" => inverse(n.acos()),
            "atan" => inverse(n.atan()),
            "ln" => n.ln(),
            "log" => n.log10(),
            "exp" => n.exp(),
            _ => return Err(CalcError::new(format!("Unknown function: {name}"))),
        };
        Value::float(result)
    }
}

fn checked(n: Option<Decimal>) -> Result<Decimal, CalcError> {
    n.ok_or_else(|| CalcError::new("The result is outside the supported number range"))
}
fn reject_underflow(number: Decimal, nonzero_expected: bool) -> Result<(), CalcError> {
    if number.is_zero() && nonzero_expected {
        Err(CalcError::new(
            "The result is smaller than the supported precision",
        ))
    } else {
        Ok(())
    }
}

fn power(a: Value, b: Value) -> Result<Value, CalcError> {
    if b.number.fract().is_zero() {
        let exponent = b
            .number
            .to_i32()
            .filter(|n| n.unsigned_abs() <= 10_000)
            .ok_or_else(|| CalcError::new("Exponent limit: ±10000"))?;
        if a.number.is_zero() && exponent <= 0 {
            return Err(CalcError::new("This power is undefined"));
        }
        let mut n = exponent.unsigned_abs();
        let mut base = if exponent < 0 {
            checked(Decimal::ONE.checked_div(a.number))?
        } else {
            a.number
        };
        let mut result = Decimal::ONE;
        while n > 0 {
            if n & 1 == 1 {
                result = checked(result.checked_mul(base))?;
            }
            n >>= 1;
            if n > 0 {
                base = checked(base.checked_mul(base))?;
            }
        }
        if result.is_zero() && !a.number.is_zero() {
            return Err(CalcError::new(
                "The result is smaller than the supported precision",
            ));
        }
        Ok(Value {
            number: result,
            approximate: a.approximate || b.approximate,
        })
    } else {
        Value::float(
            a.number
                .to_f64()
                .unwrap_or(f64::NAN)
                .powf(b.number.to_f64().unwrap_or(f64::NAN)),
        )
    }
}

pub fn parse_integer(input: &str, radix: u32) -> Result<u64, String> {
    if ![2, 8, 10, 16].contains(&radix) {
        return Err("Unsupported number base".into());
    }
    let text = input.trim().replace(['_', ' '], "");
    let text = match radix {
        16 => text
            .strip_prefix("0x")
            .or_else(|| text.strip_prefix("0X"))
            .unwrap_or(&text),
        2 => text.strip_prefix("0b").unwrap_or(&text),
        8 => text.strip_prefix("0o").unwrap_or(&text),
        _ => &text,
    };
    u64::from_str_radix(text, radix)
        .map_err(|_| "Invalid number; use the unsigned 64-bit range".into())
}
