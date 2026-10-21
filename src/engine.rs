//! Expression compiler and evaluator. Input is parsed once by a bounded Pratt
//! parser into a small tree, then evaluated against a context and optional
//! named variables, so plots and formulas can evaluate one parse many times.
//! Decimal arithmetic stays decimal; transcendental results carry an
//! approximation marker all the way to the caller.
use crate::dates;
use rust_decimal::{Decimal, prelude::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

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
        // IEEE-754 operations provide ~15 significant decimal digits. Tiny values
        // keep fewer digits so the decimal scale stays within its 28 places.
        let exponent = if n == 0.0 {
            0
        } else {
            n.abs().log10().floor() as i32
        };
        if exponent < -28 {
            return Err(CalcError::new(
                "The result is smaller than the supported precision",
            ));
        }
        let digits = (28 + exponent).clamp(0, 14) as usize;
        let s = format!("{n:.digits$e}");
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
    fn truth(value: bool) -> Self {
        Self::exact(if value { Decimal::ONE } else { Decimal::ZERO })
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
    /// The date serial returned by `today()`; the local date when unset.
    pub today: Option<i64>,
}

/// Named values available to an expression, such as formula inputs.
pub type Variables = BTreeMap<String, Value>;

/// Functions with their accepted argument counts. Arguments are separated by
/// `;` because the comma is a decimal separator.
const FUNCTIONS: &[(&str, usize, usize)] = &[
    ("abs", 1, 1),
    ("floor", 1, 1),
    ("ceil", 1, 1),
    ("round", 1, 2),
    ("sqrt", 1, 1),
    ("cbrt", 1, 1),
    ("sin", 1, 1),
    ("cos", 1, 1),
    ("tan", 1, 1),
    ("asin", 1, 1),
    ("acos", 1, 1),
    ("atan", 1, 1),
    ("ln", 1, 1),
    ("log", 1, 2),
    ("exp", 1, 1),
    ("sinh", 1, 1),
    ("cosh", 1, 1),
    ("tanh", 1, 1),
    ("asinh", 1, 1),
    ("acosh", 1, 1),
    ("atanh", 1, 1),
    ("sign", 1, 1),
    ("min", 1, 64),
    ("max", 1, 64),
    ("sum", 1, 64),
    ("avg", 1, 64),
    ("gcd", 2, 64),
    ("lcm", 2, 64),
    ("ncr", 2, 2),
    ("npr", 2, 2),
    ("mod", 2, 2),
    ("if", 3, 3),
    ("and", 2, 64),
    ("or", 2, 64),
    ("not", 1, 1),
    ("date", 3, 3),
    ("today", 0, 0),
    ("year", 1, 1),
    ("month", 1, 1),
    ("day", 1, 1),
    ("weekday", 1, 1),
    ("addmonths", 2, 2),
    ("addyears", 2, 2),
    ("months", 2, 2),
    ("years", 2, 2),
    ("workdays", 2, 2),
    ("addworkdays", 2, 2),
    ("endofmonth", 1, 1),
];

/// Names a formula variable may not use.
pub fn is_reserved(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(name.as_str(), "pi" | "e" | "ans" | "mem")
        || FUNCTIONS.iter().any(|(function, ..)| *function == name)
}

/// A name the lexer reads as one identifier: a letter, then letters, digits or `_`.
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn function_names() -> impl Iterator<Item = &'static str> {
    FUNCTIONS.iter().map(|(name, ..)| *name)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Compare {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(Decimal),
    Name(String),
    Op(char),
    Compare(Compare),
    Left,
    Right,
    Separator,
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
        let next = chars.get(i + 1).copied();
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
        } else if c == 'π' {
            out.push(Token::Name("pi".into()));
            i += 1;
        } else if c.is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Name(
                chars[start..i].iter().collect::<String>().to_lowercase(),
            ));
        } else {
            let (token, width) = match (c, next) {
                ('<', Some('=')) | ('≤', _) => (
                    Token::Compare(Compare::LessEqual),
                    1 + usize::from(c == '<'),
                ),
                ('>', Some('=')) | ('≥', _) => (
                    Token::Compare(Compare::GreaterEqual),
                    1 + usize::from(c == '>'),
                ),
                ('<', Some('>')) => (Token::Compare(Compare::NotEqual), 2),
                ('!', Some('=')) => (Token::Compare(Compare::NotEqual), 2),
                ('≠', _) => (Token::Compare(Compare::NotEqual), 1),
                ('=', Some('=')) => (Token::Compare(Compare::Equal), 2),
                ('<', _) => (Token::Compare(Compare::Less), 1),
                ('>', _) => (Token::Compare(Compare::Greater), 1),
                ('(', _) => (Token::Left, 1),
                (')', _) => (Token::Right, 1),
                (';', _) => (Token::Separator, 1),
                ('+' | '-' | '*' | '/' | '^' | '%' | '!', _) => (Token::Op(c), 1),
                ('×' | '·', _) => (Token::Op('*'), 1),
                ('÷', _) => (Token::Op('/'), 1),
                ('−', _) => (Token::Op('-'), 1),
                _ => return Err(CalcError::new(format!("Unrecognized character: {c}"))),
            };
            out.push(token);
            i += width;
        }
    }
    out.push(Token::End);
    Ok(out)
}

