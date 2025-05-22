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
pub mod workshops;
pub use workshops::Workshops;

use crate::{ui::tui::Event as UiEvent, Error};
use crossterm::event::Event;
use engine::Message;
use ratatui::{buffer::Buffer, layout::Rect};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

/// The screens
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Screens {
    Workshops,
    Log,
    License,
    Spoken,
    SpokenSetDefault,
    Programming,
    ProgrammingSetDefault,
    Lessons,
}

/// The State trait
#[async_trait::async_trait]
pub trait Screen: Send + Sync {
    /// Handle an event
    async fn handle_event(
        &mut self,
        evt: Event,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error>;

    /// Handle a message
    async fn handle_message(
        &mut self,
        msg: Message,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error>;

    /// Render the screen
    fn render_screen(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        last_frame_duration: Duration,
    ) -> Result<(), Error>;
}
