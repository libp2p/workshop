use crate::{Config, Lesson, Workshop};
use languages::{programming, spoken};
use std::collections::HashMap;

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
    /// Set workshop  UI --> Engine
    SetWorkshop {
        /// workshop key
        name: String,
    },
    /// Select lesson  UI <-- Engine
    SelectLesson {
        /// lessons in the selected workshop
        lessons: HashMap<String, Lesson>,
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
