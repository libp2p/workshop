use crate::Error;
use serde::{Deserialize, Serialize};

/// A spoken language with country code, name in both English and native language.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Language {
    pub code: Code,
    pub name: String,
    pub native_name: String,
    pub direction: Direction,
}

impl Default for Language {
    fn default() -> Self {
        Language {
            code: Code::en,
            name: "English".to_string(),
            native_name: "English".to_string(),
            direction: Direction::LeftToRight,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<Code> for Language {
    fn from(code: Code) -> Self {
        let name = code.get_name_in_english();
        let native_name = code.get_name_in_native();
        let direction = code.get_text_direction();
        Language {
            code,
            name: name.to_string(),
            native_name: native_name.to_string(),
            direction,
        }
    }
}

impl TryFrom<&str> for Language {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match Code::try_from(value) {
            Ok(code) => {
                let name = code.get_name_in_english();
                let native_name = code.get_name_in_native();

                Ok(Language {
                    code,
                    name: name.to_string(),
                    native_name: native_name.to_string(),
                    direction: code.get_text_direction(),
                })
            }
            Err(_) => Err(Error::InvalidLanguageName(value.to_string())),
        }
    }
}

impl TryFrom<&str> for Code {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some(code) = get_language_code(value) {
            Ok(code)
        } else if let Some(code) = get_language_code_from_name(value) {
            Ok(code)
        } else {
            Err(Error::InvalidLanguageCode(value.to_string()))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    #[default]
    LeftToRight,
    RightToLeft,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::LeftToRight => write!(f, "LeftToRight"),
            Direction::RightToLeft => write!(f, "RightToLeft"),
        }
    }
}

macro_rules! generate_spoken_enum {
    ($(($code:ident, $english:literal, $native:literal, $direction:ident)),* $(,)?) => {
        /// The list of language codes
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub enum Code {
            $(
                #[allow(non_camel_case_types)]
                $code,
            )*
        }

        impl Code {
            // Get the name in English
            pub fn get_name_in_english(&self) -> &'static str {
                match &self {
                    $(
                        Code::$code => $english,
                    )*
                }
            }

            // Get the name in the language
            pub fn get_name_in_native(&self) -> &'static str {
                match &self {
                    $(
                        Code::$code => $native,
                    )*
                }
            }

            // Get the text direction for the language
            pub fn get_text_direction(&self) -> Direction {
                match &self {
                    $(
                        Code::$code => Direction::$direction,
                    )*
                }
            }
        }

        impl Default for Code {
            fn default() -> Self {
                Code::en
            }
        }

        impl std::fmt::Display for Code {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Code::$code => write!(f, "{}", stringify!($code)),
                    )*
                }
            }
        }

        impl IntoIterator for Code {
            type Item = Code;
            type IntoIter = std::iter::Copied<std::slice::Iter<'static, Code>>;

            fn into_iter(self) -> Self::IntoIter {
                static LANGUAGES: [Code; 50] = [
                    $(
                        Code::$code,
                    )*
                ];
                LANGUAGES.iter().copied()
            }
        }

        /// Get the Code from the language name in English
        pub fn get_language_code_from_name(english_name: &str) -> Option<Code> {
            match english_name {
                $(
                    $english => Some(Code::$code),
                )*
                _ => None,
            }
        }

        /// Get the Code from the language code as str
        pub fn get_language_code(code: &str) -> Option<Code> {
            let code = code.to_lowercase();
            if code.len() != 2 {
                return None;
            }
            match code.as_str() {
                $(
                    stringify!($code) => Some(Code::$code),
                )*
                _ => None,
            }
        }

    };
}

