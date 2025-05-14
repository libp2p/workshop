/// the workshop engine
pub mod engine;
pub use engine::Engine;

/// the errors this crate can produce
pub mod error;
pub use error::Error;

/// the filesystem utility functions
pub mod fs;
pub use fs::Fs;

/// the lesson model
pub mod lesson;
pub use lesson::{Lesson, Status as LessonStatus};

/// the engine log
pub mod log;
pub use log::Log;

/// the messages sent to/from the engine
pub mod message;
pub use message::Message;

/// the workshop model
pub mod workshop;
pub use workshop::Workshop;

/// the engine state
pub(crate) mod state;
pub(crate) use state::State;
