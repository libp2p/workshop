use crate::{
    ui::tui::screens,
    ui::tui::{Event as UiEvent, Screen, Screens},
    Error, LocalConfig,
};
use crossterm::event::{Event, EventStream, KeyCode};
use engine::Message;
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    collections::HashMap,
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
    from_logger: Receiver<Message>,
    /// The configuration
    config: LocalConfig,
    /// The available screens - uses wrapper types with 'static lifetime
    screens: HashMap<Screens, Box<dyn Screen>>,
    /// the current screen
    screen: Vec<Screens>,
    /// the last frame duration
    last_frame_duration: Duration,
}

impl Ui {
    /// Create a new UI
    pub fn new(
        to_engine: Sender<Message>,
        from_engine: Receiver<Message>,
        from_logger: Receiver<Message>,
        config: LocalConfig,
    ) -> Self {
        let mut ui = Self {
            to_engine,
            from_engine,
            from_logger,
            config: config.clone(),
            screens: HashMap::with_capacity(5),
            screen: vec![Screens::Workshops],
            last_frame_duration: Duration::default(),
        };

        // Create screens hashmap
        ui.screens
            .insert(Screens::Workshops, Box::new(screens::Workshops::default()));
        // TODO: pull max_log from config
        ui.screens.insert(
            Screens::Log,
            Box::new(screens::Log::new(config.max_log_lines())),
        );
        ui.screens
            .insert(Screens::License, Box::new(screens::License::default()));
        ui.screens
            .insert(Screens::Spoken, Box::new(screens::Spoken::default()));
        ui.screens.insert(
            Screens::SpokenSetDefault,
            Box::new(screens::SetDefault::new("Save as default?")),
        );
        ui.screens.insert(
            Screens::Programming,
            Box::new(screens::Programming::default()),
        );
        ui.screens.insert(
            Screens::ProgrammingSetDefault,
            Box::new(screens::SetDefault::new("Save as default?")),
        );
        ui
    }

