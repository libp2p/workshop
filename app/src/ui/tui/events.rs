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
    SpokenLanguage,
    /// change the programming language
    ProgrammingLanguage,
}
