use crate::{
    ui::tui::screens,
    ui::tui::{Event as UiEvent, EventHandler, Popups, Screens},
    Config, Error,
};
use crossterm::event::{Event, EventStream, KeyCode};
use engine::Message;
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use languages::{programming, spoken};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tracing::{error, info};

/// Tui implementation of the UI
pub struct Ui<'a> {
    /// The sender to the engine
    to_engine: Sender<Message>,
    /// The receiver from the engine
    from_engine: Receiver<Message>,
    /// The receiver from the logger
    from_logger: Receiver<String>,
    /// The configuration
    config: Config,
    /// The present working directory
    pwd: PathBuf,
    /// The last frame duration
    last_frame_duration: Duration,
    /// The log messages
    log: VecDeque<String>,
    /// show a popup
    show_popup: Option<Popups>,
    /// The log popup
    log_screen: screens::Log<'a>,
    /// the license popup
    license_screen: screens::License<'a>,
    /// The current screen
    current_screen: Screens,
    /// The screens
    workshops_screen: screens::Workshops<'a>,
}

impl Ui<'_> {
    /// Create a new UI
    pub fn new(
        to_engine: Sender<Message>,
        from_engine: Receiver<Message>,
        from_logger: Receiver<String>,
        config: Config,
        pwd: &Path,
    ) -> Result<Self, Error> {
        // screens
        let workshops_screen = screens::Workshops::new(to_engine.clone());
        let log_screen = screens::Log::default();
        let license_screen = screens::License::default();

        Ok(Self {
            to_engine,
            from_engine,
            from_logger,
            config,
            pwd: pwd.to_path_buf(),
            last_frame_duration: Duration::from_millis(0),
            log: VecDeque::new(),
            show_popup: None,
            log_screen,
            license_screen,
            current_screen: Screens::Workshops,
            workshops_screen,
        })
    }

    /// async run loop
    pub async fn run(&mut self) -> Result<(), Error> {
        // initialize the terminal
        let mut terminal = ratatui::init();

        // initialize the input event stream
        let mut reader = EventStream::new();

        // set our timeout to ~16.67 ms (60 FPS)
        let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        // initial timeout
        let mut timeout = Delay::new(target_frame_duration);

        // the UI event from input
        let mut input_event: Option<Event> = None;

        // the engine message received
        let mut engine_event: Option<Message> = None;

        // the logger message received
        let mut log_msg: Option<String> = None;

        // running
        let mut running = true;

        // send the config message to the engine
        self.to_engine
            .send(Message::Config {
                data_dir: self.config.data_dir().to_path_buf(),
                pwd: self.pwd.clone(),
                spoken_language: self.config.spoken_language(),
                programming_language: self.config.programming_language(),
            })
            .await?;

        while running {
            let frame_start = Instant::now();

            let event = reader.next().fuse();

            // get the next events
            select! {
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(evt)) => {
                            input_event = Some(evt);
                        }
                        Some(Err(e)) => {
                            error!("Error reading event: {}", e);
                            running = false;
                        }
                        None => running = false,
                    }
                }
                Some(msg) = self.from_engine.recv() => {
                    engine_event = Some(msg);
                }
                Some(msg) = self.from_logger.recv() => {
                    log_msg = Some(msg);
                }
                _ = &mut timeout => {}
            }

            // time the actual work
            let start = Instant::now();

            {
                // do the actual work and time it
                //
                // add log line
                if let Some(msg) = log_msg.take() {
                    self.log.push_back(msg.clone());
                    while self.log.len() > 1000 {
                        self.log.pop_front();
                    }
                }

                // handle the engine event
                if let Some(msg) = engine_event.take() {
                    if let Err(e) = self.on_message(&msg).await {
                        error!("Error handling message: {e}");
                    }
                }

                // handle the input event
                if let Some(evt) = input_event.take() {
                    running = self.on_event(&evt).await;
                }

                // render the UI
                if let Err(e) = terminal.draw(|f| f.render_widget(&mut *self, f.area())) {
                    error!("Error drawing UI: {e}");
                }
            }

            // get the duration of the work
            let elapsed = start.elapsed();

            // adjust the timeout for the next loop to account for the time spent doing work. this
            // is to maintain a constant frame rate of 60 FPS
            let adjusted_timeout = if elapsed < target_frame_duration {
                target_frame_duration - elapsed
            } else {
                Duration::from_millis(1)
            };

            // timeout reached, do nothing
            timeout = Delay::new(adjusted_timeout);

            // set the frame time
            self.last_frame_duration = frame_start.elapsed();
        }

        // Quit the engine
        info!("Quitting...");
        self.to_engine.send(Message::Quit).await?;

        ratatui::restore();

        Ok(())
    }

    /// Handle messages from the engine
    pub async fn on_message(&mut self, msg: &Message) -> Result<(), Error> {
        match msg {
            Message::SelectWorkshop { workshops } => {
                self.current_screen = Screens::Workshops;
                // Handle workshop selection
                info!("Showing select workshop screen");
                self.workshops_screen.set_workshops(workshops);
            }
            Message::SelectSpokenLanguage { spoken_languages } => {
                // Handle spoken language selection
                info!("Select spoken language: {:?}", spoken_languages);
                self.to_engine
                    .send(Message::SetSpokenLanguage {
                        code: spoken::Code::en,
                    })
                    .await?;
            }
            Message::SelectProgrammingLanguage {
                programming_languages,
            } => {
                // Handle programming language selection
                info!("Select programming language: {:?}", programming_languages);
                self.to_engine
                    .send(Message::SetProgrammingLanguage {
                        code: programming::Code::rs,
                    })
                    .await?;
            }
            _ => {
                // Handle other messages
                info!("Received message: {:?}", msg);
            }
        }
        Ok(())
    }

    /// Handle events from the input
    pub async fn on_event(&mut self, evt: &Event) -> bool {
        // get the next ui_event if there is one
        let ui_event = {
            match (&mut *self).handle_event(evt).await {
                Ok(Some(ui_event)) => ui_event,
                Ok(None) => match self.show_popup {
                    Some(Popups::Log) => match (&mut self.log_screen).handle_event(evt).await {
                        Ok(Some(ui_event)) => ui_event,
                        Ok(None) => {
                            info!("Log popup: no event to handle");
                            return true;
                        }
                        Err(e) => {
                            error!("Log popup: error handling log event: {e}");
                            return true;
                        }
                    },
                    Some(Popups::License(_)) => {
                        match (&mut self.license_screen).handle_event(evt).await {
                            Ok(Some(ui_event)) => ui_event,
                            Ok(None) => return true,
                            Err(e) => {
                                error!("License popup: error handling license event: {e}");
                                return true;
                            }
                        }
                    }
                    None => match self.current_screen {
                        Screens::Workshops => {
                            match (&mut self.workshops_screen).handle_event(evt).await {
                                Ok(Some(ui_event)) => ui_event,
                                Ok(None) => return true,
                                Err(e) => {
                                    error!("Error handling event: {e}");
                                    return true;
                                }
                            }
                        }
                    },
                },
                Err(e) => {
                    error!("Error handling event: {e}");
                    return true;
                }
            }
        };

        match ui_event {
            UiEvent::Quit => return false,
            UiEvent::ShowLog => {
                info!("showing log popup");
                self.show_popup = Some(Popups::Log);
            }
            UiEvent::ShowLicense(ref t) => {
                info!("showing license popup");
                self.show_popup = Some(Popups::License(t.clone()));
            }
            UiEvent::Back => {
                if self.show_popup.is_some() {
                    info!("closing popup");
                    self.show_popup = None;
                }
            }
            UiEvent::Homepage(url) => {
                if let Err(e) = webbrowser::open(&url) {
                    error!("Failed to open URL: {}", e);
                }
            }
            UiEvent::SpokenLanguage => {}
            UiEvent::ProgrammingLanguage => {}
        }
        true
    }
}

