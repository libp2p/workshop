use languages::spoken;

/// UI events
#[derive(Clone, Debug)]
pub enum Event {
    /// quit the application
    Quit,
    /// show the log popup
    ShowLog,
    /// show the license popup
    ShowLicense(String),
    /// close the currently shown popup
    Back,
    /// launch the browser with the given url
    Homepage(String),
    /// change the spoken language
    ChangeSpokenLanguage,
    /// select spoken language
    SelectSpokenLanguage(Vec<spoken::Code>),
    /// set the spoken language
    SetSpokenLanguage(spoken::Code),
    /// change the programming language
    ChangeProgrammingLanguage,
}