// Using the macro with 50 common ISO 639-1 languages, including text direction
generate_spoken_enum! {
    (en, "English", "English", LeftToRight),
    (es, "Spanish", "Español", LeftToRight),
    (fr, "French", "Français", LeftToRight),
    (de, "German", "Deutsch", LeftToRight),
    (zh, "Chinese", "中文", LeftToRight),
    (ar, "Arabic", "العربية", RightToLeft),
    (hi, "Hindi", "हिन्दी", LeftToRight),
    (pt, "Portuguese", "Português", LeftToRight),
    (ru, "Russian", "Русский", LeftToRight),
    (ja, "Japanese", "日本語", LeftToRight),
    (ko, "Korean", "한국어", LeftToRight),
    (it, "Italian", "Italiano", LeftToRight),
    (nl, "Dutch", "Nederlands", LeftToRight),
    (sv, "Swedish", "Svenska", LeftToRight),
    (tr, "Turkish", "Türkçe", LeftToRight),
    (pl, "Polish", "Polski", LeftToRight),
    (vi, "Vietnamese", "Tiếng Việt", LeftToRight),
    (th, "Thai", "ไทย", LeftToRight),
    (id, "Indonesian", "Bahasa Indonesia", LeftToRight),
    (fa, "Persian", "فارسی", RightToLeft),
    (he, "Hebrew", "עברית", RightToLeft),
    (bn, "Bengali", "বাংলা", LeftToRight),
    (ta, "Tamil", "தமிழ்", LeftToRight),
    (te, "Telugu", "తెలుగు", LeftToRight),
    (mr, "Marathi", "मराठी", LeftToRight),
    (ur, "Urdu", "اردو", RightToLeft),
    (gu, "Gujarati", "ગુજરાતી", LeftToRight),
    (pa, "Punjabi", "ਪੰਜਾਬੀ", LeftToRight),
    (kn, "Kannada", "ಕನ್ನಡ", LeftToRight),
    (ml, "Malayalam", "മലയാളം", LeftToRight),
    (or, "Odia", "ଓଡ଼ିଆ", LeftToRight),
    (my, "Burmese", "မြန်မာ", LeftToRight),
    (uk, "Ukrainian", "Українська", LeftToRight),
    (cs, "Czech", "Čeština", LeftToRight),
    (hu, "Hungarian", "Magyar", LeftToRight),
    (fi, "Finnish", "Suomi", LeftToRight),
    (da, "Danish", "Dansk", LeftToRight),
    (no, "Norwegian", "Norsk", LeftToRight),
    (el, "Greek", "Ελληνικά", LeftToRight),
    (ro, "Romanian", "Română", LeftToRight),
    (sk, "Slovak", "Slovenčina", LeftToRight),
    (bg, "Bulgarian", "Български", LeftToRight),
    (hr, "Croatian", "Hrvatski", LeftToRight),
    (sr, "Serbian", "Српски", LeftToRight),
    (lt, "Lithuanian", "Lietuvių", LeftToRight),
    (lv, "Latvian", "Latviešu", LeftToRight),
    (et, "Estonian", "Eesti", LeftToRight),
    (sl, "Slovenian", "Slovenščina", LeftToRight),
    (ms, "Malay", "Bahasa Melayu", LeftToRight),
    (sw, "Swahili", "Kiswahili", LeftToRight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_code_from_name() {
        assert_eq!(get_language_code_from_name("English"), Some(Code::en));
        assert_eq!(get_language_code_from_name("Arabic"), Some(Code::ar));
        assert_eq!(get_language_code_from_name("Swahili"), Some(Code::sw));
        assert_eq!(get_language_code_from_name("Unknown"), None);
    }

    #[test]
    fn test_get_language_code() {
        assert_eq!(get_language_code("en"), Some(Code::en));
        assert_eq!(get_language_code("AR"), Some(Code::ar));
        assert_eq!(get_language_code("Sw"), Some(Code::sw));
        assert_eq!(get_language_code("xxx"), None);
        assert_eq!(get_language_code("e"), None);
        assert_eq!(get_language_code("eng"), None);
    }

    #[test]
    fn test_get_name_in_english() {
        assert_eq!(Code::en.get_name_in_english(), "English");
        assert_eq!(Code::zh.get_name_in_english(), "Chinese");
        assert_eq!(Code::sw.get_name_in_english(), "Swahili");
    }

    #[test]
    fn test_get_name_in_native() {
        assert_eq!(Code::en.get_name_in_native(), "English");
        assert_eq!(Code::ar.get_name_in_native(), "العربية");
        assert_eq!(Code::sw.get_name_in_native(), "Kiswahili");
    }

    #[test]
    fn test_get_text_direction() {
        assert_eq!(Code::en.get_text_direction(), Direction::LeftToRight);
        assert_eq!(Code::ar.get_text_direction(), Direction::RightToLeft);
        assert_eq!(Code::he.get_text_direction(), Direction::RightToLeft);
        assert_eq!(Code::sw.get_text_direction(), Direction::LeftToRight);
    }

    #[test]
    fn test_code_from_name() {
        assert_eq!(Code::try_from("English").unwrap(), Code::en);
        assert_eq!(Code::try_from("en").unwrap(), Code::en);
        assert_eq!(Code::try_from("En").unwrap(), Code::en);
        assert_eq!(Code::try_from("eN").unwrap(), Code::en);
        assert_eq!(Code::try_from("EN").unwrap(), Code::en);
        assert_eq!(Code::try_from("Arabic").unwrap(), Code::ar);
        assert_eq!(Code::try_from("ar").unwrap(), Code::ar);
        assert_eq!(Code::try_from("Swahili").unwrap(), Code::sw);
        assert_eq!(Code::try_from("sw").unwrap(), Code::sw);
        assert!(Code::try_from("Vogon").is_err());
        assert!(Code::try_from("vo").is_err());
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(
            Language::from(Code::en),
            Language {
                code: Code::en,
                name: "English".to_string(),
                native_name: "English".to_string(),
                direction: Direction::LeftToRight,
            }
        );
    }

    #[test]
    fn test_language_from_name() {
        assert_eq!(
            Language::try_from("English").unwrap(),
            Language {
                code: Code::en,
                name: "English".to_string(),
                native_name: "English".to_string(),
                direction: Direction::LeftToRight,
            }
        );
        assert_eq!(
            Language::try_from("en").unwrap(),
            Language {
                code: Code::en,
                name: "English".to_string(),
                native_name: "English".to_string(),
                direction: Direction::LeftToRight,
            }
        );

        assert!(Language::try_from("Vogon").is_err());
    }
}
