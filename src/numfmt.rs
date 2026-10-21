//! How numbers are shown: the decimal mark, digit grouping and a fixed number
//! of decimals. Calculation and storage always use the canonical form with a
//! decimal point (`-1234.5`); only the screen and the clipboard are localized.
use crate::i18n::Language;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Groups digits in threes. A no-break space is unambiguous in both
/// languages, unlike `.` or `,`, which RustCalc reads as decimal marks, and
/// unlike narrower spaces it exists in every bundled font.
pub const GROUP_SEPARATOR: char = '\u{A0}';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecimalMark {
    /// A comma in Turkish, a point in English.
    #[default]
    Automatic,
    Point,
    Comma,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NumberStyle {
    pub mark: DecimalMark,
    pub grouping: bool,
    /// Fixed decimals for display, or `None` for every significant digit.
    pub decimals: Option<u32>,
}

impl Default for NumberStyle {
    fn default() -> Self {
        Self {
            mark: DecimalMark::Automatic,
            grouping: true,
            decimals: None,
        }
    }
}

impl NumberStyle {
    pub fn comma(self, language: Language) -> bool {
        match self.mark {
            DecimalMark::Automatic => language == Language::Turkish,
            DecimalMark::Point => false,
            DecimalMark::Comma => true,
        }
    }

    /// A canonical number as shown on screen. Text that is not a number (an
    /// error, `0x1F`) is returned unchanged.
    pub fn show(self, canonical: &str, language: Language) -> String {
        self.render(canonical, language, self.grouping)
    }

    /// The text placed on the clipboard: every digit with the chosen decimal
    /// mark and no grouping, so it pastes back into RustCalc and spreadsheets
    /// without losing what fixed decimals hide.
    pub fn copy(self, canonical: &str, language: Language) -> String {
        Self {
            decimals: None,
            ..self
        }
        .render(canonical, language, false)
    }

    fn render(self, canonical: &str, language: Language, grouping: bool) -> String {
        let Ok(mut number) = Decimal::from_str(canonical.trim()) else {
            return canonical.to_owned();
        };
        if let Some(decimals) = self.decimals {
            number =
                number.round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero);
            number.rescale(decimals);
        } else {
            number = number.normalize();
        }
        let text = number.to_string();
        let (sign, digits) = match text.strip_prefix('-') {
            // Rounding can leave "-0.00"; a zero has no sign.
            Some(rest) if rest.chars().any(|c| matches!(c, '1'..='9')) => ("-", rest),
            Some(rest) => ("", rest),
            None => ("", text.as_str()),
        };
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        let mut out = String::from(sign);
        for (i, digit) in whole.chars().enumerate() {
            if grouping && i > 0 && (whole.len() - i) % 3 == 0 {
                out.push(GROUP_SEPARATOR);
            }
            out.push(digit);
        }
        if !fraction.is_empty() {
            out.push(if self.comma(language) { ',' } else { '.' });
            out.push_str(fraction);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EN: Language = Language::English;
    const TR: Language = Language::Turkish;

    fn style(mark: DecimalMark, grouping: bool, decimals: Option<u32>) -> NumberStyle {
        NumberStyle {
            mark,
            grouping,
            decimals,
        }
    }

    #[test]
    fn grouping_and_marks_follow_the_language() {
        let auto = NumberStyle::default();
        assert_eq!(auto.show("1234567.891", EN), "1\u{A0}234\u{A0}567.891");
        assert_eq!(auto.show("1234567.891", TR), "1\u{A0}234\u{A0}567,891");
        assert_eq!(auto.show("-1234", EN), "-1\u{A0}234");
        assert_eq!(auto.show("999", EN), "999");
        assert_eq!(auto.show("0.5", TR), "0,5");
        assert_eq!(auto.show("2.500", EN), "2.5");
        let point = style(DecimalMark::Point, false, None);
        assert_eq!(point.show("1234.5", TR), "1234.5");
        let comma = style(DecimalMark::Comma, false, None);
        assert_eq!(comma.show("1234.5", EN), "1234,5");
    }

    #[test]
    fn fixed_decimals_round_half_up_and_pad() {
        let two = style(DecimalMark::Point, true, Some(2));
        assert_eq!(two.show("3", EN), "3.00");
        assert_eq!(two.show("2.675", EN), "2.68");
        assert_eq!(two.show("-0.001", EN), "0.00");
        assert_eq!(two.show("1234.5", EN), "1\u{A0}234.50");
        let none = style(DecimalMark::Point, false, Some(0));
        assert_eq!(none.show("2.5", EN), "3");
    }

    #[test]
    fn copies_are_ungrouped_and_parse_back() {
        let auto = NumberStyle::default();
        let copied = auto.copy("1234567.25", TR);
        assert_eq!(copied, "1234567,25");
        let back = crate::engine::evaluate(&copied, Default::default()).unwrap();
        assert_eq!(back.text(), "1234567.25");
        // Fixed decimals shape the screen, never the copy.
        let rounded = style(DecimalMark::Point, true, Some(0));
        assert_eq!(rounded.show("0.3333", EN), "0");
        assert_eq!(rounded.copy("0.3333", EN), "0.3333");
        // What is shown, grouping included, also parses back.
        let shown = auto.show("2469135.5", TR);
        let back = crate::engine::evaluate(&shown, Default::default()).unwrap();
        assert_eq!(back.text(), "2469135.5");
    }

    #[test]
    fn text_that_is_not_a_number_is_unchanged() {
        let auto = NumberStyle::default();
        assert_eq!(auto.show("Invalid date", EN), "Invalid date");
        assert_eq!(auto.show("", EN), "");
    }
}
