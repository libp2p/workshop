use crate::{
    languages::{programming, spoken},
    ui::tui::{
        self,
        screens::{self, Screen, Screens},
    },
    Config, Error,
};
use crossterm::event::{self, EventStream, KeyCode};
use futures::{future::FutureExt, StreamExt};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Tui implementation of the UI
pub struct App {
    /// The receiver from the logger
    from_logger: Receiver<String>,
    /// The configuration
    config: Config,
    /// The available screens - uses wrapper types with 'static lifetime
    screens: HashMap<Screens, Box<dyn Screen>>,
    /// If the log window is shown
    log: AtomicBool,
    /// The current screen
    screen: AtomicU8,
    /// the cancelation token
    token: CancellationToken,
    /// the receiver for UI events
    receiver: Receiver<screens::Event>,
    /// the sender for UI events
    sender: Sender<screens::Event>,
    /// the selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the selected programming languages
    programming_language: Option<programming::Code>,
}

impl App {
    /// Create a new UI
    pub fn new(from_logger: Receiver<String>, config: Config) -> Self {
        info!("creating UI with config: {:?}", config);
        let (sender, receiver) = tokio::sync::mpsc::channel(1_000_000);
        Self {
            from_logger,
            config: config.clone(),
            screens: Self::create_screens(&config),
            log: AtomicBool::new(false),
            screen: AtomicU8::new(Screens::Workshops as u8),
            token: CancellationToken::new(),
            receiver,
            sender,
            spoken_language: config.spoken_language(),
            programming_language: config.programming_language(),
        }
    }

    // create the screens
    fn create_screens(config: &Config) -> HashMap<Screens, Box<dyn Screen>> {
        info!("creating screens");
        let mut screens = HashMap::<Screens, Box<dyn Screen>>::with_capacity(8);

        // Welcome Screen
        screens.insert(Screens::Welcome, Box::new(screens::Welcome::default()));

        // Workshop Selection Screen
        screens.insert(Screens::Workshops, Box::new(screens::Workshops::default()));

        // Log Screen
        screens.insert(
            Screens::Log,
            Box::new(screens::Log::new(config.max_log_lines())),
        );

        // License Screen
        screens.insert(Screens::License, Box::new(screens::License::default()));

        // Spoken Language Selection Screen
        screens.insert(Screens::Spoken, Box::new(screens::Spoken::default()));

        // Spoken Language Set Default Confirmation Screen
        screens.insert(
            Screens::SpokenSetDefault,
            Box::new(screens::SetDefault::new("Save as default?")),
        );

        // Programming Language Selection Screen
        screens.insert(
            Screens::Programming,
            Box::new(screens::Programming::default()),
        );

        // Programming Language Set Default Confirmation Screen
        screens.insert(
            Screens::ProgrammingSetDefault,
            Box::new(screens::SetDefault::new("Save as default?")),
        );

        // Lessons Screen
        screens.insert(Screens::Lessons, Box::new(screens::Lessons::default()));

        info!("screens created: {:?}", screens.keys());
        screens
    }

    /// async run loop
    pub async fn run(&mut self) -> Result<(), Error> {
        // initialize the terminal
        let mut terminal = ratatui::init();

        // initialize the input event stream
        let mut reader = EventStream::new();

        // initialize the workshops screen
        self.sender.send(tui::Event::LoadWorkshops.into()).await?;

        'run: loop {
            let input_event = reader.next().fuse();

            // get the next event
            select! {
                // receive an input event and queue it
                maybe_event = input_event => {
                    match maybe_event {
                        Some(Ok(evt)) => {
                            self.sender.send(evt.into()).await?;
                        }
                        Some(Err(e)) => {
                            error!("Error reading event: {}", e);
                            break 'run;
                        }
                        None => break 'run,
                    }
                }

                // queue up a log message
                Some(msg) = self.from_logger.recv() => {
                    self.sender.send(tui::Event::Log(msg).into()).await?;
                }

                // get the next event in the queue
                Some(evt) = self.receiver.recv() => {
                    self.handle_event(evt, self.sender.clone()).await?;
                }

                // check if we should quit
                _ = self.token.cancelled() => {
                    info!("cancelation token triggered, quitting...");
                    break 'run;
                }
            }

            // render the UI
            if let Err(e) = terminal.draw(|f| f.render_widget(&mut *self, f.area())) {
                error!("Error drawing UI: {e}");
            }
        }

