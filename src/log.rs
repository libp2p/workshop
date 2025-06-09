use crate::Error;
use std::{
    cell::RefCell,
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{
    field::{Field, Visit},
    Event, Id, Subscriber,
};
use tracing_subscriber::{
    filter::EnvFilter, layer::Context, prelude::*, registry::LookupSpan, Layer,
};

thread_local! {
    static INDENT_LEVEL: RefCell<usize> = const { RefCell::new(0) };
}

// Custom tracing layer to send log events over mpsc
struct MpscLayer {
    sender: Sender<String>,
    file: Mutex<Option<File>>,
}

// Implement a visitor to extract fields from the event
struct FieldVisitor {
    message: Option<String>,
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
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
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        // Increase the indent level when entering a span
        INDENT_LEVEL.with(|level| {
            *level.borrow_mut() += 1;
        });

        // Log the span enter event
        if let Some(span) = ctx.span(id) {
            let indent = INDENT_LEVEL.with(|l| "  ".repeat(*l.borrow()));
            let name = span.name();
            let msg = format!("{indent}{name}");
            // if a file is provided, write the log message to it
            if let Ok(mut guard) = self.file.lock() {
                if let Some(file) = guard.as_mut() {
                    writeln!(file, "{msg}").unwrap();
                    let _ = file.flush();
                }
            }
            let _ = self.sender.try_send(msg);
        }
    }

    fn on_exit(&self, _id: &Id, _ctx: Context<S>) {
        // Decrease the indent level when exiting a span
        INDENT_LEVEL.with(|level| {
            let mut level = level.borrow_mut();
            *level = level.saturating_sub(1);
        });
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor { message: None };
        event.record(&mut visitor);

        // get the log message and format it
        let indent = INDENT_LEVEL.with(|l| "  ".repeat(*l.borrow()));
        let level = *event.metadata().level();
        let message = visitor.message.unwrap_or_default();
        let emoji = match level {
            tracing::Level::ERROR => "! ",
            tracing::Level::WARN => "^ ",
            tracing::Level::INFO => "i ",
            tracing::Level::DEBUG => "",
            tracing::Level::TRACE => "",
        };
        let msg = format!("{emoji}{indent}{message}");

        // if a file is provided, write the log message to it
        if let Ok(mut guard) = self.file.lock() {
            if let Some(file) = guard.as_mut() {
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
            Mutex::new(Some(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path.as_ref())?,
            ))
        } else {
            Mutex::new(None)
        };

        let filter = EnvFilter::from_default_env();
        let layer = MpscLayer { sender, file }.with_filter(filter);

        tracing_subscriber::registry().with(layer).init();

        Ok(receiver)
    }
}
