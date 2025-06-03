use crate::Error;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_subscriber::{
    filter::EnvFilter, layer::Context, prelude::*, registry::LookupSpan, Layer,
};

// Custom tracing layer to send log events over mpsc
struct MpscLayer {
    sender: Sender<String>,
    file: Option<Mutex<File>>,
}

// Implement a visitor to extract fields from the event
struct FieldVisitor {
    message: Option<String>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }
}

impl<S> Layer<S> for MpscLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor { message: None };
        event.record(&mut visitor);

        // get the log message and format it
        let level = *event.metadata().level();
        let message = visitor.message.unwrap_or_default();
        let msg = format!("[{}]: {}", level, message);

        // if a file is provided, write the log message to it
        if let Some(mutex) = &self.file {
            if let Ok(mut file) = mutex.lock() {
                writeln!(file, "{msg}").unwrap();
                let _ = file.flush();
            }
        }

        // send the log message over the mpsc channel
        let _ = self.sender.try_send(msg);
    }
}

/// Async tracing logger wrapper that filters and feeds log messages over an mpsc channel for
/// integration into the TUI gui.
pub struct Log;

impl Log {
    /// Starts the logger and returns the task handle and receiver for the log messages.
    pub fn init<T: AsRef<Path>>(log: Option<T>) -> Result<Receiver<String>, Error> {
        let (sender, receiver) = mpsc::channel(16);
        let file = if let Some(path) = log {
            Some(Mutex::new(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path.as_ref())?,
            ))
        } else {
            None
        };

        let filter = EnvFilter::from_default_env();
        let layer = MpscLayer { sender, file }.with_filter(filter);

        tracing_subscriber::registry().with(layer).init();

        Ok(receiver)
    }
}
