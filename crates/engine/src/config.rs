use languages::{programming, spoken};
use std::path::Path;

/// This is the trait for all configuration impls
pub trait Config: Send + Sync {
    /// Get the path to the application data directory
    fn data_dir(&self) -> &Path;
    /// Get the present working directory
    fn pwd(&self) -> &Path;
    /// Get the preferred spoken language
    fn spoken_language(&self) -> Option<spoken::Code>;
    /// Get the preferred programming language
    fn programming_language(&self) -> Option<programming::Code>;
}
