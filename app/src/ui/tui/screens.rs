pub mod license;
pub use license::License;
pub mod log;
pub use log::Log;
pub mod workshops;
pub use workshops::Workshops;

use crate::{ui::tui::Event as UiEvent, Error};
use crossterm::event::Event;

/// The popups
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Popups {
    Log,
    License(String),
}

/// The screens
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Screens {
    Workshops,
    // Add other screens here
}

/// The event handler trait
#[async_trait::async_trait]
pub trait EventHandler {
    /// Handle an event
    async fn handle_event(&mut self, evt: &Event) -> Result<Option<UiEvent>, Error>;
}
