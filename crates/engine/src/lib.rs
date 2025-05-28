/// the config trait
pub mod config;
pub use config::Config;

/// the workshop engine
pub mod engine;
pub use engine::Engine;

/// the errors this crate can produce
pub mod error;
pub use error::Error;

/// the filesystem utility functions
pub(crate) mod fs;
pub(crate) use fs::Fs;

/// the TryLoad trait and LazyLoader
pub(crate) mod lazy_loader;
pub(crate) use lazy_loader::{LazyLoader, TryLoad};

/// the lesson model
pub mod lesson;
pub(crate) use lesson::LessonData;
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
pub(crate) use workshop::WorkshopData;

/// the engine state
pub(crate) mod state;
pub(crate) use state::State;
