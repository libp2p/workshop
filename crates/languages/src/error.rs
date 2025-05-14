/// Errors generated from this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error when the language code is not found
    #[error("Invalid language code: {0}")]
    InvalidLanguageCode(String),

    /// Error when the language name is not found
    #[error("Invalid language name: {0}")]
    InvalidLanguageName(String),
}
