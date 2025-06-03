use crate::{
    models::{LessonData, WorkshopData},
    ui::tui::screens::Screens,
};
use languages::{programming, spoken};
use std::collections::HashMap;

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
    /// set the workshops
    SetWorkshops(HashMap<String, WorkshopData>),
    /// set the workshop
    SelectWorkshop(String),
    /// load the license for a workshop
    ShowLicense(String),
    /// set the license for a workshop
    SetLicense(String),
    /// change the spoken language
    ChangeSpokenLanguage,
    /// set the spoken languages
    SetSpokenLanguages(Vec<spoken::Code>),
    /// new spoken language selected
    SpokenLanguage(Option<spoken::Code>),
    /// set the default spoken language
    SetDefaultSpokenLanguage(Option<spoken::Code>),
    /// change the programming language
    ChangeProgrammingLanguage,
    /// set the programming languages
    SetProgrammingLanguages(Vec<programming::Code>),
    /// new programming language selected
    ProgrammingLanguage(Option<programming::Code>),
    /// set the default programming language
    SetDefaultProgrammingLanguage(Option<programming::Code>),
    /// set the event in the confirmation dialog
    SetEvent(Box<Event>),
    /// launch the browser with the given url
    Homepage(String),
    /// load lessons
    LoadLessons(String),
    /// Set the selected lesson
    SetLessons(HashMap<String, LessonData>),
    /// set the lesson
    SelectLesson(String),
}
