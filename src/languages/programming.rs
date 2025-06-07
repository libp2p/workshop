use crate::{languages::Error as LanguageError, Error};
use serde::{Deserialize, Serialize};

/// A programming language with language code, file extension, and name
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Language {
    pub code: Code,
    pub extension: String,
    pub name: String,
}

impl Default for Language {
    fn default() -> Self {
        Language {
            code: Code::rs,
            extension: "rs".to_string(),
            name: "Rust".to_string(),
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
        let name = code.get_name();
        let extension = code.get_extension();
        Language {
            code,
            extension: extension.to_string(),
            name: name.to_string(),
        }
    }
}

impl TryFrom<&str> for Language {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match Code::try_from(value) {
            Ok(code) => {
                let name = code.get_name();
                let extension = code.get_extension();
                Ok(Language {
                    code,
                    extension: extension.to_string(),
                    name: name.to_string(),
                })
            }
            Err(_) => Err(LanguageError::InvalidLanguageName(value.to_string()).into()),
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
            Err(LanguageError::InvalidLanguageCode(value.to_string()).into())
        }
    }
}

impl TryFrom<String> for Code {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Code::try_from(value.as_str())
    }
}

macro_rules! generate_programming_enum {
    ($(($code:ident, $name:literal, $ext:literal)),* $(,)?) => {
        /// The list of language codes
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub enum Code {
            $(
                #[allow(non_camel_case_types)]
                $code,
            )*
        }

        impl Code {
            // Get the programming language name from the code
            pub fn get_name(&self) -> &'static str {
                match &self {
                    $(
                        Code::$code => $name,
                    )*
                }
            }

            // Get the programming language extension from the code
            pub fn get_extension(&self) -> &'static str {
                match &self {
                    $(
                        Code::$code => $ext,
                    )*
                }
            }
        }

        impl Default for Code {
            fn default() -> Self {
                Code::rs
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

        impl std::cmp::PartialOrd for Code {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl std::cmp::Ord for Code {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.to_string().cmp(&other.to_string())
            }
        }

        impl IntoIterator for Code {
            type Item = Code;
            type IntoIter = std::iter::Copied<std::slice::Iter<'static, Code>>;

            fn into_iter(self) -> Self::IntoIter {
                static LANGUAGES: [Code; 20] = [
                    $(
                        Code::$code,
                    )*
                ];
                LANGUAGES.iter().copied()
            }
        }

        /// Get the Code from the language name
        pub fn get_language_code_from_name(name: &str) -> Option<Code> {
            match name {
                $(
                    $name => Some(Code::$code),
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

// Generate the programming language enum and associated functions
generate_programming_enum! {
    (py, "Python", "py"),
    (js, "JavaScript", "js"),
    (ja, "Java", "java"),
    (cs, ".Net", "cs"),
    (cp, "C++", "cpp"),
    (go, "Go", "go"),
    (rb, "Ruby", "rb"),
    (ph, "PHP", "php"),
    (ts, "TypeScript", "ts"),
    (sw, "Swift", "swift"),
    (kt, "Kotlin", "kt"),
    (rs, "Rust", "rs"),
    (sc, "Scala", "scala"),
    (lu, "Lua", "lua"),
    (pe, "Perl", "pl"),
    (hs, "Haskell", "hs"),
    (er, "Erlang", "erl"),
    (cl, "Clojure", "clj"),
    (el, "Elixir", "ex"),
    (fs, "F#", "fs"),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_code_from_name() {
        assert_eq!(get_language_code_from_name("Python"), Some(Code::py));
        assert_eq!(get_language_code_from_name("JavaScript"), Some(Code::js));
        assert_eq!(get_language_code_from_name("Java"), Some(Code::ja));
        assert_eq!(get_language_code_from_name(".Net"), Some(Code::cs));
        assert_eq!(get_language_code_from_name("C++"), Some(Code::cp));
        assert_eq!(get_language_code_from_name("Go"), Some(Code::go));
        assert_eq!(get_language_code_from_name("Ruby"), Some(Code::rb));
        assert_eq!(get_language_code_from_name("PHP"), Some(Code::ph));
        assert_eq!(get_language_code_from_name("TypeScript"), Some(Code::ts));
        assert_eq!(get_language_code_from_name("Swift"), Some(Code::sw));
        assert_eq!(get_language_code_from_name("Kotlin"), Some(Code::kt));
        assert_eq!(get_language_code_from_name("Rust"), Some(Code::rs));
        assert_eq!(get_language_code_from_name("Scala"), Some(Code::sc));
        assert_eq!(get_language_code_from_name("Lua"), Some(Code::lu));
        assert_eq!(get_language_code_from_name("Perl"), Some(Code::pe));
        assert_eq!(get_language_code_from_name("Haskell"), Some(Code::hs));
        assert_eq!(get_language_code_from_name("Erlang"), Some(Code::er));
        assert_eq!(get_language_code_from_name("Clojure"), Some(Code::cl));
        assert_eq!(get_language_code_from_name("Elixir"), Some(Code::el));
        assert_eq!(get_language_code_from_name("F#"), Some(Code::fs));

        // Test invalid names
        assert!(get_language_code_from_name("Brainfuck").is_none());
    }

    #[test]
    fn test_get_language_code() {
        assert_eq!(get_language_code("py"), Some(Code::py));
        assert_eq!(get_language_code("js"), Some(Code::js));
        assert_eq!(get_language_code("ja"), Some(Code::ja));
        assert_eq!(get_language_code("cs"), Some(Code::cs));
        assert_eq!(get_language_code("cp"), Some(Code::cp));
        assert_eq!(get_language_code("go"), Some(Code::go));
        assert_eq!(get_language_code("rb"), Some(Code::rb));
        assert_eq!(get_language_code("ph"), Some(Code::ph));
        assert_eq!(get_language_code("ts"), Some(Code::ts));
        assert_eq!(get_language_code("sw"), Some(Code::sw));
        assert_eq!(get_language_code("kt"), Some(Code::kt));
        assert_eq!(get_language_code("rs"), Some(Code::rs));
        assert_eq!(get_language_code("sc"), Some(Code::sc));
        assert_eq!(get_language_code("lu"), Some(Code::lu));
        assert_eq!(get_language_code("pe"), Some(Code::pe));
        assert_eq!(get_language_code("hs"), Some(Code::hs));
        assert_eq!(get_language_code("er"), Some(Code::er));
        assert_eq!(get_language_code("cl"), Some(Code::cl));
        assert_eq!(get_language_code("el"), Some(Code::el));
        assert_eq!(get_language_code("fs"), Some(Code::fs));

        // Test invalid codes
        assert!(get_language_code("bf").is_none());
    }

    #[test]
    fn test_get_name() {
        assert_eq!(Code::py.get_name(), "Python");
        assert_eq!(Code::js.get_name(), "JavaScript");
        assert_eq!(Code::ja.get_name(), "Java");
        assert_eq!(Code::cs.get_name(), ".Net");
        assert_eq!(Code::cp.get_name(), "C++");
        assert_eq!(Code::go.get_name(), "Go");
        assert_eq!(Code::rb.get_name(), "Ruby");
        assert_eq!(Code::ph.get_name(), "PHP");
        assert_eq!(Code::ts.get_name(), "TypeScript");
        assert_eq!(Code::sw.get_name(), "Swift");
        assert_eq!(Code::kt.get_name(), "Kotlin");
        assert_eq!(Code::rs.get_name(), "Rust");
        assert_eq!(Code::sc.get_name(), "Scala");
        assert_eq!(Code::lu.get_name(), "Lua");
        assert_eq!(Code::pe.get_name(), "Perl");
        assert_eq!(Code::hs.get_name(), "Haskell");
        assert_eq!(Code::er.get_name(), "Erlang");
        assert_eq!(Code::cl.get_name(), "Clojure");
        assert_eq!(Code::el.get_name(), "Elixir");
        assert_eq!(Code::fs.get_name(), "F#");

        // Test invalid codes
        assert!(get_language_code("bf").is_none());
    }

    #[test]
    fn test_code_from_name() {
        assert_eq!(Code::try_from("Python").unwrap(), Code::py);
        assert_eq!(Code::try_from("JavaScript").unwrap(), Code::js);
        assert_eq!(Code::try_from("Java").unwrap(), Code::ja);
        assert_eq!(Code::try_from(".Net").unwrap(), Code::cs);
        assert_eq!(Code::try_from("C++").unwrap(), Code::cp);
        assert_eq!(Code::try_from("Go").unwrap(), Code::go);
        assert_eq!(Code::try_from("Ruby").unwrap(), Code::rb);
        assert_eq!(Code::try_from("PHP").unwrap(), Code::ph);
        assert_eq!(Code::try_from("TypeScript").unwrap(), Code::ts);
        assert_eq!(Code::try_from("Swift").unwrap(), Code::sw);
        assert_eq!(Code::try_from("Kotlin").unwrap(), Code::kt);
        assert_eq!(Code::try_from("Rust").unwrap(), Code::rs);
        assert_eq!(Code::try_from("Scala").unwrap(), Code::sc);
        assert_eq!(Code::try_from("Lua").unwrap(), Code::lu);
        assert_eq!(Code::try_from("Perl").unwrap(), Code::pe);
        assert_eq!(Code::try_from("Haskell").unwrap(), Code::hs);
        assert_eq!(Code::try_from("Erlang").unwrap(), Code::er);
        assert_eq!(Code::try_from("Clojure").unwrap(), Code::cl);
        assert_eq!(Code::try_from("Elixir").unwrap(), Code::el);
        assert_eq!(Code::try_from("F#").unwrap(), Code::fs);

        // Test invalid names
        assert!(Language::try_from("Brainfuck").is_err());
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(
            Language::from(Code::py),
            Language {
                code: Code::py,
                extension: "py".to_string(),
                name: "Python".to_string(),
            }
        );
        assert_eq!(
            Language::from(Code::js),
            Language {
                code: Code::js,
                extension: "js".to_string(),
                name: "JavaScript".to_string(),
            }
        );
        assert_eq!(
            Language::from(Code::ja),
            Language {
                code: Code::ja,
                extension: "java".to_string(),
                name: "Java".to_string(),
            }
        );
    }

    #[test]
    fn test_language_from_name() {
        assert_eq!(
            Language::try_from("Python").unwrap(),
            Language {
                code: Code::py,
                extension: "py".to_string(),
                name: "Python".to_string(),
            }
        );
        assert_eq!(
            Language::try_from("JavaScript").unwrap(),
            Language {
                code: Code::js,
                extension: "js".to_string(),
                name: "JavaScript".to_string(),
            }
        );
        assert_eq!(
            Language::try_from("Java").unwrap(),
            Language {
                code: Code::ja,
                extension: "java".to_string(),
                name: "Java".to_string(),
            }
        );
    }
}
