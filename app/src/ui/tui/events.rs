use languages::{programming, spoken};

/// UI events
#[derive(Clone, Debug)]
pub enum Event {
    /// close the currently shown popup
    Back,
    /// quit the application
    Quit,
    /// show the workshop selection screen
    SelectWorkshop,
    /// set the workshop
    SetWorkshop(String),
    /// show the log popup
    ToggleLog,
    /// show the license popup
    ShowLicense,
    /// launch the browser with the given url
    Homepage(String),
    /// change the spoken language
    ChangeSpokenLanguage,
    /// select spoken language
    SelectSpokenLanguage,
    /// set the spoken language
    SetSpokenLanguage {
        spoken_language: Option<spoken::Code>,
    },
    /// set a value as default
    SetSpokenLanguageDefault {
        spoken_language: Option<spoken::Code>,
    },
    /// change the programming language
    ChangeProgrammingLanguage,
    /// select programming language
    SelectProgrammingLanguage,
    /// set the programming language
    SetProgrammingLanguage {
        programming_language: Option<programming::Code>,
    },
    /// set a value as default
    SetProgrammingLanguageDefault {
        programming_language: Option<programming::Code>,
    },
}
