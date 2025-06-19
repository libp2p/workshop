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
use futures_timer::Delay;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

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

impl Drop for App {
    fn drop(&mut self) {
        // cancel the token to stop the run loop
        self.token.cancel();
        ratatui::restore();
    }
}

impl App {
    /// Create a new UI
    pub fn new(from_logger: Receiver<String>) -> Result<Self, Error> {
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
        screens.insert(Screens::Lessons, Box::new(screens::Lessons::new()));

        // Lesson Screen
        screens.insert(Screens::Lesson, Box::new(screens::Lesson::default()));

        screens
    }

    /// Get a reference to the command runner
    pub fn command_runner(&self) -> &CommandRunner {
        &self.command_runner
    }

    /// Setup python
    async fn detect_python(&mut self) -> Result<(), Error> {
        // try to get the python executable and minimum version from the status
        let (py_exe, py_min_ver) = {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            (
                status.python_executable().map(String::from),
                status.python_minimum_version().to_string(),
            )
        };

        // if we don't have the path, try to find it
        if py_exe.is_none() {
            let python_executable = fs::application::find_python_executable(py_min_ver).await?;
            debug!("Setting Python executable: {}", python_executable);
            {
                let mut status = self
                    .status
                    .lock()
                    .map_err(|e| Error::StatusLock(e.to_string()))?;
                status.set_python_executable(&python_executable, true);
            }
        }

        Ok(())
    }

    // Setup docker compose
    async fn detect_docker_compose(&mut self) -> Result<(), Error> {
        // try to get the docker executable from the status
        let (docker_compose_exe, docker_compose_min_ver) = {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            (
                status.docker_compose_executable().map(String::from),
                status.docker_compose_minimum_version().to_string(),
            )
        };

        // if we don't have the path, try to find it
        if docker_compose_exe.is_none() {
            let docker_compose_executable =
                fs::application::find_docker_compose_executable(docker_compose_min_ver).await?;
            debug!(
                "Setting docker compose executable: {}",
                docker_compose_executable
            );
            {
                let mut status = self
                    .status
                    .lock()
                    .map_err(|e| Error::StatusLock(e.to_string()))?;
                status.set_docker_compose_executable(&docker_compose_executable, true);
            }
        }

        Ok(())
    }

    /// Setup git
    async fn detect_git(&mut self) -> Result<(), Error> {
        // try to get the git executable and minimum version from the status
        let (git_exe, git_min_ver) = {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            (
                status.git_executable().map(String::from),
                status.git_minimum_version().to_string(),
            )
        };

        // if we don't have the path, try to find it
        if git_exe.is_none() {
            let git_executable = fs::application::find_git_executable(git_min_ver).await?;
            debug!("Setting Git executable: {}", git_executable);
            {
                let mut status = self
                    .status
                    .lock()
                    .map_err(|e| Error::StatusLock(e.to_string()))?;
                status.set_git_executable(&git_executable, true);
            }
        }

        Ok(())
    }

    /// Queue up the initial events for the application
    async fn initial_events(&mut self, install: Option<String>) -> Result<(), Error> {
        // initialize the state
        let (workshop, lesson) = {
            let status = self
                .status
                .lock()
                .map_err(|e| Error::StatusLock(e.to_string()))?;
            (
                status.workshop().map(String::from),
                status.lesson().map(String::from),
            )
        };

        // send the correct initial message to restore the state
        let event = match (workshop, lesson) {
            (None, _) => {
                let load_workshops = evt!(Screens::Workshops, tui::Event::LoadWorkshops);
                evt!(None, tui::Event::HideLog(Some(load_workshops)))
            }
            (Some(workshop), lesson) => {
                // re-runs the deps.py check and if it succeeds will drop you back into the lesson
                let load = if lesson.is_none() {
                    evt!(Screens::Lessons, tui::Event::LoadLessons)
                } else {
                    evt!(Screens::Lesson, tui::Event::LoadLesson)
                };
                let hide_log = evt!(None, tui::Event::HideLog(Some(load)));
                evt!(
                    None,
                    tui::Event::CheckDeps(workshop.to_string(), Some(hide_log), None,),
                )
            }
        };

        // if there's a workshop to install, do that first
        if let Some(install) = install {
            // if we are installing a workshop, send the install event
            let install_event = evt!(None, tui::Event::InstallWorkshop(install, event.into()));
            self.sender.send(install_event.into()).await?;
        } else {
            self.sender.send(event.into()).await?;
        }

        Ok(())
    }

