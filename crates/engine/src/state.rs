use crate::{LessonData, WorkshopData};
use languages::{programming, spoken};
use std::collections::HashMap;

// The state machine for the engine
//
//            ┌─────┐
//            │ Nil │
//            └──┬──┘
//               │
//            <Config>
//               │
//               │
//               │
//               │                             ┌──────────────────────────────────────────────────────┐
// ╟──<quit>──┐  │  ┌───────────<back>─────────┤ SetProgrammingLanguageDefault (programming_language) │
//            │  │  │                          └──────────────────────────────────────────────────────┘
//            │  │  │                                                ▲
//            │  │  │                                                │
//            │  │  │                                     <SetProgrammingLanguage>
//            │  │  │                                                │
//            │  │  │                            ┌───────────────────┴───────────────────────────────┐
//            │  │  │                            │ SelectProgrammingLanguage (programming_languages) │
//            │  ▼  ▼                            └───────────────────────────────────────────────────┘
// ┌──────────┴──────────────────┐                                   ▲
// │                             ├────<ChangeProgrammingLanguage>────┘
// │                             │
// │                             │                      ┌────────────────────────────┐
// │                             ├─────<GetLicense>────>│                            │
// │ SelectWorkshop (workshops)  │                      │ ShowLicense (license_text) │
// │                             │<───────<Back>────────┤                            │
// │                             │                      └────────────────────────────┘
// │                             │
// │                             ├────<ChangeSpokenLanguage>─────────┐
// └─────────────┬───────────────┘                                   ▼
//            ▲  │  ▲                            ┌─────────────────────────────────────────┐
//            │  │  │                            │ SelectSpokenLanguage (spoken_languages) │
//         <back>│  │                            └───────────────────┬─────────────────────┘
//            │  │  │                                                │
//            │  │  │                                       <SetSpokenLanguage>
//            │  │  │                                                │
//            │  │  │                                                ▼
//            │  │  │                           ┌────────────────────────────────────────────┐
//            │  │  └───────────<back>──────────┤ SetSpokenLanguageDefault (spoken_language) │
//            │  │                              └────────────────────────────────────────────┘
//         <SetWorkshop>
//            │  │
//            │  ▼
//   ┌────────┴───────────────┐
//   │ SelectLesson (lessons) │
//   └───────────┬────────────┘
//            ▲  │  ▲                                          ┌────────────────┐
//            │  │  └──────────────────<back>──────────────────┤ LessonComplete │
//         <back>│                                             └────────────────┘
//            │  │                                                     ▲
//            │  │                                                     │
//            │  │                                                 [Complete]
//            │  ▼                                                     │
//  ┌─────────┴────────────────┐                       ┌───────────────┴────────────────┐
//  │ ShowLesson (lesson_text) ├─────<CheckLesson>────>│ CheckLesson (task, log_handle) │
//  └──────────────────────────┘                       └───────────────┬────────────────┘
//               ▲                                                     │
//               │                                                 [Failure]
//               │                                                     │
//               │                                                     ▼
//               │                                            ┌──────────────────┐
//               └─────────────────────<back>─────────────────┤ LessonIncomplete │
//                                                            └──────────────────┘

/// the engine state
#[derive(Clone, Debug, Default)]
pub enum State {
    /// uninitialized state
    #[default]
    Nil,
    /// select the workshop
    SelectWorkshop {
        workshops_data: HashMap<String, WorkshopData>,
    },
    /// send the license text
    ShowLicense { license_text: String },
    /// select the spoken language
    SelectSpokenLanguage { spoken_languages: Vec<spoken::Code> },
    /// set the spoken language default
    SetSpokenLanguageDefault {
        spoken_language: Option<spoken::Code>,
    },
    /// select the programming language
    SelectProgrammingLanguage {
        programming_languages: Vec<programming::Code>,
    },
    /// set the programming language default
    SetProgrammingLanguageDefault {
        programming_language: Option<programming::Code>,
    },
    /// select the lesson
    SelectLesson {
        lessons_data: HashMap<String, LessonData>,
    },
    /// check the lesson
    CheckLesson,
    /// lesson is complete
    LessonComplete,
    /// lesson is incomplete
    LessonIncomplete,
    /// Error state
    Error(String),
    /// quit
    Quit,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Nil => write!(f, "Nil"),
            State::SelectWorkshop { .. } => write!(f, "SelectWorkshop"),
            State::ShowLicense { .. } => write!(f, "License"),
            State::SelectSpokenLanguage { .. } => write!(f, "SelectSpokenLanguage"),
            State::SetSpokenLanguageDefault { .. } => write!(f, "SetSpokenLanguageDefault"),
            State::SelectProgrammingLanguage { .. } => write!(f, "SelectProgrammingLanguage"),
            State::SetProgrammingLanguageDefault { .. } => {
                write!(f, "SetProgrammingLanguageDefault")
            }
            State::SelectLesson { .. } => write!(f, "SelectLesson"),
            State::CheckLesson => write!(f, "CheckLesson"),
            State::LessonComplete => write!(f, "LessonComplete"),
            State::LessonIncomplete => write!(f, "LessonIncomplete"),
            State::Error(error) => write!(f, "Error: {}", error),
            State::Quit => write!(f, "Quit"),
        }
    }
}
