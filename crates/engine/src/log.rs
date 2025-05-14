use crate::Error;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::Write,
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
    file: Option<File>,
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

        let level = *event.metadata().level();
        let message = visitor.message.unwrap_or_default();
        let msg = format!("[{}]: {}", level, message);

        let mut f = self.file.as_ref().unwrap().try_clone().unwrap();
        writeln!(f, "{msg}").unwrap();
        f.flush().unwrap();

        let _ = self.sender.try_send(msg.clone());
    }
}

/// Async tracing logger wrapper that filters and feeds log messages over an mpsc channel for
/// integration into the TUI gui.
pub struct Log;

impl Log {
    /// Starts the logger and returns the task handle and receiver for the log messages.
    pub fn init() -> Result<Receiver<String>, Error> {
        let (sender, receiver) = mpsc::channel(16);
        let file = Some(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open("log.txt")?,
        );

        let filter = EnvFilter::from_default_env();
        let layer = MpscLayer { sender, file }.with_filter(filter);

        tracing_subscriber::registry().with(layer).init();

        Ok(receiver)
    }
}
