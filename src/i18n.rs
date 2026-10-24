// SPDX-License-Identifier: AGPL-3.0-only
//! Interface language selection and translation catalogs.
//!
//! English source strings are the catalog keys (gettext style), so untranslated text falls back
//! to English. Catalogs live in `locales/<code>.json`; `tests/i18n.rs` keeps them complete.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    sync::{
        atomic::{AtomicU8, Ordering},
        OnceLock,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "pt-BR")]
    Portuguese,
}
impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Portuguese];
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Portuguese => "pt-BR",
        }
    }
    /// Name shown in the language menu, always in the language itself.
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português (Brasil)",
        }
    }
    fn catalog(self) -> Option<&'static HashMap<&'static str, &'static str>> {
        static PORTUGUESE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
        match self {
            Self::English => None,
            Self::Portuguese => {
                Some(PORTUGUESE.get_or_init(|| parse(include_str!("../locales/pt-BR.json"))))
            }
        }
    }
}

/// Parses a catalog once; entries live for the whole process.
fn parse(json: &'static str) -> HashMap<&'static str, &'static str> {
    let entries: HashMap<String, String> =
        serde_json::from_str(json).expect("embedded translation catalog must be valid JSON");
    entries
        .into_iter()
        .map(|(k, v)| -> (&'static str, &'static str) {
            (Box::leak(k.into_boxed_str()), Box::leak(v.into_boxed_str()))
        })
        .collect()
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_language(language: Language) {
    let index = Language::ALL.iter().position(|l| *l == language).unwrap();
    CURRENT.store(index as u8, Ordering::Relaxed);
}
pub fn language() -> Language {
    Language::ALL
        .get(CURRENT.load(Ordering::Relaxed) as usize)
        .copied()
        .unwrap_or_default()
}

/// Looks up `english` in a specific catalog, falling back to the English text.
pub fn translate(language: Language, english: &'static str) -> &'static str {
    language
        .catalog()
        .and_then(|c| c.get(english).copied())
        .unwrap_or(english)
}

/// Translates `english` into the current interface language.
pub fn tr(english: &'static str) -> &'static str {
    translate(language(), english)
}

/// Replaces `{name}` placeholders. Translators may reorder placeholders freely.
pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut text = template.to_owned();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

/// Translates a template and fills named placeholders:
/// `trf!("Connecting to {port}…", port = name)`. Pre-format numbers that need a precision.
#[macro_export]
macro_rules! trf {
    ($key:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::i18n::fill(
            $crate::i18n::tr($key),
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),*],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_is_the_default_and_fallback() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(translate(Language::English, "Save project"), "Save project");
        assert_eq!(
            translate(Language::Portuguese, "text that is not in any catalog"),
            "text that is not in any catalog"
        );
    }
    #[test]
    fn portuguese_catalog_translates_and_placeholders_can_move() {
        assert_eq!(
            translate(Language::Portuguese, "Save project"),
            "Salvar projeto"
        );
        assert_eq!(
            fill("{b} then {a}", &[("a", &1), ("b", &"two")]),
            "two then 1"
        );
    }
    #[test]
    fn language_codes_round_trip_through_serde() {
        for language in Language::ALL {
            let json = serde_json::to_string(&language).unwrap();
            assert_eq!(json, format!("\"{}\"", language.code()));
            assert_eq!(serde_json::from_str::<Language>(&json).unwrap(), language);
        }
    }
}
