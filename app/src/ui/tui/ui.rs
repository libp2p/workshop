use crate::{
    ui::tui::screens,
    ui::tui::{Event as UiEvent, EventHandler, Popups, Screens},
    Config, Error,
};
use crossterm::event::{Event, EventStream};
use engine::Message;
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tracing::{error, info};

/// Tui implementation of the UI
pub struct Ui {
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
    /// show a popup
    show_popup: Option<Popups>,
    /// The log popup
    log_screen: screens::Log,
    /// the license popup
    license_screen: screens::License,
    /// The current screen
    current_screen: Screens,
    /// The screens
    workshops_screen: screens::Workshops,
}

impl Ui {
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
                    self.log_screen.add_message(msg);
                }

                // handle the engine event
                if let Some(msg) = engine_event.take() {
                    if let Err(e) = self.handle_message(msg).await {
                        error!("Error handling message: {e}");
                    }
                }

                // handle the input event
                if let Some(evt) = input_event.take() {
                    match self.handle_event(evt).await {
                        Ok(UiEvent::Quit) => running = false,
                        Ok(UiEvent::ShowPopup(popup)) => {
                            info!("showing popup");

                            // initialize the popup
                            match popup {
                                Popups::License(ref t) => {
                                    self.license_screen.set_license(t.clone());
                                }
                                _ => {}
                            }

                            self.show_popup = Some(popup);
                        }
                        Ok(UiEvent::ClosePopup) => {
                            info!("closing log popup");
                            self.show_popup = None;
                        }
                        Ok(UiEvent::Homepage(url)) => {
                            if let Err(e) = webbrowser::open(&url) {
                                error!("Failed to open URL: {}", e);
                            }
                        }
                        Ok(UiEvent::Noop) => {}
                        Err(e) => {
                            error!("Error handling event: {e}");
                        }
                    }
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
    pub async fn handle_message(&mut self, msg: Message) -> Result<(), Error> {
        match msg {
            Message::SelectWorkshop { workshops } => {
                self.current_screen = Screens::Workshops;
                // Handle workshop selection
                info!("Showing select workshop screen");
                self.workshops_screen.set_workshops(workshops);
            }
            /*
            Message::SelectSpokenLanguage => {
                // Handle spoken language selection
                info!("Select spoken language");
                self.to_engine
                    .send(Message::SetSpokenLanguage {
                        code: spoken::Code::en,
                    })
                    .await?;
            }
            Message::SelectProgrammingLanguage => {
                // Handle programming language selection
                info!("Select programming language");
                self.to_engine
                    .send(Message::SetProgrammingLanguage {
                        code: programming::Code::rs,
                    })
                    .await?;
            }
            */
            _ => {
                // Handle other messages
                info!("Received message: {:?}", msg);
            }
        }
        Ok(())
    }

    /// Handle events from the input
    pub async fn handle_event(&mut self, evt: Event) -> Result<UiEvent, Error> {
        match self.show_popup {
            Some(Popups::Log) => Ok((&mut self.log_screen).handle_event(evt).await?),
            Some(Popups::License(_)) => Ok((&mut self.license_screen).handle_event(evt).await?),
            None => match self.current_screen {
                Screens::Workshops => Ok(self.workshops_screen.handle_event(evt).await?),
            },
        }
    }
}

impl Widget for &mut Ui {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.current_screen {
            Screens::Workshops => {
                self.workshops_screen
                    .set_last_frame_duration(self.last_frame_duration);
                Widget::render(&mut self.workshops_screen, area, buf);
            }
        }
        match self.show_popup {
            Some(Popups::Log) => {
                Widget::render(&mut self.log_screen, area, buf);
            }
            Some(Popups::License(_)) => {
                Widget::render(&mut self.license_screen, area, buf);
            }
            None => {}
        }
    }
}
