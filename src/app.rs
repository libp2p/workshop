use crate::{
    languages::{programming, spoken},
    ui::tui::{
        self,
        screens::{self, Screen, Screens},
    },
    Error, Status,
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

const MAX_LOG_LINES: usize = 10000;

/// Tui implementation of the UI
pub struct App {
    /// The receiver from the logger
    from_logger: Receiver<String>,
    /// The status
    status: Status,
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
}

impl App {
    /// Create a new UI
    pub fn new(from_logger: Receiver<String>) -> Result<Self, Error> {
        info!("creating UI");
        let (sender, receiver) = tokio::sync::mpsc::channel(1_000_000);

        Ok(Self {
            from_logger,
            status: Status::load()?,
            screens: Self::create_screens(),
            log: AtomicBool::new(false),
            screen: AtomicU8::new(Screens::Workshops as u8),
            token: CancellationToken::new(),
            receiver,
            sender,
        })
    }

    // create the screens
    fn create_screens() -> HashMap<Screens, Box<dyn Screen>> {
        info!("creating screens");
        let mut screens = HashMap::<Screens, Box<dyn Screen>>::with_capacity(8);

        // Welcome Screen
        screens.insert(Screens::Welcome, Box::new(screens::Welcome::default()));

        // Workshop Selection Screen
        screens.insert(Screens::Workshops, Box::new(screens::Workshops::default()));

        // Log Screen
        screens.insert(Screens::Log, Box::new(screens::Log::new(MAX_LOG_LINES)));

        // License Screen
        screens.insert(Screens::License, Box::new(screens::License::default()));

        // Spoken Language Selection Screen
        screens.insert(Screens::Spoken, Box::new(screens::Spoken::default()));

        // Programming Language Selection Screen
        screens.insert(
            Screens::Programming,
            Box::new(screens::Programming::default()),
        );

        // Set Default Confirmation Screen
        screens.insert(
            Screens::SetDefault,
            Box::new(screens::SetDefault::default()),
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
        self.sender
            .send((Some(Screens::Workshops), tui::Event::LoadWorkshops).into())
            .await?;

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
                    self.sender.send((Some(Screens::Log), tui::Event::Log(msg)).into()).await?;
                }

                // get the next event in the queue
                Some(evt) = self.receiver.recv() => {
                    self.handle_event(evt, self.sender.clone(), self.status.clone()).await?;
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
        self.status.save()?;
        ratatui::restore();

        Ok(())
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        screen: Option<Screens>,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
        status: Status,
    ) -> Result<(), Error> {
        if let Some(dest_screen) = screen.clone() {
            // pass the event to the target screen
            if let Some(screen_state) = self.screens.get_mut(&dest_screen) {
                return screen_state
                    .handle_event((screen, event).into(), to_ui, status)
                    .await;
            }
        } else {
            match event {
                tui::Event::Quit => {
                    info!("UI event: Quit");
                    self.token.cancel();
                }
                tui::Event::ToggleLog => {
                    info!("UI event: Toggle Log");
                    self.log.fetch_xor(true, Ordering::SeqCst);
                }
                tui::Event::Show(screen) => {
                    info!("UI event: Show screen: {}", screen);
                    self.screen.store(screen.clone() as u8, Ordering::SeqCst);
                }
                tui::Event::SetSpokenLanguage(spoken_language, default) => {
                    info!("UI event: Spoken language set: {:?}", spoken_language);
                    if let Some(default) = default {
                        info!("Setting spoken language as default: {:?}", spoken_language);
                        self.status.set_spoken_language(spoken_language, default);
                        to_ui
                            .send(
                                (Some(screens::Screens::Workshops), tui::Event::LoadWorkshops)
                                    .into(),
                            )
                            .await?;
                    } else {
                        info!("Setting spoken language: {:?}", spoken_language);
                        self.status.set_spoken_language(spoken_language, false);
                        to_ui
                            .send(
                                (
                                    Some(Screens::SetDefault),
                                    tui::Event::SetDefault(
                                        "Set as Default?".to_string(),
                                        Some(Box::new(tui::Event::SetSpokenLanguage(
                                            spoken_language,
                                            Some(true),
                                        ))),
                                    ),
                                )
                                    .into(),
                            )
                            .await?;
                    }
                }
                tui::Event::SetProgrammingLanguage(programming_language, default) => {
                    info!(
                        "UI event: Programming language set: {:?}",
                        programming_language
                    );
                    if let Some(default) = default {
                        info!(
                            "Setting programming language as default: {:?}",
                            programming_language
                        );
                        self.status
                            .set_programming_language(programming_language, default);
                        to_ui
                            .send(
                                (Some(screens::Screens::Workshops), tui::Event::LoadWorkshops)
                                    .into(),
                            )
                            .await?;
                    } else {
                        info!("Setting programming language: {:?}", programming_language);
                        self.status
                            .set_programming_language(programming_language, false);
                        to_ui
                            .send(
                                (
                                    Some(Screens::SetDefault),
                                    tui::Event::SetDefault(
                                        "Set as Default?".to_string(),
                                        Some(Box::new(tui::Event::SetProgrammingLanguage(
                                            programming_language,
                                            Some(true),
                                        ))),
                                    ),
                                )
                                    .into(),
                            )
                            .await?;
                    }
                }
                /*
                tui::Event::LoadWorkshops => {
                    info!("UI event: Load Workshops");
                    let workshop_data = crate::fs::get_workshops_data(self.config.data_dir())?
                        .into_iter()
                        .filter(|(_, workshop_data)| {
                            workshop_data
                                .is_selected(self.spoken_language, self.programming_language)
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
                                tui::Event::SetEvent(Box::new(
                                    tui::Event::SetDefaultSpokenLanguage(spoken_language),
                                ))
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
                */
                _ => {
                    // pass the event to every screen
                    for screen in Screens::iter() {
                        if let Some(screen_state) = self.screens.get_mut(&screen) {
                            screen_state
                                .handle_event(
                                    (Some(screen), event.clone()).into(),
                                    to_ui.clone(),
                                    status.clone(),
                                )
                                .await?;
                        } else {
                            error!("Screen not found: {:?}", screen);
                        }
                    }
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
        status: Status,
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
                    to_ui.send((None, tui::Event::ToggleLog).into()).await?
                }
                _ => {
                    // pass the key events to the current screen
                    let current_screen = self.screen.load(Ordering::SeqCst).into();
                    if let Some(screen_state) = self.screens.get_mut(&current_screen) {
                        return screen_state.handle_event(event.into(), to_ui, status).await;
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
        status: Status,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => {
                self.handle_input_event(input_event, to_ui, status).await
            }
            screens::Event::Ui(screen, ui_event) => {
                self.handle_ui_event(screen, ui_event, to_ui, status).await
            }
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
