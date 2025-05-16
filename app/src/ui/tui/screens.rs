pub mod license;
pub use license::License;
pub mod log;
pub use log::Log;
pub mod spoken;
pub use spoken::Spoken;
pub mod workshops;
pub use workshops::Workshops;

use crate::{ui::tui::Event as UiEvent, Error};
use crossterm::event::Event;
use engine::Message;
use ratatui::widgets::StatefulWidget;

/// The popups
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Popups {
    Log,
    License(String),
    Spoken(Vec<languages::spoken::Code>),
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

/// The message handler trait
#[async_trait::async_trait]
pub trait MessageHandler {
    /// Handle a message
    async fn handle_message(&mut self, msg: &Message) -> Result<Option<UiEvent>, Error>;
}

/// The screen trait
trait Screen: StatefulWidget + EventHandler + MessageHandler + Send + Sync {}
