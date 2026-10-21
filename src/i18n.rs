//! English source messages with a complete Turkish catalogue. Unknown external
//! messages fall back to English rather than disappearing from the interface.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Turkish,
}
impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Turkish => "tr",
        }
    }
    pub fn parse(code: &str) -> Result<Self, String> {
        match code {
            "en" => Ok(Self::English),
            "tr" => Ok(Self::Turkish),
            _ => Err("Unsupported language. Use en or tr.".into()),
        }
    }
    pub fn text(self, source: impl AsRef<str>) -> String {
        let source = source.as_ref();
        if self == Self::English {
            return source.to_owned();
        }
        static TURKISH: OnceLock<BTreeMap<String, String>> = OnceLock::new();
        let translations = TURKISH.get_or_init(|| {
            serde_json::from_str(include_str!("../locales/tr.json"))
                .expect("valid bundled Turkish catalogue")
        });
        if let Some(text) = translations.get(source) {
            return text.clone();
        }
        for (prefix, template, placeholder) in [
            (
                "Unrecognized character: ",
                "Unrecognized character: {c}",
                "{c}",
            ),
            ("Unknown function: ", "Unknown function: {name}", "{name}"),
            (
                "Parentheses required for ",
                "Parentheses required for {name}",
                "{name}",
            ),
            ("Allowed range: ", "Allowed range: {range}", "{range}"),
            (
                "Bit pattern too wide; maximum bits: ",
                "Bit pattern too wide; maximum bits: {bits}",
                "{bits}",
            ),
            (
                "Shift amount must be 0 to ",
                "Shift amount must be 0 to {max}",
                "{max}",
            ),
            (
                "RustCalc update available: ",
                "RustCalc update available: {version}",
                "{version}",
            ),
            ("Update to ", "Update to {version}", "{version}"),
        ] {
            if let Some(value) = source.strip_prefix(prefix)
                && let Some(text) = translations.get(template)
            {
                return text.replace(placeholder, value);
            }
        }
        source.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_catalogues_have_matching_nonempty_messages() {
        let en: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        let tr: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../locales/tr.json")).unwrap();
        assert_eq!(en.keys().collect::<Vec<_>>(), tr.keys().collect::<Vec<_>>());
        assert!(en.len() > 150);
        for (key, value) in &en {
            assert_eq!(key, value);
            assert!(!tr[key].trim().is_empty());
        }
    }
    #[test]
    fn dynamic_diagnostics_are_translated_without_losing_the_value() {
        assert_eq!(
            Language::Turkish.text("Unknown function: foo"),
            "Bilinmeyen fonksiyon: foo"
        );
        assert_eq!(
            Language::Turkish.text("Parentheses required for sin"),
            "sin için parantez gerekli"
        );
        assert_eq!(
            Language::English.text("Division by zero is undefined"),
            "Division by zero is undefined"
        );
        assert_eq!(Language::Turkish.text("External error"), "External error");
        for (source, turkish) in [
            (
                "Allowed range: -128 … 127",
                "İzin verilen aralık: -128 … 127",
            ),
            (
                "Bit pattern too wide; maximum bits: 16",
                "Bit deseni çok geniş; en fazla bit: 16",
            ),
            (
                "Shift amount must be 0 to 32",
                "Kaydırma miktarı 0 ile 32 arasında olmalı",
            ),
            ("Update to 1.2.3", "1.2.3 sürümüne güncelle"),
            (
                "RustCalc update available: 1.2.3",
                "RustCalc güncellemesi hazır: 1.2.3",
            ),
        ] {
            assert_eq!(Language::Turkish.text(source), turkish);
        }
    }
}
