use crate::{
    command::CommandResult,
    languages::{programming, spoken},
    ui::tui::{screens::Screens, widgets::StatusMode},
};
use std::collections::HashMap;
use tokio::time::Duration;

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
    /// toggle the log
    ToggleLog,
    /// show the log
    ShowLog(Option<Evt>),
    /// hide the log
    HideLog(Option<Evt>),
    /// delay
    Delay(Duration, Option<Evt>),
    /// quit the application
    Quit,
    /// show the specified screen
    Show(Screens),
    /// load the workshops
    LoadWorkshops,
    /// set the workshop
    SetWorkshop(
        Option<String>,
        HashMap<spoken::Code, Vec<programming::Code>>,
    ),
    /// load the license for a workshop
    ShowLicense(String),
    /// change the spoken language
    ChangeSpokenLanguage(
        HashMap<spoken::Code, Vec<programming::Code>>,
        Option<spoken::Code>,
        bool,        // show the "Any" option?
        Option<Evt>, // the event to send when language is selected
    ),
    /// set the default spoken language
    SetSpokenLanguage(Option<spoken::Code>, Option<bool>, Option<Evt>),
    /// change the programming language
    ChangeProgrammingLanguage(
        HashMap<spoken::Code, Vec<programming::Code>>,
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
    /// check dependendcies for the specified workshop
    CheckDeps(String, Option<Evt>, Option<Evt>),
    /// check the solutionto the lesson
    CheckSolution(Option<Evt>, Option<Evt>),
    /// the solution is correct
    SolutionComplete,
    /// the solution is incorrect
    SolutionIncomplete,
    /// command started (show log screen)
    CommandStarted(StatusMode, String),
    /// command output
    CommandOutput(String, Option<u8>),
    /// command completed
    CommandCompleted(CommandResult, Option<Evt>, Option<Evt>),
    /// install a workshop from a URL
    InstallWorkshop(String, Option<Evt>),
}
