use crate::{
    languages::{programming, spoken},
    ui::tui::screens::Screens,
};

/// UI events
#[derive(Clone, Debug)]
pub enum Event {
    /// log event
    Log(String),
    /// show the log popup
    ToggleLog,
    /// quit the application
    Quit,
    /// show the specified screen
    Show(Screens),
    /// load the workshops
    LoadWorkshops,
    /// set the workshop
    SetWorkshop(Option<String>),
    /// load the license for a workshop
    ShowLicense(String),
    /// change the spoken language
    ChangeSpokenLanguage,
    /// set the default spoken language
    SetSpokenLanguage(Option<spoken::Code>, Option<bool>),
    /// change the programming language
    ChangeProgrammingLanguage,
    /// set the default programming language
    SetProgrammingLanguage(Option<programming::Code>, Option<bool>),
    /// initialize the "set default" dialog
    SetDefault(String, Option<Box<Event>>, Option<Box<Event>>),
    /// load lessons
    LoadLessons,
    /// set the lesson
    SetLesson(Option<String>),
    /// load the selected lesson
    LoadLesson,
    /// check the solutionto the lesson
    CheckSolution,
    /// the solution is a success
    SolutionSuccess,
    /// the solution is a failure
    SolutionFailure,
    /// command started (show log screen)
    CommandStarted,
    /// command completed
    CommandCompleted { success: bool },
    /// command output (stdout)
    CommandOutput(String),
}