#[derive(Clone, Debug)]
enum Node {
    Value(Value),
    Name(String),
    Negate(Box<Node>),
    /// A left operand followed by the operators one parser level applied in
    /// order. Long sums stay flat, so evaluation depth is bounded by nesting.
    Chain(Box<Node>, Vec<Step>),
    Call(String, Vec<Node>),
}

#[derive(Clone, Debug)]
enum Step {
    Binary(char, Node),
    Compare(Compare, Node),
    Percent,
    Factorial,
}

/// A parsed expression that can be evaluated repeatedly.
#[derive(Clone, Debug)]
pub struct Expression {
    root: Node,
}

impl Expression {
    pub fn parse(input: &str) -> Result<Self, CalcError> {
        let tokens = lex(input)?;
        if tokens.len() == 1 {
            return Err(CalcError::new("Enter an expression"));
        }
        let mut parser = Parser {
            tokens,
            at: 0,
            depth: 0,
        };
        let root = parser.expression(0)?;
        if parser.peek() != &Token::End {
            return Err(CalcError::new("Unexpected token or unmatched parenthesis"));
        }
        Ok(Self { root })
    }
    pub fn eval(&self, context: &Context, variables: &Variables) -> Result<Value, CalcError> {
        Evaluator { context, variables }.node(&self.root)
    }
    /// Variable names the expression reads, excluding `ans`, `mem` and `x`.
    pub fn names(&self) -> BTreeSet<String> {
        fn walk(node: &Node, names: &mut BTreeSet<String>) {
            match node {
                Node::Value(_) => {}
                Node::Name(name) => {
                    if !matches!(name.as_str(), "ans" | "mem" | "x") {
                        names.insert(name.clone());
                    }
                }
                Node::Negate(inner) => walk(inner, names),
                Node::Chain(first, steps) => {
                    walk(first, names);
                    for step in steps {
                        if let Step::Binary(_, node) | Step::Compare(_, node) = step {
                            walk(node, names);
                        }
                    }
                }
                Node::Call(_, args) => args.iter().for_each(|a| walk(a, names)),
            }
        }
        let mut names = BTreeSet::new();
        walk(&self.root, &mut names);
        names
    }
}

pub fn evaluate(input: &str, context: Context) -> Result<Value, CalcError> {
    evaluate_with(input, context, &Variables::new())
}