        // clean up the terminal
        info!("Quitting...");
        ratatui::restore();

        Ok(())
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::Log(_) => {
                if let Some(screen) = self.screens.get_mut(&Screens::Log) {
                    screen.handle_event(event.into(), to_ui).await?;
                } else {
                    error!("Log screen not found");
                }
            }
            tui::Event::Quit => {
                info!("UI event: Quit");
                self.token.cancel();
            }
            tui::Event::ToggleLog => {
                self.log.fetch_xor(true, Ordering::SeqCst);
                if let Some(screen) = self.screens.get_mut(&Screens::Log) {
                    screen
                        .handle_event(
                            tui::Event::SpokenLanguage(self.spoken_language).into(),
                            to_ui.clone(),
                        )
                        .await?;
                } else {
                    error!("Log screen not found");
                }
            }
            tui::Event::Show(screen) => {
                self.screen.store(screen.clone() as u8, Ordering::SeqCst);
                // always update the current screen with the latest spoken and programming
                // languages
                if let Some(screen) = self.screens.get_mut(&screen) {
                    screen
                        .handle_event(
                            tui::Event::SpokenLanguage(self.spoken_language).into(),
                            to_ui.clone(),
                        )
                        .await?;
                    screen
                        .handle_event(
                            tui::Event::ProgrammingLanguage(self.programming_language).into(),
                            to_ui.clone(),
                        )
                        .await?;
                } else {
                    error!("Screen not found {:?}", screen);
                }
            }
            tui::Event::LoadWorkshops => {
                info!("UI event: Load Workshops");
                let workshop_data = crate::fs::get_workshops_data(self.config.data_dir())?
                    .into_iter()
                    .filter(|(_, workshop_data)| {
                        workshop_data.is_selected(self.spoken_language, self.programming_language)
                    })
                    .collect::<HashMap<_, _>>();

                // send the workshops to the workshops screen
                if let Some(screen) = self.screens.get_mut(&Screens::Workshops) {
                    // set the workshop data
                    screen
                        .handle_event(
                            tui::Event::SetWorkshops(workshop_data).into(),
                            to_ui.clone(),
                        )
                        .await?;

                    // show the workshops screen
                    to_ui
                        .send(tui::Event::Show(Screens::Workshops).into())
                        .await?;
                } else {
                    error!("Workshops screen not found");
                }
            }
            tui::Event::ShowLicense(text) => {
                info!("UI event: Show License for workshop");
                if let Some(screen) = self.screens.get_mut(&Screens::License) {
                    // set the license text
                    screen
                        .handle_event(tui::Event::SetLicense(text).into(), to_ui.clone())
                        .await?;

                    // show the license screen
                    to_ui
                        .send(tui::Event::Show(Screens::License).into())
                        .await?;
                } else {
                    error!("License screen not found");
                }
            }
            tui::Event::ChangeSpokenLanguage => {
                info!("UI event: Change spoken language");
                let spoken_languages =
                    crate::fs::get_workshops_spoken_languages(self.config.data_dir())?;
                // send the spoken languages to the spoken language screen
                if let Some(screen) = self.screens.get_mut(&Screens::Spoken) {
                    screen
                        .handle_event(
                            tui::Event::SetSpokenLanguages(spoken_languages).into(),
                            to_ui.clone(),
                        )
                        .await?;

                    // show the spoken language selection screen
                    to_ui.send(tui::Event::Show(Screens::Spoken).into()).await?;
                } else {
                    error!("Spoken language screen not found");
                }
            }
            tui::Event::SpokenLanguage(spoken_language) => {
                info!("UI event: Spoken language set: {:?}", spoken_language);
                self.spoken_language = spoken_language;
                // send the event to send back if they select "yes"
                if let Some(screen) = self.screens.get_mut(&Screens::SpokenSetDefault) {
                    screen
                        .handle_event(
                            tui::Event::SetEvent(Box::new(tui::Event::SetDefaultSpokenLanguage(
                                spoken_language,
                            )))
                            .into(),
                            to_ui.clone(),
                        )
                        .await?;

                    // show the set as default dialog
                    to_ui
                        .send(tui::Event::Show(Screens::SpokenSetDefault).into())
                        .await?;
                } else {
                    error!("Spoken language set default screen not found");
                }
            }
            tui::Event::SetDefaultSpokenLanguage(spoken_language) => {
                info!(
                    "UI event: Saving spoken language as default: {:?}",
                    spoken_language
                );
                self.config.set_spoken_language(spoken_language)?;
            }
            tui::Event::ChangeProgrammingLanguage => {
                info!("UI event: Change programming language");
                let programming_languages =
                    crate::fs::get_workshops_programming_languages(self.config.data_dir())?;
                // send the programming languages to the programming language screen
                if let Some(screen) = self.screens.get_mut(&Screens::Programming) {
                    screen
                        .handle_event(
                            tui::Event::SetProgrammingLanguages(programming_languages).into(),
                            to_ui.clone(),
                        )
                        .await?;

                    // show the programming language selection screen
                    to_ui
                        .send(tui::Event::Show(Screens::Programming).into())
                        .await?;
                } else {
                    error!("Programming language screen not found");
                }
            }
            tui::Event::ProgrammingLanguage(programming_language) => {
                info!(
                    "UI event: Programming language set: {:?}",
                    programming_language
                );
                self.programming_language = programming_language;
                // send the event to send back if they select "yes"
                if let Some(screen) = self.screens.get_mut(&Screens::ProgrammingSetDefault) {
                    screen
                        .handle_event(
                            tui::Event::SetEvent(Box::new(
                                tui::Event::SetDefaultProgrammingLanguage(programming_language),
                            ))
                            .into(),
                            to_ui.clone(),
                        )
                        .await?;

                    // show the set as default dialog
                    to_ui
                        .send(tui::Event::Show(Screens::ProgrammingSetDefault).into())
                        .await?;
                } else {
                    error!("Programming language set default screen not found");
                }
            }
            tui::Event::SetDefaultProgrammingLanguage(programming_language) => {
                info!(
                    "UI event: Saving programming language as default: {:?}",
                    programming_language
                );
                self.config.set_programming_language(programming_language)?;
            }
            tui::Event::Homepage(url) => {
                info!("UI event: Launching browser with URL: {}", url);
                if let Err(e) = webbrowser::open(&url) {
                    error!("Failed to open browser: {}", e);
                }
            }
            tui::Event::LoadLessons(workshop_key) => {
                info!("UI event: Load lessons for workshop: {}", workshop_key);
            }
            _ => {
                // pass the key events to the current screen
                let current_screen = self.screen.load(Ordering::SeqCst).into();
                if let Some(screen_state) = self.screens.get_mut(&current_screen) {
                    return screen_state.handle_event(event.into(), to_ui).await;
                } else {
                    return Err(Error::Tui(format!(
                        "Unknown screen type: {}",
                        current_screen
                    )));
                }
            }
        }
        Ok(())
    }

    /// handle input events
    pub async fn handle_input_event(
        &mut self,
        event: event::Event,
        to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                // These key bindings work on every screen
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    info!("input event: Quit");
                    self.token.cancel();
                }
                KeyCode::Char('`') => {
                    info!("input event: Show Log");
                    to_ui.send(tui::Event::ToggleLog.into()).await?
                }
                _ => {
                    // pass the key events to the current screen
                    let current_screen = self.screen.load(Ordering::SeqCst).into();
                    if let Some(screen_state) = self.screens.get_mut(&current_screen) {
                        return screen_state.handle_event(event.into(), to_ui).await;
                    } else {
                        return Err(Error::Tui(format!(
                            "Unknown screen type: {}",
                            current_screen
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for App {
    async fn handle_event(
        &mut self,
        event: screens::Event,
        to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => self.handle_input_event(input_event, to_ui).await,
            screens::Event::Ui(ui_event) => self.handle_ui_event(ui_event, to_ui).await,
        }
    }

    fn render_screen(&mut self, area: Rect, buf: &mut Buffer) -> Result<(), Error> {
        // render the log if it is being show
        if self.log.load(Ordering::SeqCst) {
            if let Some(screen) = self.screens.get_mut(&Screens::Log) {
                screen.render_screen(area, buf)?;
            } else {
                error!("Log screen not found");
            }
        } else {
            // render the current screen
            let current_screen = self.screen.load(Ordering::SeqCst).into();
            if let Some(screen) = self.screens.get_mut(&current_screen) {
                screen.render_screen(area, buf)?;
            } else {
                error!("Unknown screen: {:?}", current_screen);
            }
        }
        Ok(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.render_screen(area, buf);
    }
}
