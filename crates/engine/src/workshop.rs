use serde::{Deserialize, Serialize};

/// Represents a workshop's metadata
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Workshop {
    pub title: String,
    pub authors: Vec<String>,
    pub copyright: String,
    pub license: String,
    pub homepage: String,
    pub difficulty: String,
    #[serde(skip)]
    pub description: String,
    #[serde(skip)]
    pub setup: String,
    #[serde(skip)]
    pub license_text: String,
}
