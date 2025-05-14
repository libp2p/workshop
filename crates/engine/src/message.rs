use crate::Workshop;
use languages::{programming, spoken};
use std::{collections::HashMap, path::PathBuf};

/// Engine messages
#[derive(Clone, Debug)]
pub enum Message {
    /// Configuration message
    Config {
        /// data directory
        data_dir: PathBuf,
        /// present working directory
        pwd: PathBuf,
        /// preferred spoken language
        spoken_language: spoken::Code,
        /// preferred programming language
        programming_language: programming::Code,
    },
    /// Select workshop
    SelectWorkshop {
        /// supported workshops
        workshops: HashMap<String, Workshop>,
    },
    /// Set workshop
    SetWorkshop {
        /// workshop name
        name: String,
    },
    /// Change spoken language
    ChangeSpokenLanguage,
    /// Select spoken language
    SelectSpokenLanguage {
        /// supported spoken languages
        spoken_languages: Vec<spoken::Code>,
    },
    /// Set spoken language
    SetSpokenLanguage {
        /// spoken language
        code: spoken::Code,
    },
    /// Change programming language
    ChangeProgrammingLanguage,
    /// Select programming language
    SelectProgrammingLanguage {
        /// supported programming languages
        programming_languages: Vec<programming::Code>,
    },
    /// Set programming language
    SetProgrammingLanguage {
        /// programming language
        code: programming::Code,
    },
    /// An error occured
    Error {
        /// the error
        error: String,
    },
    /// Quit
    Quit,
}
