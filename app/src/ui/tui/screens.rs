pub mod lessons;
pub use lessons::Lessons;
pub mod license;
pub use license::License;
pub mod log;
pub use log::Log;
pub mod programming;
pub use programming::Programming;
pub mod set_default;
pub use set_default::SetDefault;
pub mod spoken;
pub use spoken::Spoken;
pub mod welcome;
pub use welcome::Welcome;
pub mod workshops;
pub use workshops::Workshops;

use crate::{ui::tui, Error};
use crossterm::event;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt;
use tokio::sync::mpsc::Sender;

/// The screens
#[repr(u8)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Screens {
    Welcome,
    Workshops,
    Log,
    License,
    Spoken,
    SpokenSetDefault,
    Programming,
    ProgrammingSetDefault,
    Lessons,
}

impl fmt::Display for Screens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Screens::Welcome => write!(f, "Welcome"),
            Screens::Workshops => write!(f, "Workshops"),
            Screens::Log => write!(f, "Log"),
            Screens::License => write!(f, "License"),
            Screens::Spoken => write!(f, "Spoken"),
            Screens::SpokenSetDefault => write!(f, "Spoken Set Default"),
            Screens::Programming => write!(f, "Programming"),
            Screens::ProgrammingSetDefault => write!(f, "Programming Set Default"),
            Screens::Lessons => write!(f, "Lessons"),
        }
    }
}

impl From<Screens> for u8 {
    fn from(screen: Screens) -> Self {
        screen as u8
    }
}

impl From<u8> for Screens {
    fn from(value: u8) -> Self {
        match value {
            0 => Screens::Welcome,
            1 => Screens::Workshops,
            2 => Screens::Log,
            3 => Screens::License,
            4 => Screens::Spoken,
            5 => Screens::SpokenSetDefault,
            6 => Screens::Programming,
            7 => Screens::ProgrammingSetDefault,
            8 => Screens::Lessons,
            _ => panic!("Invalid screen value"),
        }
    }
}

/// The possible events to handle
#[derive(Clone, Debug)]
pub enum Event {
    Input(event::Event),
    Ui(tui::Event),
}

impl From<event::Event> for Event {
    fn from(event: event::Event) -> Self {
        Event::Input(event)
    }
}

impl From<tui::Event> for Event {
    fn from(event: tui::Event) -> Self {
        Event::Ui(event)
    }
}

/// The State trait
#[async_trait::async_trait]
pub trait Screen: Send + Sync {
    /// Handle an event
    async fn handle_event(&mut self, event: Event, to_ui: Sender<Event>) -> Result<(), Error>;

    /// Render the screen
    fn render_screen(&mut self, area: Rect, buf: &mut Buffer) -> Result<(), Error>;
}
