use crate::{Error, Message, State};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tracing::info;

/// The workshop engine
#[derive(Debug)]
pub struct Engine {
    /// The channel from the UI
    from_ui: Receiver<Message>,
    /// The engine state
    state: State,
}

impl Engine {
    /// Creates a new instance of the engine
    pub fn new(to_ui: Sender<Message>, from_ui: Receiver<Message>) -> Result<Self, Error> {
        info!("engine initialzied");
        // Initialize the engine with the given data directory and password
        let engine = Engine {
            from_ui,
            state: State::Uninitialized(to_ui),
        };
        Ok(engine)
    }

    /// Runs the engine
    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            select! {
                // Process messages from the UI
                Some(msg) = self.from_ui.recv() => {
                    match msg {
                        Message::Config { data_dir, pwd, spoken_language, programming_language } => {
                            info!("Configuring engine with:");
                            info!("- data dir: {}", data_dir.display());
                            info!("- pwd: {}", pwd.display());
                            info!("- preferred spoken language: {}", spoken_language.get_name_in_english());
                            info!("- preferred programming language: {}", programming_language.get_name());

                            // Initialize the engine state with the given data directory and password
                            self.state = self.state.config(&data_dir, &pwd, spoken_language, programming_language).await?;
                        },
                        Message::ChangeSpokenLanguage => {
                            info!("Changing spoken language");
                            // Change the spoken language in the engine state
                            self.state = self.state.change_spoken_language().await?;
                        },
                        Message::SetSpokenLanguage { code } => {
                            info!("Setting spoken language to: {}", code.get_name_in_english());
                            // Set the spoken language in the engine state
                            self.state = self.state.set_spoken_language(code).await?;
                        },
                        Message::ChangeProgrammingLanguage => {
                            info!("Changing programming language");
                            // Change the programming language in the engine state
                            self.state = self.state.change_programming_language().await?;
                        },
                        Message::SetProgrammingLanguage { code } => {
                            info!("Setting programming language to: {}", code.get_name());
                            // Set the programming language in the engine state
                            self.state = self.state.set_programming_language(code).await?;
                        },
                        Message::Quit => {
                            self.state = self.state.quit().await?;
                            return Ok(());
                        }
                        _ => {
                        }
                    }
                }
            }
        }
    }
}