    /// async run loop
    pub async fn run(&mut self) -> Result<(), Error> {
        // initialize the terminal
        let mut terminal = ratatui::init();

        // initialize the input event stream
        let mut reader = EventStream::new();

        // set our timeout to ~16.67 ms (60 FPS)
        let target_frame_duration = Duration::from_secs_f64(1.0 / 30.0);

        // initial timeout
        let mut timeout = Delay::new(target_frame_duration);

        // the UI event from input
        let mut input_event: Option<Event> = None;

        // the engine message received
        let mut engine_event: Option<Message> = None;

        // running
        let mut running = true;

        // send the config message to the engine
        self.to_engine
            .send(Message::Config {
                config: Box::new(self.config.clone()),
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
                    engine_event = Some(msg);
                }
                _ = &mut timeout => {}
            }

            // time the actual work
            let start = Instant::now();

            {
                // handle the engine event and process ui updates
                if let Some(msg) = engine_event.take() {
                    let to_engine = self.to_engine.clone();
                    match self.handle_message(msg, to_engine).await {
                        Ok(Some(ui_event)) => match self.handle_ui_event(ui_event).await {
                            Ok(keep_going) => running = keep_going,
                            Err(e) => {
                                error!("Error handling UI event: {e}");
                            }
                        },
                        Ok(None) => {}
                        Err(e) => {
                            error!("Error handling message: {e}");
                        }
                    }
                }

                // handle the input event and process ui updates
                if let Some(evt) = input_event.take() {
                    let to_engine = self.to_engine.clone();
                    match self.handle_event(evt, to_engine).await {
                        Ok(Some(ui_event)) => match self.handle_ui_event(ui_event).await {
                            Ok(keep_going) => running = keep_going,
                            Err(e) => {
                                error!("Error handling UI event: {e}");
                            }
                        },
                        Ok(None) => {}
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

    pub async fn handle_ui_event(&mut self, ui_event: UiEvent) -> Result<bool, Error> {
        match ui_event {
            UiEvent::Back => {
                if !self.screen.is_empty() {
                    self.to_engine.send(Message::Back).await?;
                    info!("going back to previous screen");
                    self.screen.pop();
                }
            }
            UiEvent::Quit => {
                self.to_engine.send(Message::Quit).await?;
                return Ok(false);
            }
            UiEvent::SelectWorkshop => {
                info!("showing workshop selection screen");
                if self.screen.last() != Some(&Screens::Spoken) {
                    // show the workshop selection screen
                    self.screen.push(Screens::Workshops);
                }
            }
            UiEvent::SetWorkshop(ref workshop) => {
                info!("workshop selected: {}", workshop);
            }
            UiEvent::ToggleLog => {
                if self.screen.last() != Some(&Screens::Log) {
                    info!("showing log screen");
                    self.screen.push(Screens::Log);
                } else {
                    info!("hiding log screen");
                    self.screen.pop();
                }
            }
            UiEvent::ShowLicense => {
                if self.screen.last() != Some(&Screens::License) {
                    info!("showing license popup");
                    self.screen.push(Screens::License);
                }
            }
            UiEvent::Homepage(url) => {
                if let Err(e) = webbrowser::open(&url) {
                    error!("Failed to open URL: {}", e);
                }
            }
            UiEvent::ChangeSpokenLanguage => {
                info!("Change spoken language");
                if self.screen.last() != Some(&Screens::Spoken) {
                    // show the spoken language selection screen
                    self.screen.push(Screens::Spoken);
                }
                // send the message to the engine to get the list of spoken languages back
                self.to_engine.send(Message::ChangeSpokenLanguage).await?;
            }
            UiEvent::SelectSpokenLanguage => {
                info!("Selecting spoken language");
            }
            UiEvent::SetSpokenLanguage { .. } => {
                self.screen.pop();
                // pop up the confirmation dialog
                self.screen.push(Screens::SpokenSetDefault);
            }
            UiEvent::SetSpokenLanguageDefault { spoken_language } => {
                info!("Saving spoken language as default {:?}", spoken_language);
                self.config.set_spoken_language(spoken_language)?;
                self.screen.pop();
                self.to_engine.send(Message::Back).await?;
            }
            UiEvent::ChangeProgrammingLanguage => {
                info!("Change programming language");
                if self.screen.last() != Some(&Screens::Programming) {
                    // show the programming language selection screen
                    self.screen.push(Screens::Programming);
                }
                self.to_engine
                    .send(Message::ChangeProgrammingLanguage)
                    .await?;
            }
            UiEvent::SelectProgrammingLanguage => {
                info!("Selecting programming language");
            }
            UiEvent::SetProgrammingLanguage { .. } => {
                self.screen.pop();
                // pop up the confirmation dialog
                self.screen.push(Screens::ProgrammingSetDefault);
            }
            UiEvent::SetProgrammingLanguageDefault {
                programming_language,
            } => {
                info!(
                    "Saving programming language as default {:?}",
                    programming_language
                );
                self.config.set_programming_language(programming_language)?;
                self.screen.pop();
                self.to_engine.send(Message::Back).await?;
            }
        }
        Ok(true)
    }
}

#[async_trait::async_trait]
impl Screen for Ui {
    async fn handle_event(
        &mut self,
        evt: Event,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                // These key bindings work on every screen
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(Some(UiEvent::Quit)),
                KeyCode::Char('`') => return Ok(Some(UiEvent::ToggleLog)),
                _ => {
                    // pass the key events to the current screen
                    if let Some(screen_type) = self.screen.last() {
                        if let Some(screen_state) = self.screens.get_mut(screen_type) {
                            return screen_state.handle_event(evt, to_engine).await;
                        } else {
                            error!("Unknown screen: {:?}", screen_type);
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    async fn handle_message(
        &mut self,
        msg: Message,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        // the log screen is a special case, it handles log messages always
        if let Message::Log { .. } = msg {
            // log the message
            if let Some(screen) = self.screens.get_mut(&Screens::Log) {
                return screen.handle_message(msg, to_engine).await;
            }
        }

        /*
        if self.screen.is_empty() {
            // the engine might need to ask the user for their spoken and/or programming languages
            // before we have any other screen set up. so if screen is empty and we get either of
            // these messages we need to show the correct screen before processing the message
            // below so that the screen will get the message.
            match msg {
                Message::SelectWorkshop { .. } => {
                    // show the workshop selection screen before the message is processed
                    self.screen.push(Screens::Workshops);
                }
                _ => {}
            }
        }
        */

        // pass the message to the current screen
        if let Some(screen_type) = self.screen.last() {
            if let Some(screen) = self.screens.get_mut(screen_type) {
                return screen.handle_message(msg, to_engine).await;
            } else {
                error!("Unknown screen: {:?}", screen_type);
            }
        }
        Ok(None)
    }

    fn render_screen(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        last_frame_duration: Duration,
    ) -> Result<(), Error> {
        // render the current screen
        if let Some(screen_type) = self.screen.last() {
            if let Some(screen) = self.screens.get_mut(screen_type) {
                screen.render_screen(area, buf, last_frame_duration)?;
            } else {
                error!("Unknown screen: {:?}", screen_type);
            }
        }
        Ok(())
    }
}

impl Widget for &mut Ui {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.render_screen(area, buf, self.last_frame_duration);
    }
}