#[async_trait::async_trait]
impl EventHandler for &mut Ui<'_> {
    async fn handle_event(&mut self, evt: &Event) -> Result<Option<UiEvent>, Error> {
        match evt {
            Event::Key(key) => match key.code {
                // These key bindings work on every screen
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    info!("Quit");
                    Ok(Some(UiEvent::Quit))
                }
                KeyCode::Char('b') | KeyCode::Esc => Ok(Some(UiEvent::Back)),
                KeyCode::Char('`') => Ok(Some(UiEvent::ShowLog)),
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    info!("Change spoken language");
                    self.to_engine.send(Message::ChangeSpokenLanguage).await?;
                    Ok(Some(UiEvent::SpokenLanguage))
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    info!("Change programming language");
                    self.to_engine
                        .send(Message::ChangeProgrammingLanguage)
                        .await?;
                    Ok(Some(UiEvent::ProgrammingLanguage))
                }
                _ => Ok(None), // not handled at the top level
            },
            _ => {
                Ok(None) // not handled at the top level
            }
        }
    }
}

impl Widget for &mut Ui<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.current_screen {
            Screens::Workshops => {
                StatefulWidget::render(
                    &mut self.workshops_screen,
                    area,
                    buf,
                    &mut self.last_frame_duration,
                );
            }
        }
        match self.show_popup {
            Some(Popups::Log) => {
                StatefulWidget::render(&mut self.log_screen, area, buf, &mut self.log);
            }
            Some(Popups::License(ref t)) => {
                StatefulWidget::render(&mut self.license_screen, area, buf, &mut t.to_string());
            }
            None => {}
        }
    }
}
