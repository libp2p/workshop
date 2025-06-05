use crate::{
    command::CommandRunner,
    evt, fs, languages,
    ui::tui::{
        self,
        screens::{self, Screen, Screens},
        Evt,
    },
    Error, Status,
};
use crossterm::event::{self, EventStream, KeyCode};
use futures::{future::FutureExt, StreamExt};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

const MAX_LOG_LINES: usize = 10000;

/// Tui implementation of the UI
pub struct App {
    /// The receiver from the logger
    from_logger: Receiver<String>,
    /// The status
    status: Arc<Mutex<Status>>,
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
    /// command runner for external processes
    command_runner: CommandRunner,
}

impl App {
    /// Create a new UI
    pub fn new(from_logger: Receiver<String>) -> Result<Self, Error> {
        debug!("Creating UI");
        let (sender, receiver) = tokio::sync::mpsc::channel(1_000_000);
        let command_runner = CommandRunner::new(sender.clone());

        Ok(Self {
            from_logger,
            status: Arc::new(Mutex::new(Status::load()?)),
            screens: Self::create_screens(),
            log: AtomicBool::new(false),
            screen: AtomicU8::new(Screens::Workshops as u8),
            token: CancellationToken::new(),
            receiver,
            sender,
            command_runner,
        })
    }

    // create the screens
    fn create_screens() -> HashMap<Screens, Box<dyn Screen>> {
        trace!("Creating screens");
        let mut screens = HashMap::<Screens, Box<dyn Screen>>::with_capacity(8);

        // Welcome Screen
        screens.insert(Screens::Welcome, Box::new(screens::Welcome::default()));

        // Workshop Selection Screen
        screens.insert(Screens::Workshops, Box::new(screens::Workshops::new()));

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

        // Lesson Screen
        screens.insert(Screens::Lesson, Box::new(screens::Lesson::default()));

        debug!("screens created: {:?}", screens.keys());
        screens
    }

    /// Get a reference to the command runner
    pub fn command_runner(&self) -> &CommandRunner {
        &self.command_runner
    }