pub fn evaluate_with(
    input: &str,
    context: Context,
    variables: &Variables,
) -> Result<Value, CalcError> {
    Expression::parse(input)?.eval(&context, variables)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
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
    fn expression(&mut self, min_bp: u8) -> Result<Node, CalcError> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(CalcError::new("Expression too deep (maximum 64 levels)"));
        }
        let first = match self.take() {
            Token::Number(n) => Node::Value(Value::exact(n)),
            Token::Op('+') => self.expression(25)?,
            Token::Op('-') => Node::Negate(Box::new(self.expression(25)?)),
            Token::Left => {
                let v = self.expression(0)?;
                self.close()?;
                v
            }
            Token::Name(name) => match name.as_str() {
                // Full 28-digit constants, still marked approximate.
                "pi" => Node::Value(Value {
                    number: Decimal::from_i128_with_scale(
                        31_415_926_535_897_932_384_626_433_833,
                        28,
                    ),
                    approximate: true,
                }),
                "e" => Node::Value(Value {
                    number: Decimal::from_i128_with_scale(
                        27_182_818_284_590_452_353_602_874_714,
                        28,
                    ),
                    approximate: true,
                }),
                _ => match FUNCTIONS.iter().find(|(f, ..)| *f == name) {
                    Some(&(_, min, max)) => {
                        if self.take() != Token::Left {
                            return Err(CalcError::new(format!("Parentheses required for {name}")));
                        }
                        let args = self.arguments()?;
                        if args.len() < min || args.len() > max {
                            return Err(CalcError::new(format!(
                                "Wrong number of arguments for {name}"
                            )));
                        }
                        Node::Call(name, args)
                    }
                    // Any other name is a variable; `name(…)` multiplies implicitly.
                    None => Node::Name(name),
                },
            },
            _ => return Err(CalcError::new("Expected a number, constant or function")),
        };
        let mut steps = Vec::new();
        loop {
            if matches!(self.peek(), Token::Op('!' | '%')) && min_bp <= 40 {
                steps.push(if self.take() == Token::Op('%') {
                    Step::Percent
                } else {
                    Step::Factorial
                });
                continue;
            }
            if let Token::Compare(compare) = *self.peek() {
                if 5 < min_bp {
                    break;
                }
                self.take();
                steps.push(Step::Compare(compare, self.expression(6)?));
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
            steps.push(Step::Binary(op, self.expression(rbp)?));
        }
        self.depth -= 1;
        Ok(if steps.is_empty() {
            first
        } else {
            Node::Chain(Box::new(first), steps)
        })
    }
    fn arguments(&mut self) -> Result<Vec<Node>, CalcError> {
        let mut args = Vec::new();
        if self.peek() == &Token::Right {
            self.take();
            return Ok(args);
        }
        loop {
            args.push(self.expression(0)?);
            match self.take() {
                Token::Separator => continue,
                Token::Right => return Ok(args),
                _ => return Err(CalcError::new("Missing closing parenthesis")),
            }
        }
    }
    fn close(&mut self) -> Result<(), CalcError> {
        if self.take() == Token::Right {
            Ok(())
        } else {
            Err(CalcError::new("Missing closing parenthesis"))
        }
    }
}