    /// async run loop
    pub async fn run(&mut self, install: Option<String>) -> Result<(), Error> {
        // initialize the terminal
        let mut terminal = ratatui::init();

        // initialize the input event stream
        let mut reader = EventStream::new();

        // the timeout
        let mut timeout = Delay::new(Duration::from_secs(600));

        // try to get the python executable and minimum version from the status
        if self.detect_python().await.is_err() {
            error!("Failed to detect Python executable or version");
            return Err(fs::Error::NoPythonExecutable.into());
        }

        // try to get the docker compose executable and minimum version from the status
        if self.detect_docker_compose().await.is_err() {
            error!("Failed to detect Docker Compose executable or version");
            return Err(fs::Error::NoDockerComposeExecutable.into());
        }

        // try to get the git executable and minimum version from the status
        if self.detect_git().await.is_err() {
            error!("Failed to detect Git executable or version");
            return Err(fs::Error::NoGitExecutable.into());
        }

        // queue up the initial events
        if self.initial_events(install).await.is_err() {
            error!("Failed to queue initial events");
            return Err(Error::InitialEvents);
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

                // check the timeout
                _ = &mut timeout => {}

                // check if we should quit
                _ = self.token.cancelled() => {
                    debug!("cancelation token triggered, quitting...");
                    break 'run;
                }
            }

            if self.log.load(Ordering::SeqCst) {
                // if the log is visible, set a timer to redraw the UI @ 60 FPS
                timeout = Delay::new(Duration::from_secs_f64(1.0 / 60.0));
            } else {
                // otherwise set the timer to 10 minutes
                timeout = Delay::new(Duration::from_secs(600));
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
                    self.token.cancel();
                }
                tui::Event::ToggleLog => {
                    self.log.fetch_xor(true, Ordering::SeqCst);
                }
                tui::Event::ShowLog(next) => {
                    self.log.store(true, Ordering::SeqCst);
                    if let Some(next) = next {
                        to_ui.send(next.into()).await?;
                    }
                }
                tui::Event::HideLog(next) => {
                    self.log.store(false, Ordering::SeqCst);
                    if let Some(next) = next {
                        to_ui.send(next.into()).await?;
                    }
                }
                tui::Event::Delay(duration, next) => {
                    // send a delay event to the UI
                    tokio::time::sleep(duration).await;
                    if let Some(next) = next {
                        to_ui.send(next.into()).await?;
                    }
                }
                tui::Event::Show(screen) => {
                    debug!("Show screen: {}", screen);
                    self.screen.store(screen.clone() as u8, Ordering::SeqCst);
                }
                tui::Event::SetSpokenLanguage(spoken_language, default, next) => {
                    debug!(
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
                    debug!(
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
                tui::Event::SetWorkshop(workshop, all_languages) => {
                    if let Some(workshop) = workshop {
                        debug!("Setting workshop: {:?}", workshop);
                        let (spoken_language, programming_language) = {
                            let status = self
                                .status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            (status.spoken_language(), status.programming_language())
                        };

                        debug!(
                            "Spoken language: {:?}, Programming language: {:?}",
                            spoken_language, programming_language
                        );

                        debug!(
                            "All languages for workshop {}: {:?}",
                            workshop, all_languages
                        );

                        if spoken_language.is_none() {
                            // this kicks off a cycle of selecting the spoken language, asking
                            // if they want to set it as the default, and then coming back here
                            debug!("No spoken language selected");

                            let set_workshop = evt!(
                                None,
                                tui::Event::SetWorkshop(
                                    Some(workshop.clone()),
                                    all_languages.clone()
                                )
                            );
                            let change_spoken_language = (
                                Some(Screens::Spoken),
                                tui::Event::ChangeSpokenLanguage(
                                    all_languages.clone(),
                                    None,
                                    false,
                                    Some(set_workshop),
                                ),
                            );

                            to_ui.send(change_spoken_language.into()).await?;
                        } else if programming_language.is_none() {
                            // this kicks off a cycle of selecting the programming language, asking
                            // if they want to set it as the default, and then coming back here
                            debug!("No programming language selected");

                            let set_workshop = evt!(
                                None,
                                tui::Event::SetWorkshop(
                                    Some(workshop.clone()),
                                    all_languages.clone()
                                )
                            );
                            let change_programming_language = (
                                Some(Screens::Programming),
                                tui::Event::ChangeProgrammingLanguage(
                                    all_languages.clone(),
                                    None,
                                    false,
                                    Some(set_workshop),
                                ),
                            );

                            to_ui.send(change_programming_language.into()).await?;
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
                            let load_lessons = evt!(Screens::Lessons, tui::Event::LoadLessons);
                            let hide_log = evt!(None, tui::Event::HideLog(Some(load_lessons)));
                            let check_deps = evt!(
                                None,
                                tui::Event::CheckDeps(workshop.clone(), Some(hide_log), None,),
                            );
                            to_ui.send(check_deps.into()).await?;
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
                }
                tui::Event::SetLesson(lesson) => {
                    debug!("Lesson set: {:?}", lesson);
                    if let Some(lesson) = lesson {
                        debug!("Setting lesson: {:?}", lesson);
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
                        debug!("Clearing lesson");
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
                tui::Event::CheckDeps(workshop, success, failed) => {
                    // Run dependency check using workshop data (with fallback to defaults)
                    if let Some(workshop_data) = fs::workshops::load(&workshop) {
                        let (programming_language, spoken_language, python_executable) = {
                            let status = self
                                .status
                                .lock()
                                .map_err(|e| Error::StatusLock(e.to_string()))?;
                            (
                                status.programming_language(),
                                status.spoken_language(),
                                status.python_executable().map(String::from),
                            )
                        };

                        let py_exe = python_executable.ok_or(fs::Error::NoPythonExecutable)?;

                        let show_log = evt!(None, tui::Event::ShowLog(None));
                        to_ui.send(show_log.into()).await?;

                        let running = evt!(
                            Screens::Log,
                            tui::Event::Log(format!(
                                "r Running dependency check: {}, {}, {}",
                                workshop,
                                languages::spoken_name(spoken_language),
                                languages::programming_name(programming_language)
                            ))
                        );
                        to_ui.send(running.into()).await?;

                        // Get deps.py path using workshop model (handles defaults automatically)
                        match workshop_data
                            .get_deps_script_path(spoken_language, programming_language)
                        {
                            Ok(deps_script) => {
                                debug!(
                                    "Attempting to run dependency script: {}",
                                    deps_script.display()
                                );
                                debug!("Script exists: {}", deps_script.exists());

                                // Run dependency check in background
                                let command_runner = self.command_runner.clone();
                                let token = self.token.clone();
                                let sender = to_ui.clone();

                                tokio::spawn(async move {
                                    match command_runner
                                        .check_dependencies(&py_exe, &deps_script, &token)
                                        .await
                                    {
                                        Ok(result) => {
                                            let _ = sender
                                                .send(
                                                    (
                                                        Some(Screens::Log),
                                                        tui::Event::CommandCompleted(
                                                            result, success, failed,
                                                        ),
                                                    )
                                                        .into(),
                                                )
                                                .await;
                                        }
                                        Err(e) => {
                                            let _ = sender
                                                .send(
                                                    (
                                                        Some(Screens::Log),
                                                        tui::Event::Log(format!(
                                                            "! check deps failed: {e}"
                                                        )),
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
                                if let Some(failed) = failed {
                                    let _ = to_ui.send(failed.into()).await;
                                }
                            }
                        }
                    } else {
                        error!("Failed to load workshop data for: {}", workshop);
                        if let Some(failed) = failed {
                            let _ = to_ui.send(failed.into()).await;
                        }
                    }
                }
                tui::Event::CheckSolution(success, failed) => {
                    debug!("Check solution");
                    // Get current status information
                    let (
                        spoken,
                        programming,
                        workshop,
                        lesson,
                        python_executable,
                        docker_compose_executable,
                    ) = {
                        let status = status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        (
                            status.spoken_language(),
                            status.programming_language(),
                            status.workshop().map(String::from),
                            status.lesson().map(String::from),
                            status.python_executable().map(String::from),
                            status.docker_compose_executable().map(String::from),
                        )
                    };

                    let py_exe = python_executable.ok_or(fs::Error::NoPythonExecutable)?;
                    let dc_exe =
                        docker_compose_executable.ok_or(fs::Error::NoDockerComposeExecutable)?;

                    // Check if we have required workshop and lesson
                    if let (Some(workshop), Some(lesson)) = (workshop, lesson) {
                        if let Some(workshop_data) = fs::workshops::load(&workshop) {
                            let show_log = evt!(None, tui::Event::ShowLog(None));
                            to_ui.send(show_log.into()).await?;

                            let running = evt!(
                                Screens::Log,
                                tui::Event::Log(format!("r Running solution check: {lesson}"))
                            );
                            to_ui.send(running.into()).await?;

                            // Get lesson directory path using workshop model (handles defaults automatically)
                            match workshop_data.get_lesson_dir_path(&lesson, spoken, programming) {
                                Ok(lesson_dir) => {
                                    debug!(
                                        "Solution check lesson directory: {}",
                                        lesson_dir.display()
                                    );

                                    // Spawn async task to run solution check
                                    let command_runner = self.command_runner.clone();
                                    let token = self.token.clone();
                                    let sender = to_ui.clone();

                                    tokio::spawn(async move {
                                        match command_runner
                                            .check_solution(&dc_exe, &py_exe, &lesson_dir, &token)
                                            .await
                                        {
                                            Ok(result) => {
                                                let _ = sender
                                                    .send(
                                                        (
                                                            Some(Screens::Log),
                                                            tui::Event::CommandCompleted(
                                                                result, success, failed,
                                                            ),
                                                        )
                                                            .into(),
                                                    )
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = sender
                                                    .send(
                                                        (
                                                            Some(Screens::Log),
                                                            tui::Event::Log(format!(
                                                                "! check solution failed: {e}"
                                                            )),
                                                        )
                                                            .into(),
                                                    )
                                                    .await;
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Failed to get lesson directory path: {}", e);
                                    if let Some(failed) = failed {
                                        let _ = to_ui.send(failed.into()).await;
                                    }
                                }
                            }
                        } else {
                            error!("Failed to load workshop data for: {}", workshop);
                            if let Some(failed) = failed {
                                let _ = to_ui.send(failed.into()).await;
                            }
                        }
                    } else {
                        error!("Cannot check solution: missing workshop or lesson selection");
                        if let Some(failed) = failed {
                            let _ = to_ui.send(failed.into()).await;
                        }
                    }
                }
                tui::Event::InstallWorkshop(url, next) => {
                    // Get current status information
                    let git_executable = {
                        let status = status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        status.git_executable().map(String::from)
                    };
                    let git_exe = git_executable.ok_or(fs::Error::NoGitExecutable)?;

                    let show_log = evt!(None, tui::Event::ShowLog(None));
                    to_ui.send(show_log.into()).await?;

                    let running = evt!(
                        Screens::Log,
                        tui::Event::Log(format!("r Installing workshop from: {url}",))
                    );
                    to_ui.send(running.into()).await?;

                    debug!("Attempting to clone the workshop from: {url}");

                    // Run dependency check in background
                    let command_runner = self.command_runner.clone();
                    let token = self.token.clone();
                    let sender = to_ui.clone();
                    let data_dir = fs::application::data_dir()?;

                    tokio::spawn(async move {
                        match command_runner
                            .install_workshop(&git_exe, &url, &data_dir, &token)
                            .await
                        {
                            Ok(result) => {
                                let _ = sender
                                    .send(
                                        (
                                            Some(Screens::Log),
                                            tui::Event::CommandCompleted(
                                                result,
                                                next.clone(),
                                                next.clone(),
                                            ),
                                        )
                                            .into(),
                                    )
                                    .await;
                            }
                            Err(e) => {
                                let _ = sender
                                    .send(
                                        (
                                            Some(Screens::Log),
                                            tui::Event::Log(format!(
                                                "! workshop install failed: {e}"
                                            )),
                                        )
                                            .into(),
                                    )
                                    .await;
                            }
                        }
                    });
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
                                "Unknown screen type: {current_screen}",
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
