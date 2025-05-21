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
        code: spoken::Code,
        set_default: bool,
    },
    /// change the programming language
    ChangeProgrammingLanguage,
    /// select programming language
    SelectProgrammingLanguage,
    /// set the programming language
    SetProgrammingLanguage {
        code: programming::Code,
        set_default: bool,
    },
}
