use crate::{
    languages::{programming, spoken},
    ui::tui::screens::Screens,
};

/// a type alias defining a targeted event
pub type Evt = (Option<Screens>, Box<Event>);

#[macro_export]
macro_rules! evt {
    (None, $event:expr $(,)?) => {
        (None, Box::new($event))
    };
    ($screen:expr, $event:expr $(,)?) => {
        (Some($screen), Box::new($event))
    };
}

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
    ChangeSpokenLanguage(
        Option<spoken::Code>,
        bool,        // show the "Any" option?
        Option<Evt>, // the event to send when language is selected
    ),
    /// set the default spoken language
    SetSpokenLanguage(Option<spoken::Code>, Option<bool>, Option<Evt>),
    /// change the programming language
    ChangeProgrammingLanguage(
        Option<programming::Code>,
        bool,        // show the "Any" option?
        Option<Evt>, // the event to send language is selected
    ),
    /// set the default programming language
    SetProgrammingLanguage(
        Option<programming::Code>,
        Option<bool>,
        Option<Evt>, // the event to send after setting language
    ),
    /// initialize the "set default" dialog
    SetDefault(
        String,
        Option<Evt>, // the event to send when they select "yes"
        Option<Evt>, // the event to send when they select "no"
    ),
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
