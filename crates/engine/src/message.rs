use crate::{Config, Lesson, Workshop};
use languages::{programming, spoken};
use std::{collections::HashMap, fmt};

/// Engine messages
pub enum Message {
    /// Log message  UI <-- Engine
    Log { msg: String },
    /// Configuration message  UI --> Engine
    Config {
        config: Box<dyn Config + Send + 'static>,
    },
    /// Select workshop  UI <-- Engine
    SelectWorkshop {
        /// supported workshops
        workshops: HashMap<String, Workshop>,
        /// the current selected spoken language
        spoken_language: Option<spoken::Code>,
        /// the current selected programming language
        programming_language: Option<programming::Code>,
    },
    /// Get workshop description
    GetWorkshopDescription {
        /// workshop key
        name: String,
    },
    /// Show the workshop description
    ShowWorkshopDescription {
        /// workshop key
        name: String,
        /// workshop description text
        text: String,
    },
    /// Get workshop setup instructions
    GetWorkshopSetupInstructions {
        /// workshop key
        name: String,
    },
    /// Show the workshop setup instructions
    ShowWorkshopSetupInstructions {
        /// workshop key
        name: String,
        /// workshop description text
        text: String,
    },
    /// Get workshop spoken languages
    GetWorkshopSpokenLanguages {
        /// workshop key
        name: String,
    },
    /// Show the workshop spoken languages
    ShowWorkshopSpokenLanguages {
        /// workshop key
        name: String,
        /// workshop spoken languages
        spoken_languages: Vec<spoken::Code>,
    },
    /// Get workshop programming languages
    GetWorkshopProgrammingLanguages {
        /// workshop key
        name: String,
    },
    /// Show the workshop programming languages
    ShowWorkshopProgrammingLanguages {
        /// workshop key
        name: String,
        /// workshop programming languages
        programming_languages: Vec<programming::Code>,
    },
    /// Set workshop  UI --> Engine
    SetWorkshop {
        /// workshop key
        name: String,
    },
    /// Select lesson  UI <-- Engine
    SelectLesson {
        /// lessons in the selected workshop
        lessons: HashMap<String, Lesson>,
        /// lesson texts
        lesson_texts: HashMap<String, String>,
        /// the current selected spoken language
        spoken_language: Option<spoken::Code>,
        /// the current selected programming language
        programming_language: Option<programming::Code>,
    },
    /// Set the lesson  UI --> Engine
    SetLesson {
        /// lesson key
        name: String,
    },
    /// Get the license text  UI --> Engine
    GetLicense {
        /// workshop key
        name: String,
    },
    /// License  UI <-- Engine
    ShowLicense {
        /// license text
        text: String,
    },
    /// Change spoken language  UI --> Engine
    ChangeSpokenLanguage,
    /// Select spoken language  UI <-- Engine
    SelectSpokenLanguage {
        /// supported spoken languages
        spoken_languages: Vec<spoken::Code>,
        /// the current selected spoken language
        spoken_language: Option<spoken::Code>,
    },
    /// Set spoken language  UI --> Engine
    SetSpokenLanguage {
        /// spoken language
        spoken_language: Option<spoken::Code>,
    },
    /// Set spoken language default  UI <-- Engine
    SetSpokenLanguageDefault {
        /// set the selection as default
        spoken_language: Option<spoken::Code>,
    },
    /// Change programming language  UI --> Engine
    ChangeProgrammingLanguage,
    /// Select programming language  UI <-- Engine
    SelectProgrammingLanguage {
        /// supported programming languages
        programming_languages: Vec<programming::Code>,
        /// the current selected programming language
        programming_language: Option<programming::Code>,
    },
    /// Set programming language  UI --> Engine
    SetProgrammingLanguage {
        /// programming language
        programming_language: Option<programming::Code>,
    },
    /// Set programming language default  UI <-- Engine
    SetProgrammingLanguageDefault {
        /// set the selection as default
        programming_language: Option<programming::Code>,
    },
    /// An error occured  UI <-- Engine
    Error {
        /// the error
        error: String,
    },
    /// Go back a state
    Back,
    /// Quit   UI --> Engine
    Quit,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Log { msg } => write!(f, "Log: {}", msg),
            Message::Config { .. } => write!(f, "Config message"),
            Message::SelectWorkshop { workshops, .. } => {
                write!(f, "SelectWorkshop with {} workshops", workshops.len())
            }
            Message::GetWorkshopDescription { name } => {
                write!(f, "GetWorkshopDescription for workshop: {}", name)
            }
            Message::ShowWorkshopDescription { name, text } => {
                write!(f, "ShowWorkshopDescription: {} {}", name, text)
            }
            Message::GetWorkshopSetupInstructions { name } => {
                write!(f, "GetWorkshopSetupInstructions for workshop: {}", name)
            }
            Message::ShowWorkshopSetupInstructions { name, text } => {
                write!(f, "ShowWorkshopSetupInstructions: {} {}", name, text)
            }
            Message::GetWorkshopSpokenLanguages { name } => {
                write!(f, "GetWorkshopSpokenLanguages for workshop: {}", name)
            }
            Message::ShowWorkshopSpokenLanguages {
                name,
                spoken_languages,
            } => {
                write!(
                    f,
                    "ShowWorkshopSpokenLanguages: {} with {} languages",
                    name,
                    spoken_languages.len()
                )
            }
            Message::GetWorkshopProgrammingLanguages { name } => {
                write!(f, "GetWorkshopProgrammingLanguages for workshop: {}", name)
            }
            Message::ShowWorkshopProgrammingLanguages {
                name,
                programming_languages,
            } => {
                write!(
                    f,
                    "ShowWorkshopProgrammingLanguages: {} with {} languages",
                    name,
                    programming_languages.len()
                )
            }
            Message::SetWorkshop { name } => write!(f, "SetWorkshop: {}", name),
            Message::SelectLesson { lessons, .. } => {
                write!(f, "SelectLesson with {} lessons", lessons.len())
            }
            Message::SetLesson { name } => write!(f, "SetLesson: {}", name),
            Message::GetLicense { name } => write!(f, "GetLicense for workshop: {}", name),
            Message::ShowLicense { text } => write!(f, "ShowLicense: {}", text),
            Message::ChangeSpokenLanguage => write!(f, "ChangeSpokenLanguage"),
            Message::SelectSpokenLanguage {
                spoken_languages, ..
            } => {
                write!(
                    f,
                    "SelectSpokenLanguage with {} languages",
                    spoken_languages.len()
                )
            }
            Message::SetSpokenLanguage { spoken_language } => {
                write!(f, "SetSpokenLanguage: {:?}", spoken_language)
            }
            Message::SetSpokenLanguageDefault { spoken_language } => {
                write!(f, "SetSpokenLanguageDefault: {:?}", spoken_language)
            }
            Message::ChangeProgrammingLanguage => write!(f, "ChangeProgrammingLanguage"),
            Message::SelectProgrammingLanguage {
                programming_languages,
                ..
            } => {
                write!(
                    f,
                    "SelectProgrammingLanguage with {} languages",
                    programming_languages.len()
                )
            }
            Message::SetProgrammingLanguage {
                programming_language,
            } => {
                write!(f, "SetProgrammingLanguage: {:?}", programming_language)
            }
            Message::SetProgrammingLanguageDefault {
                programming_language,
            } => {
                write!(
                    f,
                    "SetProgrammingLanguageDefault: {:?}",
                    programming_language
                )
            }
            Message::Error { error } => write!(f, "Error: {}", error),
            Message::Back => write!(f, "Back"),
            Message::Quit => write!(f, "Quit"),
        }
    }
}
