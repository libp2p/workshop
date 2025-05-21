use crate::{Lesson, WorkshopData};
use languages::{programming, spoken};
use std::collections::HashMap;

/// the engine state
#[derive(Clone, Debug, Default)]
pub enum State {
    /// uninitialized state
    #[default]
    Nil,
    /// select the workshop
    SelectWorkshop(HashMap<String, WorkshopData>),
    /// select the lesson
    SelectLesson(Vec<Lesson>),
    /// select the spoken language
    SelectSpokenLanguage { spoken_languages: Vec<spoken::Code> },
    /// set the spoken language default
    SetSpokenLanguageDefault {
        spoken_language: Option<spoken::Code>,
    },
    /// select the programming language
    SelectProgrammingLanguage {
        programming_languages: Vec<programming::Code>,
        set_default: bool,
    },
    /// send the license text
    ShowLicense(String),
    /// Error state
    Error(String),
    /// quit
    Quit,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Nil => write!(f, "Nil"),
            State::SelectWorkshop(_) => write!(f, "SelectWorkshop"),
            State::SelectLesson(_) => write!(f, "SelectLesson"),
            State::SelectSpokenLanguage { .. } => write!(f, "SelectSpokenLanguage"),
            State::SetSpokenLanguageDefault { .. } => write!(f, "SetSpokenLanguageDefault"),
            State::SelectProgrammingLanguage { .. } => write!(f, "SelectProgrammingLanguage"),
            State::ShowLicense(_) => write!(f, "License"),
            State::Error(error) => write!(f, "Error: {}", error),
            State::Quit => write!(f, "Quit"),
        }
    }
}
