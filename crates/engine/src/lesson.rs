use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the status of a Lesson
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum Status {
    /// The lesson is not started
    #[default]
    NotStarted,
    /// The lesson is in progress
    InProgress,
    /// The lesson is completed
    Completed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::NotStarted => write!(f, "Not Started"),
            Status::InProgress => write!(f, "In Progress"),
            Status::Completed => write!(f, "Completed"),
        }
    }
}

/// Represents a workshop's metadata
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Lesson {
    pub title: String,
    pub description: String,
    pub status: Status,
}