    /// async run loop
    pub async fn run(&mut self) -> Result<(), Error> {
        // initialize the terminal
        let mut terminal = ratatui::init();

        // initialize the input event stream
        let mut reader = EventStream::new();

        // initialize the state
        let (workshop, lesson) = {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            (status.workshop(), status.lesson())
        };

        // send the correct initial message to restore the state
        match (workshop, lesson) {
            (None, _) => {
                self.sender
                    .send((Some(Screens::Workshops), tui::Event::LoadWorkshops).into())
                    .await?;
            }
            (Some(_), None) => {
                self.sender
                    .send((Some(Screens::Lessons), tui::Event::LoadLessons).into())
                    .await?;
            }
            (Some(_), Some(_)) => {
                self.sender
                    .send((Some(Screens::Lesson), tui::Event::LoadLesson).into())
                    .await?;
            }
        }

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
        {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            status.save()?;
        }
        ratatui::restore();

        Ok(())
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        screen: Option<Screens>,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        if let Some(dest_screen) = screen.clone() {
            // pass the event to the target screen
            if let Some(screen_state) = self.screens.get_mut(&dest_screen) {
                return screen_state
                    .handle_event((Some(dest_screen), event).into(), to_ui, status)
                    .await;
            }
        } else {
            match event {
                tui::Event::Quit => {
                    info!("Quit");
                    self.token.cancel();
                }
                tui::Event::ToggleLog => {
                    self.log.fetch_xor(true, Ordering::SeqCst);
                }
                tui::Event::Show(screen) => {
                    info!("Show screen: {}", screen);
                    self.screen.store(screen.clone() as u8, Ordering::SeqCst);
                }
                tui::Event::SetSpokenLanguage(spoken_language, default, next) => {
                    info!(
                        "Spoken language set: {}",
                        languages::spoken_name(spoken_language)
                    );

                    let (default, next): (bool, Option<Evt>) = match default {
                        Some(default) => {
                            debug!(
                                "Setting spoken language as default: {}, {}",
                                languages::spoken_name(spoken_language),
                                default
                            );
                            (default, next)
                        }
                        _ => {
                            debug!(
                                "Setting spoken language: {}",
                                languages::spoken_name(spoken_language)
                            );

                            // this is the event to send if the user selects "yes" in the dialog
                            let set_default_yes = evt!(
                                None,
                                tui::Event::SetSpokenLanguage(
                                    spoken_language,
                                    Some(true),
                                    next.clone(),
                                ),
                            );

                            // this is the event to send if the user selects "no" in the dialog
                            let set_default_no = evt!(
                                None,
                                tui::Event::SetSpokenLanguage(
                                    spoken_language,
                                    Some(false),
                                    next.clone(),
                                ),
                            );

                            // this is the event to send to initialize the dialog
                            let set_default = evt!(
                                Screens::SetDefault,
                                tui::Event::SetDefault(
                                    "Set as Default?".to_string(),
                                    Some(set_default_yes),
                                    Some(set_default_no),
                                ),
                            );
                            (false, Some(set_default))
                        }
                    };

                    // set the default spoken language
                    {
                        let mut status = self
                            .status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        status.set_spoken_language(spoken_language, default);
                    }

                    // send the next event if there is one
                    if let Some(next) = next {
                        to_ui.send(next.into()).await?;
                    }
                }
                tui::Event::SetProgrammingLanguage(programming_language, default, next) => {
                    info!(
                        "Programming language set: {}",
                        languages::programming_name(programming_language)
                    );

                    let (default, n): (bool, Option<Evt>) = match default {
                        Some(default) => {
                            debug!(
                                "Setting programming language as default: {}, {}, next: {:?}",
                                languages::programming_name(programming_language),
                                default,
                                next
                            );
                            (default, next)
                        }
                        _ => {
                            debug!(
                                "Setting programming language: {}, next: {:?}",
                                languages::programming_name(programming_language),
                                next
                            );

                            // this is the event to send if the user selects "yes" in the dialog
                            let set_default_yes = evt!(
                                None,
                                tui::Event::SetProgrammingLanguage(
                                    programming_language,
                                    Some(true),
                                    next.clone(),
                                ),
                            );

                            // this is the event to send if the user selects "no" in the dialog
                            let set_default_no = evt!(
                                None,
                                tui::Event::SetProgrammingLanguage(
                                    programming_language,
                                    Some(false),
                                    next.clone(),
                                ),
                            );

                            // this is the event to send to initialize the dialog
                            let set_default = evt!(
                                Screens::SetDefault,
                                tui::Event::SetDefault(
                                    "Set as Default?".to_string(),
                                    Some(set_default_yes),
                                    Some(set_default_no),
                                ),
                            );
                            (false, Some(set_default))
                        }
                    };

                    // set the default programming language
                    {
                        let mut status = self
                            .status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        status.set_programming_language(programming_language, default);
                    }

                    // send the next event if there is one
                    if let Some(n) = n {
                        to_ui.send(n.into()).await?;
                    }
                }
                tui::Event::SetWorkshop(workshop) => {
                    if let Some(workshop) = workshop {
                        info!("Setting workshop: {:?}", workshop);
                        let (programming_language, spoken_language) = {
                            let status = self
                                .status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            (status.programming_language(), status.spoken_language())
                        };

                        if programming_language.is_none() {
                            // this kicks off a cycle of selecting the programming language, asking
                            // if they want to set it as the default, and then coming back here
                            debug!("No programming language selected");

                            let set_workshop =
                                evt!(None, tui::Event::SetWorkshop(Some(workshop.clone())));
                            let change_programming_language = (
                                Some(Screens::Programming),
                                tui::Event::ChangeProgrammingLanguage(
                                    None,
                                    false,
                                    Some(set_workshop),
                                ),
                            );

                            to_ui.send(change_programming_language.into()).await?;
                        } else if spoken_language.is_none() {
                            // this kicks off a cycle of selecting the spoken language, asking
                            // if they want to set it as the default, and then coming back here
                            debug!("No spoken language selected");

                            let set_workshop =
                                evt!(None, tui::Event::SetWorkshop(Some(workshop.clone())));
                            let change_spoken_language = (
                                Some(Screens::Spoken),
                                tui::Event::ChangeSpokenLanguage(None, false, Some(set_workshop)),
                            );

                            to_ui.send(change_spoken_language.into()).await?;
                        } else {
                            // we have both languages selected, so we can proceed with setting the
                            // workshop, initializing the local workshop data and loading the lessons
                            info!("Workshop selected: {}", workshop);
                            {
                                let mut status = self
                                    .status
                                    .lock()
                                    .map_err(|e| Error::StatusLock(e.to_string()))?;
                                status.set_workshop(Some(workshop.clone()));
                                fs::workshops::init_data_dir(&workshop)?;
                            }
                            to_ui
                                .send((Some(Screens::Lessons), tui::Event::LoadLessons).into())
                                .await?;
                        }
                    } else {
                        debug!("Clearing workshop");
                        {
                            let mut status = self
                                .status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            status.set_workshop(None);
                        }
                        to_ui
                            .send((Some(Screens::Workshops), tui::Event::LoadWorkshops).into())
                            .await?;
                    }

                    /*

                    // Run dependency check using workshop data (with fallback to defaults)
                    if let Some(workshop_data) = fs::workshops::load(&workshop) {
                        info!("Running dependency check for workshop: '{}'", workshop);

                        // Get deps.py path using workshop model (handles defaults automatically)
                        match workshop_data
                            .get_deps_script_path(spoken_language, programming_language)
                        {
                            Ok(deps_script) => {
                                info!(
                                    "Attempting to run dependency script: {}",
                                    deps_script.display()
                                );
                                info!("Script exists: {}", deps_script.exists());

                                // Run dependency check in background
                                let command_runner = self.command_runner.clone();
                                let token = self.token.clone();
                                let sender = to_ui.clone();

                                tokio::spawn(async move {
                                    match command_runner
                                        .check_dependencies(&deps_script, &token)
                                        .await
                                    {
                                        Ok(result) => {
                                            info!(
                                                "Dependency check completed with exit code: {}",
                                                result.exit_code
                                            );
                                            // Send LoadLessons event after dependency check completes
                                            let _ = sender
                                                .send(
                                                    (
                                                        Some(Screens::Lessons),
                                                        tui::Event::LoadLessons,
                                                    )
                                                        .into(),
                                                )
                                                .await;
                                        }
                                        Err(e) => {
                                            error!("Failed to check dependencies: {}", e);
                                            // Still proceed to lessons even if dependency check fails
                                            let _ = sender
                                                .send(
                                                    (
                                                        Some(Screens::Lessons),
                                                        tui::Event::LoadLessons,
                                                    )
                                                        .into(),
                                                )
                                                .await;
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Failed to get deps script path: {}", e);
                                // Proceed to lessons even if we can't get the script path
                                to_ui
                                    .send(
                                        (Some(Screens::Lessons), tui::Event::LoadLessons)
                                            .into(),
                                    )
                                    .await?;
                            }
                        }
                    } else {
                        error!("Failed to load workshop data for: {}", workshop);
                        // Proceed to lessons even if workshop data loading fails
                        to_ui
                            .send((Some(Screens::Lessons), tui::Event::LoadLessons).into())
                            .await?;
                    }
                    */
                }
                tui::Event::SetLesson(lesson) => {
                    info!("Lesson set: {:?}", lesson);
                    if let Some(lesson) = lesson {
                        info!("Setting lesson: {:?}", lesson);
                        {
                            let mut status = status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            status.set_lesson(Some(lesson.clone()));
                        }
                        to_ui
                            .send((Some(Screens::Lesson), tui::Event::LoadLesson).into())
                            .await?;
                    } else {
                        info!("Clearing lesson");
                        {
                            let mut status = status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            status.set_lesson(None);
                        }
                        to_ui
                            .send((Some(Screens::Lessons), tui::Event::LoadLessons).into())
                            .await?;
                    }
                }
                tui::Event::CommandStarted => {
                    info!("Command started - showing log screen");
                    self.log.store(true, Ordering::SeqCst);
                }
                tui::Event::CommandCompleted { success } => {
                    info!("Command completed - success: {}", success);
                    if success {
                        // Hide log screen on successful command completion
                        self.log.store(false, Ordering::SeqCst);
                    }
                    // If command failed, leave log screen visible so user can see errors
                }
                tui::Event::CommandOutput(output) => {
                    // Forward command output directly to Log screen
                    to_ui
                        .send((Some(Screens::Log), tui::Event::Log(output)).into())
                        .await?;
                }
                tui::Event::CheckSolution => {
                    info!("Check solution");
                    // Get current status information
                    let (spoken, programming, workshop, lesson) = {
                        let status = status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        (
                            status.spoken_language(),
                            status.programming_language(),
                            status.workshop(),
                            status.lesson(),
                        )
                    };

                    // Check if we have required workshop and lesson
                    if let (Some(workshop), Some(lesson)) = (workshop, lesson) {
                        if let Some(workshop_data) = fs::workshops::load(&workshop) {
                            info!("Running solution check for lesson: '{}'", lesson);

                            // Get lesson directory path using workshop model (handles defaults automatically)
                            match workshop_data.get_lesson_dir_path(&lesson, spoken, programming) {
                                Ok(lesson_dir) => {
                                    info!(
                                        "Solution check lesson directory: {}",
                                        lesson_dir.display()
                                    );

                                    // Spawn async task to run solution check
                                    let command_runner = self.command_runner.clone();
                                    let token = self.token.clone();
                                    let sender = to_ui.clone();

                                    tokio::spawn(async move {
                                        match command_runner
                                            .check_solution(&lesson_dir, &token)
                                            .await
                                        {
                                            Ok(result) => {
                                                let event = if result.success {
                                                    tui::Event::SolutionSuccess
                                                } else {
                                                    tui::Event::SolutionFailure
                                                };
                                                let _ = sender.send((None, event).into()).await;
                                            }
                                            Err(e) => {
                                                error!("Failed to check solution: {}", e);
                                                let _ = sender
                                                    .send(
                                                        (None, tui::Event::SolutionFailure).into(),
                                                    )
                                                    .await;
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Failed to get lesson directory path: {}", e);
                                    to_ui
                                        .send((None, tui::Event::SolutionFailure).into())
                                        .await?;
                                }
                            }
                        } else {
                            error!("Failed to load workshop data for: {}", workshop);
                            to_ui
                                .send((None, tui::Event::SolutionFailure).into())
                                .await?;
                        }
                    } else {
                        error!("Cannot check solution: missing workshop or lesson selection");
                        to_ui
                            .send((None, tui::Event::SolutionFailure).into())
                            .await?;
                    }
                }
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
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                // These key bindings work on every screen
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.token.cancel();
                }
                KeyCode::Char('`') => to_ui.send((None, tui::Event::ToggleLog).into()).await?,
                _ => {
                    if self.log.load(Ordering::SeqCst) {
                        // send key events to the log window if it is showing
                        if let Some(screen) = self.screens.get_mut(&Screens::Log) {
                            return screen.handle_event(event.into(), to_ui, status).await;
                        } else {
                            error!("Log screen not found");
                        }
                    } else {
                        // pass the key events to the current screen
                        let current_screen = self.screen.load(Ordering::SeqCst).into();
                        if let Some(screen) = self.screens.get_mut(&current_screen) {
                            return screen.handle_event(event.into(), to_ui, status).await;
                        } else {
                            return Err(Error::Tui(format!(
                                "Unknown screen type: {}",
                                current_screen
                            )));
                        }
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
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => {
                self.handle_input_event(input_event, to_ui, status.clone())
                    .await
            }
            screens::Event::Ui(screen, ui_event) => {
                self.handle_ui_event(screen, ui_event, to_ui, status.clone())
                    .await
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
