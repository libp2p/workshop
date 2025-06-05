pub mod error;
pub use error::Error;
pub mod programming;
pub mod spoken;

pub fn programming_name(p: Option<programming::Code>) -> String {
    match p {
        Some(code) => code.get_name().to_string(),
        None => "Any".to_string(),
    }
}

pub fn spoken_name(s: Option<spoken::Code>) -> String {
    match s {
        Some(code) => code.get_name_in_english().to_string(),
        None => "Any".to_string(),
    }
}