struct Evaluator<'a> {
    context: &'a Context,
    variables: &'a Variables,
}
impl Evaluator<'_> {
    fn node(&self, node: &Node) -> Result<Value, CalcError> {
        match node {
            Node::Value(value) => Ok(*value),
            Node::Name(name) => {
                if let Some(value) = self.variables.get(name) {
                    return Ok(*value);
                }
                match name.as_str() {
                    "ans" => Ok(Value {
                        number: self.context.ans,
                        approximate: self.context.ans_approximate,
                    }),
                    "mem" => Ok(Value {
                        number: self.context.memory,
                        approximate: self.context.memory_approximate,
                    }),
                    "x" => Ok(Value::exact(self.context.x)),
                    _ => Err(CalcError::new(format!("Unknown name: {name}"))),
                }
            }
            Node::Negate(inner) => {
                let v = self.node(inner)?;
                Ok(Value {
                    number: -v.number,
                    ..v
                })
            }
            Node::Chain(first, steps) => {
                let mut left = self.node(first)?;
                for step in steps {
                    left = self.step(left, step)?;
                }
                Ok(left)
            }
            Node::Call(name, args) => self.call(name, args),
        }
    }
    fn step(&self, left: Value, step: &Step) -> Result<Value, CalcError> {
        match step {
            Step::Percent => {
                let number = checked(left.number.checked_div(Decimal::from(100)))?;
                reject_underflow(number, !left.number.is_zero())?;
                Ok(Value { number, ..left })
            }
            Step::Factorial => {
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
                Ok(Value {
                    number: result,
                    ..left
                })
            }
            Step::Compare(compare, right) => {
                let (a, b) = (left.number, self.node(right)?.number);
                Ok(Value::truth(match compare {
                    Compare::Less => a < b,
                    Compare::LessEqual => a <= b,
                    Compare::Greater => a > b,
                    Compare::GreaterEqual => a >= b,
                    Compare::Equal => a == b,
                    Compare::NotEqual => a != b,
                }))
            }
            Step::Binary(op @ ('+' | '-'), right) if percent_rate(right).is_some() => {
                // Calculator convention: 100 + 10% adds ten percent of 100 (110),
                // and 100 - 10% removes it (90).
                let (rate, negative) = percent_rate(right).expect("guarded");
                let rate = self.node(&rate)?;
                let share = checked(left.number.checked_mul(rate.number))?;
                let mut share = checked(share.checked_div(Decimal::ONE_HUNDRED))?;
                reject_underflow(share, !left.number.is_zero() && !rate.number.is_zero())?;
                if negative != (*op == '-') {
                    share = -share;
                }
                Ok(Value {
                    number: checked(left.number.checked_add(share))?,
                    approximate: left.approximate || rate.approximate,
                })
            }
            Step::Binary(op, right) => {
                let right = self.node(right)?;
                if *op == '^' {
                    return power(left, right);
                }
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
                Ok(Value {
                    number,
                    approximate: left.approximate || right.approximate,
                })
            }
        }
    }
    fn call(&self, name: &str, args: &[Node]) -> Result<Value, CalcError> {
        // Conditionals evaluate only the branch they need.
        match name {
            "if" => {
                let condition = self.node(&args[0])?;
                return self.node(&args[if condition.number.is_zero() { 2 } else { 1 }]);
            }
            "and" => {
                for arg in args {
                    if self.node(arg)?.number.is_zero() {
                        return Ok(Value::truth(false));
                    }
                }
                return Ok(Value::truth(true));
            }
            "or" => {
                for arg in args {
                    if !self.node(arg)?.number.is_zero() {
                        return Ok(Value::truth(true));
                    }
                }
                return Ok(Value::truth(false));
            }
            "today" => {
                return Ok(Value::exact(Decimal::from(
                    self.context.today.unwrap_or_else(dates::today),
                )));
            }
            _ => {}
        }
        let values = args
            .iter()
            .map(|arg| self.node(arg))
            .collect::<Result<Vec<_>, _>>()?;
        let approximate = values.iter().any(|v| v.approximate);
        let exact = |number: Decimal| Value {
            number,
            approximate,
        };
        let whole = |v: &Value| {
            v.number
                .fract()
                .is_zero()
                .then(|| v.number.to_i64())
                .flatten()
                .ok_or_else(|| CalcError::new(format!("Whole numbers required for {name}")))
        };
        let date = |v: &Value| {
            whole(v)
                .ok()
                .filter(|serial| dates::in_range(*serial))
                .ok_or_else(|| CalcError::new("Invalid date"))
        };
        let serial = |serial: Option<i64>| {
            serial
                .filter(|s| dates::in_range(*s))
                .map(|s| exact(Decimal::from(s)))
                .ok_or_else(|| CalcError::new("Invalid date"))
        };
        match name {
            "min" | "max" => Ok(*values
                .iter()
                .reduce(|a, b| {
                    if (name == "min") == (b.number < a.number) {
                        b
                    } else {
                        a
                    }
                })
                .expect("arity checked")),
            "sum" | "avg" => {
                let mut total = Decimal::ZERO;
                for v in &values {
                    total = checked(total.checked_add(v.number))?;
                }
                if name == "avg" {
                    total = checked(total.checked_div(Decimal::from(values.len())))?;
                }
                Ok(exact(total))
            }
            "gcd" | "lcm" => {
                let mut result: i128 = if name == "gcd" { 0 } else { 1 };
                for v in &values {
                    let n = v
                        .number
                        .fract()
                        .is_zero()
                        .then(|| v.number.to_i128())
                        .flatten()
                        .ok_or_else(|| {
                            CalcError::new(format!("Whole numbers required for {name}"))
                        })?
                        .abs();
                    result = if name == "gcd" {
                        gcd(result, n)
                    } else if n == 0 || result == 0 {
                        0
                    } else {
                        (result / gcd(result, n)).checked_mul(n).ok_or_else(|| {
                            CalcError::new("The result is outside the supported number range")
                        })?
                    };
                }
                Decimal::from_i128(result)
                    .filter(|_| result.unsigned_abs() <= Decimal::MAX.to_u128().unwrap_or(0))
                    .map(exact)
                    .ok_or_else(|| {
                        CalcError::new("The result is outside the supported number range")
                    })
            }
            "ncr" | "npr" => {
                let (n, k) = (whole(&values[0])?, whole(&values[1])?);
                if n < 0 || k < 0 || k > n {
                    return Err(CalcError::new(format!("Whole numbers required for {name}")));
                }
                let mut result = Decimal::ONE;
                if name == "npr" {
                    for i in 0..k {
                        result = checked(result.checked_mul(Decimal::from(n - i)))?;
                    }
                } else {
                    let k = k.min(n - k);
                    for i in 1..=k {
                        // Each partial product is itself a binomial coefficient.
                        result = checked(result.checked_mul(Decimal::from(n - k + i)))?;
                        result = checked(result.checked_div(Decimal::from(i)))?;
                    }
                }
                Ok(Value::exact(result))
            }
            "mod" => {
                let (a, b) = (values[0].number, values[1].number);
                if b.is_zero() {
                    return Err(CalcError::new("Division by zero is undefined"));
                }
                // The result takes the divisor's sign, as in mathematics.
                let mut r = checked(a.checked_rem(b))?;
                if !r.is_zero() && r.is_sign_negative() != b.is_sign_negative() {
                    r = checked(r.checked_add(b))?;
                }
                Ok(exact(r))
            }
            "not" => Ok(Value::truth(values[0].number.is_zero())),
            "sign" => Ok(Value::exact(Decimal::from(
                values[0].number.signum().to_i32().unwrap_or(0),
            ))),
            "round" => {
                let digits = match values.get(1) {
                    Some(v) => whole(v)?,
                    None => 0,
                };
                let digits = u32::try_from(digits)
                    .ok()
                    .filter(|d| *d <= 28)
                    .ok_or_else(|| CalcError::new("Rounding digits must be 0 to 28"))?;
                Ok(Value {
                    number: values[0].number.round_dp(digits),
                    approximate: values[0].approximate,
                })
            }
            "log" if values.len() == 2 => {
                let base = to_f64(values[1])?;
                if base <= 0.0 || base == 1.0 {
                    return Err(CalcError::new(
                        "The logarithm base must be positive and not 1",
                    ));
                }
                Value::float(to_f64(values[0])?.ln() / base.ln())
            }
            "date" => {
                let (y, m, d) = (whole(&values[0])?, whole(&values[1])?, whole(&values[2])?);
                serial(dates::from_ymd(
                    y,
                    u32::try_from(m).unwrap_or(0),
                    u32::try_from(d).unwrap_or(0),
                ))
            }
            "year" | "month" | "day" => {
                let (y, m, d) = dates::to_ymd(date(&values[0])?);
                Ok(Value::exact(Decimal::from(match name {
                    "year" => y,
                    "month" => m as i64,
                    _ => d as i64,
                })))
            }
            "weekday" => Ok(Value::exact(Decimal::from(dates::weekday(date(
                &values[0],
            )?)))),
            "endofmonth" => serial(Some(dates::end_of_month(date(&values[0])?))),
            "addworkdays" => {
                let (start, count) = (date(&values[0])?, whole(&values[1])?);
                if count.unsigned_abs() > 3_000_000 {
                    return Err(CalcError::new("Invalid date"));
                }
                serial(Some(dates::add_workdays(start, count)))
            }
            "addmonths" | "addyears" => {
                let start = date(&values[0])?;
                let count = whole(&values[1])?;
                let months = if name == "addyears" {
                    count.checked_mul(12)
                } else {
                    Some(count)
                };
                serial(months.and_then(|m| dates::add_months(start, m)))
            }
            "months" | "years" | "workdays" => {
                let (a, b) = (date(&values[0])?, date(&values[1])?);
                Ok(Value::exact(Decimal::from(match name {
                    "months" => dates::whole_months(a, b),
                    "years" => dates::whole_months(a, b) / 12,
                    _ => dates::workdays(a, b),
                })))
            }
            _ => self.scientific(name, values[0]),
        }
    }
    fn scientific(&self, name: &str, v: Value) -> Result<Value, CalcError> {
        let n = to_f64(v)?;
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
            // Hyperbolic functions take plain numbers, never degrees.
            "sinh" => n.sinh(),
            "cosh" => n.cosh(),
            "tanh" => n.tanh(),
            "asinh" => n.asinh(),
            "acosh" => n.acosh(),
            "atanh" => n.atanh(),
            _ => return Err(CalcError::new(format!("Unknown function: {name}"))),
        };
        // sin(π), cos(90°) and tan(180°) land on rounding residue near 1e-16;
        // below the input's float error floor a trigonometric zero is exact.
        let result = if matches!(name, "sin" | "cos" | "tan")
            && result.abs() < 4.0 * f64::EPSILON * angle.abs()
        {
            0.0
        } else {
            result
        };
        let value = Value::float(result)?;
        if !v.approximate && self.proven_exact(name, v.number, value.number) {
            return Ok(Value::exact(value.number));
        }
        Ok(value)
    }
    /// Whether a float result is exactly right, verified in decimal arithmetic,
    /// so `sqrt(144)`, `log(1000)` and `sin(30)` in degrees show no ≈.
    fn proven_exact(&self, name: &str, input: Decimal, result: Decimal) -> bool {
        let times = |a: Decimal, b: Decimal| a.checked_mul(b);
        let degrees = self.context.angle == Angle::Degrees;
        let simple =
            |n: Decimal| [Decimal::ZERO, Decimal::new(5, 1), Decimal::ONE].contains(&n.abs());
        match name {
            "sqrt" => times(result, result) == Some(input),
            "cbrt" => times(result, result).and_then(|r2| times(r2, result)) == Some(input),
            "log" => {
                result.fract().is_zero()
                    && result.abs() <= Decimal::from(28)
                    && power(Value::exact(Decimal::TEN), Value::exact(result))
                        .is_ok_and(|p| p.number == input)
            }
            "ln" => input == Decimal::ONE,
            "exp" => input.is_zero(),
            // Whole-degree angles whose sine, cosine or tangent is 0, ±½ or ±1.
            "sin" | "cos" | "tan" => {
                degrees
                    && input.fract().is_zero()
                    && (input % Decimal::from(15)).is_zero()
                    && simple(result)
            }
            "asin" | "acos" | "atan" => degrees && simple(input) && result.fract().is_zero(),
            _ => false,
        }
    }
}

/// For a right operand written as `b%` (or `-b%`), the expression for `b` and
/// whether it was negated. Only a trailing percent qualifies: `10% * 2` is an
/// ordinary product of 0.1 and 2.
fn percent_rate(node: &Node) -> Option<(Node, bool)> {
    match node {
        Node::Negate(inner) => percent_rate(inner).map(|(rate, negative)| (rate, !negative)),
        Node::Chain(first, steps) if matches!(steps.last(), Some(Step::Percent)) => {
            let rest = &steps[..steps.len() - 1];
            let rate = if rest.is_empty() {
                (**first).clone()
            } else {
                Node::Chain(first.clone(), rest.to_vec())
            };
            Some((rate, false))
        }
        _ => None,
    }
}

fn to_f64(v: Value) -> Result<f64, CalcError> {
    v.number
        .to_f64()
        .ok_or_else(|| CalcError::new("Number outside the supported range"))
}
fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
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
