//! Fixed-width integer arithmetic for the programmer workspace. Values are kept
//! as raw bit patterns; the word decides width and whether DEC is signed.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Word {
    pub bits: u32,
    pub signed: bool,
}
impl Default for Word {
    fn default() -> Self {
        Self {
            bits: 64,
            signed: false,
        }
    }
}

pub const SIZES: [u32; 4] = [8, 16, 32, 64];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Not,
    Neg,
    Shl,
    Shr,
}

impl Word {
    pub fn mask(self) -> u64 {
        if self.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        }
    }
    /// The pattern as a signed number when the word is signed (two's complement).
    pub fn signed_value(self, pattern: u64) -> i128 {
        let pattern = pattern & self.mask();
        if self.signed && pattern >> (self.bits - 1) & 1 == 1 {
            pattern as i128 - (1i128 << self.bits)
        } else {
            pattern as i128
        }
    }
    fn range(self) -> (i128, i128) {
        if self.signed {
            (-(1i128 << (self.bits - 1)), (1i128 << (self.bits - 1)) - 1)
        } else {
            (0, self.mask() as i128)
        }
    }
    fn range_error(self) -> String {
        let (min, max) = self.range();
        format!("Allowed range: {min} … {max}")
    }
    /// DEC accepts a sign within the word's numeric range; HEX/OCT/BIN describe
    /// a bit pattern that must fit in the word.
    pub fn parse(self, input: &str, radix: u32) -> Result<u64, String> {
        if ![2, 8, 10, 16].contains(&radix) {
            return Err("Unsupported number base".into());
        }
        let text = input.trim().replace(['_', ' '], "");
        if text.is_empty() {
            return Err("Enter a whole number".into());
        }
        if radix == 10 {
            let value: i128 = text
                .parse()
                .map_err(|_| "Enter a whole number".to_owned())?;
            let (min, max) = self.range();
            if value < min || value > max {
                return Err(self.range_error());
            }
            return Ok(value as u64 & self.mask());
        }
        let lower = text.to_ascii_lowercase();
        let digits = match radix {
            16 => lower.strip_prefix("0x"),
            8 => lower.strip_prefix("0o"),
            _ => lower.strip_prefix("0b"),
        }
        .unwrap_or(&lower);
        let value = u64::from_str_radix(digits, radix).map_err(|_| {
            if digits.chars().all(|c| c.is_digit(radix)) && !digits.is_empty() {
                format!("Bit pattern too wide; maximum bits: {}", self.bits)
            } else {
                "Invalid digit for this number base".to_owned()
            }
        })?;
        if value > self.mask() {
            return Err(format!("Bit pattern too wide; maximum bits: {}", self.bits));
        }
        Ok(value)
    }
    pub fn format(self, pattern: u64, radix: u32) -> String {
        let pattern = pattern & self.mask();
        match radix {
            16 => format!("{pattern:X}"),
            8 => format!("{pattern:o}"),
            2 => format!("{pattern:b}"),
            _ => self.signed_value(pattern).to_string(),
        }
    }
    /// Binary grouped in nibbles and padded to the word width.
    pub fn grouped_binary(self, pattern: u64) -> String {
        let text = format!(
            "{:0width$b}",
            pattern & self.mask(),
            width = self.bits as usize
        );
        text.as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ")
    }
    /// Results wrap to the word, as in hardware. Shifts by the operand count;
    /// a right shift is arithmetic (sign-preserving) for signed words.
    pub fn apply(self, op: Op, a: u64, b: u64) -> Result<u64, String> {
        let (sa, sb) = (self.signed_value(a), self.signed_value(b));
        let wrap = |v: i128| (v as u64) & self.mask();
        let shift = || {
            u32::try_from(sb)
                .ok()
                .filter(|n| *n <= self.bits)
                .ok_or_else(|| format!("Shift amount must be 0 to {}", self.bits))
        };
        Ok(match op {
            Op::Add => wrap(sa.wrapping_add(sb)),
            Op::Sub => wrap(sa.wrapping_sub(sb)),
            Op::Mul => wrap(sa.wrapping_mul(sb)),
            Op::Div | Op::Mod => {
                if sb == 0 {
                    return Err("Division by zero is undefined".into());
                }
                // Truncating division; MIN / -1 wraps like the hardware result.
                wrap(if op == Op::Div { sa / sb } else { sa % sb })
            }
            Op::And => a & b & self.mask(),
            Op::Or => (a | b) & self.mask(),
            Op::Xor => (a ^ b) & self.mask(),
            Op::Not => !a & self.mask(),
            Op::Neg => wrap(-sa),
            Op::Shl => {
                let n = shift()?;
                if n >= 64 { 0 } else { (a << n) & self.mask() }
            }
            Op::Shr => {
                let n = shift()?;
                if self.signed {
                    wrap(sa >> n.min(127))
                } else if n >= 64 {
                    0
                } else {
                    (a & self.mask()) >> n
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const S8: Word = Word {
        bits: 8,
        signed: true,
    };
    const U8: Word = Word {
        bits: 8,
        signed: false,
    };

    #[test]
    fn decimal_input_respects_the_signed_or_unsigned_range() {
        assert_eq!(S8.parse("-1", 10), Ok(0xFF));
        assert_eq!(S8.parse("-128", 10), Ok(0x80));
        assert_eq!(S8.parse("127", 10), Ok(0x7F));
        assert_eq!(S8.parse("128", 10), Err("Allowed range: -128 … 127".into()));
        assert!(U8.parse("-1", 10).is_err());
        assert_eq!(U8.parse("255", 10), Ok(255));
        assert_eq!(
            Word::default().parse("18446744073709551615", 10),
            Ok(u64::MAX)
        );
        assert!(Word::default().parse("18446744073709551616", 10).is_err());
        let s64 = Word {
            bits: 64,
            signed: true,
        };
        assert_eq!(s64.parse("-9223372036854775808", 10), Ok(1 << 63));
        assert!(U8.parse("1.5", 10).is_err());
        assert!(U8.parse("", 10).is_err());
    }

    #[test]
    fn patterns_must_fit_the_word_and_display_by_signedness() {
        assert_eq!(S8.parse("0xff", 16), Ok(0xFF));
        assert_eq!(S8.parse("1111_0000", 2), Ok(0xF0));
        assert_eq!(S8.parse("377", 8), Ok(0xFF));
        assert_eq!(
            U8.parse("1FF", 16),
            Err("Bit pattern too wide; maximum bits: 8".into())
        );
        assert_eq!(
            U8.parse("12G", 16),
            Err("Invalid digit for this number base".into())
        );
        assert_eq!(S8.format(0xFF, 10), "-1");
        assert_eq!(U8.format(0xFF, 10), "255");
        assert_eq!(S8.format(0xFF, 16), "FF");
        assert_eq!(S8.format(0x1FF, 2), "11111111");
        assert_eq!(U8.grouped_binary(5), "0000 0101");
    }

    #[test]
    fn arithmetic_wraps_to_the_word() {
        assert_eq!(S8.apply(Op::Add, 0x7F, 1), Ok(0x80));
        assert_eq!(S8.format(S8.apply(Op::Add, 0x7F, 1).unwrap(), 10), "-128");
        assert_eq!(U8.apply(Op::Add, 0xFF, 1), Ok(0));
        assert_eq!(U8.apply(Op::Sub, 0, 1), Ok(0xFF));
        assert_eq!(S8.apply(Op::Mul, 16, 16), Ok(0));
        // -7 / 2 truncates toward zero; the remainder keeps the dividend's sign.
        let minus7 = S8.parse("-7", 10).unwrap();
        assert_eq!(S8.format(S8.apply(Op::Div, minus7, 2).unwrap(), 10), "-3");
        assert_eq!(S8.format(S8.apply(Op::Mod, minus7, 2).unwrap(), 10), "-1");
        assert_eq!(U8.format(U8.apply(Op::Div, minus7, 2).unwrap(), 10), "124");
        assert_eq!(S8.apply(Op::Div, 0x80, 0xFF), Ok(0x80));
        assert!(S8.apply(Op::Mod, 1, 0).is_err());
        assert_eq!(S8.apply(Op::Neg, 0x80, 0), Ok(0x80));
        assert_eq!(S8.apply(Op::Neg, 1, 0), Ok(0xFF));
        let u64w = Word::default();
        assert_eq!(u64w.apply(Op::Mul, u64::MAX, 2), Ok(u64::MAX - 1));
    }

    #[test]
    fn bitwise_operations_and_shifts_follow_the_word() {
        assert_eq!(U8.apply(Op::Not, 0x0F, 0), Ok(0xF0));
        assert_eq!(Word::default().apply(Op::Not, 0, 0), Ok(u64::MAX));
        assert_eq!(U8.apply(Op::And, 0xFC, 0x0F), Ok(0x0C));
        assert_eq!(U8.apply(Op::Or, 0xF0, 0x0F), Ok(0xFF));
        assert_eq!(U8.apply(Op::Xor, 0xFF, 0x0F), Ok(0xF0));
        assert_eq!(U8.apply(Op::Shl, 0x81, 1), Ok(0x02));
        assert_eq!(U8.apply(Op::Shr, 0x80, 7), Ok(1));
        assert_eq!(S8.apply(Op::Shr, 0x80, 7), Ok(0xFF));
        assert_eq!(S8.apply(Op::Shr, 0x80, 8), Ok(0xFF));
        assert_eq!(U8.apply(Op::Shl, 1, 8), Ok(0));
        assert_eq!(Word::default().apply(Op::Shl, 1, 64), Ok(0));
        assert_eq!(Word::default().apply(Op::Shr, u64::MAX, 64), Ok(0));
        assert!(U8.apply(Op::Shl, 1, 9).is_err());
        assert!(S8.apply(Op::Shr, 1, 0xFF).is_err());
    }
}
