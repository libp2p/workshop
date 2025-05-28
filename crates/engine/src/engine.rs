use crate::{Config, Error, Fs, Message, State, Workshop};
use futures::stream::StreamExt;
use languages::{programming, spoken};
use std::collections::HashMap;
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
    /// The channel to the UI
    to_ui: Sender<Message>,
    /// The filesystem abstraction
    fs: Fs,
    /// The engine state
    state: Vec<State>,
}

impl Engine {
    /// Creates a new instance of the engine
    pub fn new(to_ui: Sender<Message>, from_ui: Receiver<Message>) -> Result<Self, Error> {
        info!("(engine) engine initialzied");
        // Initialize the engine with the given data directory and password
        Ok(Engine {
            from_ui,
            to_ui,
            fs: Fs::default(),
            state: vec![State::Nil],
        })
    }

    /// Runs the engine
    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            select! {
                // Process messages from the UI
                Some(msg) = self.from_ui.recv() => {
                    let _ = match msg {
                        Message::Config { config } => {
                            info!("(engine) Configuring engine with:");
                            info!("(engine) - data dir: {}", config.data_dir().display());
                            info!("(engine) - pwd: {}", config.pwd().display());
                            info!("(engine) - preferred spoken language: {:?}", config.spoken_language());
                            info!("(engine) - preferred programming language: {:?}", config.programming_language());

                            // Initialize the engine state with the given data directory and password
                            self.config(config).await
                        },
                        Message::GetLicense { name } => {
                            info!("(engine) Getting license for: {}", name);
                            // Get the license text for the given workshop
                            self.get_license(&name).await
                        },
                        Message::ChangeSpokenLanguage => {
                            info!("(engine) Changing spoken language");
                            // Change the spoken language in the engine state
                            self.change_spoken_language().await
                        },
                        Message::SetSpokenLanguage { spoken_language } => {
                            match spoken_language {
                                Some(code) => {
                                    info!("(engine) Setting spoken language to: {}", code.get_name_in_english());
                                }
                                None => {
                                    info!("(engine) Setting spoken language to: Any");
                                }
                            }
                            // Set the spoken language in the engine state
                            self.set_spoken_language(spoken_language).await
                        },
                        Message::ChangeProgrammingLanguage => {
                            info!("(engine) Changing programming language");
                            // Change the programming language in the engine state
                            self.change_programming_language().await
                        },
                        Message::SetProgrammingLanguage { programming_language } => {
                            match programming_language {
                                Some(code) => {
                                    info!("(engine) Setting programming language to: {}", code.get_name());
                                }
                                None => {
                                    info!("(engine) Setting programming language to: Any");
                                }
                            }
                            // Set the programming language in the engine state
                            self.set_programming_language(programming_language).await
                        },
                        Message::SetWorkshop { name } => {
                            info!("(engine) Setting workshop to: {}", name);
                            // Set the workshop in the engine state
                            self.set_workshop(&name).await
                        },
                        Message::Back => {
                            info!("(engine) Going back to previous state");
                            // Go back to the previous state in the engine state
                            self.back().await
                        },
                        Message::Quit => {
                            match self.quit().await {
                                Ok(_) => {
                                    info!("(engine) Quitting engine");
                                    // Quit the engine
                                    return Ok(());
                                }
                                err @ Err(_) => err
                            }
                        }
                        _ => {
                            Ok(())
                        }
                    }.map_err(|err| {
                        self.state.push(State::Error(err.to_string()));
                    });

                    // send the next message to the UI
                    self.send_message().await?;
                }
            }
        }
    }

    async fn send_message(&self) -> Result<(), Error> {
        if let Some(state) = self.state.last() {
            match state {
                State::Nil => {
                    info!("(engine) Tried sending message when in Nil");
                }
                State::SelectWorkshop(workshops_data) => {
                    info!(
                        "(engine) Sending select workshops to {:?}",
                        workshops_data.keys().collect::<Vec<_>>()
                    );
                    let mut workshops =
                        HashMap::<String, Workshop>::with_capacity(workshops_data.len());
                    let mut descriptions =
                        HashMap::<String, String>::with_capacity(workshops.len());
                    let mut setup_instructions =
                        HashMap::<String, String>::with_capacity(workshops.len());
                    let spoken_language = self.fs.get_spoken_language();
                    futures::stream::iter(workshops_data.iter())
                        .try_for_each(|(name, workshop)| async move {
                            workshops.insert(
                                name.clone(),
                                workshop.get_metadata(spoken_language.clone()).await?,
                            );
                            descriptions.insert(
                                name.clone(),
                                workshop.get_description(spoken_language.clone()).await?,
                            );
                            setup_instructions.insert(
                                name.clone(),
                                workshop
                                    .get_setup_instructions(spoken_language.clone())
                                    .await?,
                            );
                            Ok(())
                        })
                        .collect::<Result<(), Error>>()?;
                    self.to_ui
                        .send(Message::SelectWorkshop {
                            workshops,
                            descriptions,
                            setup_instructions,
                            spoken_language: self.fs.get_spoken_language(),
                            programming_language: self.fs.get_programming_language(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::SelectLesson(lessons) => {
                    info!("(engine) Sending select lessons message {:?}", lessons);
                    self.to_ui
                        .send(Message::SelectLesson {
                            lessons: lessons.to_vec(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::SelectSpokenLanguage { spoken_languages } => {
                    info!(
                        "(engine) Sending select language message: {:?}",
                        spoken_languages
                    );
                    self.to_ui
                        .send(Message::SelectSpokenLanguage {
                            spoken_languages: spoken_languages.to_vec(),
                            spoken_language: self.fs.get_spoken_language(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::SetSpokenLanguageDefault { spoken_language } => {
                    info!(
                        "(engine) Sending set default spoken language message: {:?}",
                        spoken_language
                    );
                    self.to_ui
                        .send(Message::SetSpokenLanguageDefault {
                            spoken_language: *spoken_language,
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::SelectProgrammingLanguage {
                    programming_languages,
                } => {
                    info!(
                        "(engine) Sending select programming language message: {:?}",
                        programming_languages
                    );
                    self.to_ui
                        .send(Message::SelectProgrammingLanguage {
                            programming_languages: programming_languages.to_vec(),
                            programming_language: self.fs.get_programming_language(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::SetProgrammingLanguageDefault {
                    programming_language,
                } => {
                    info!(
                        "(engine) Sending set default programming language message: {:?}",
                        programming_language
                    );
                    self.to_ui
                        .send(Message::SetProgrammingLanguageDefault {
                            programming_language: *programming_language,
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::ShowLicense(license) => {
                    info!("(engine) Sending show license message");
                    self.to_ui
                        .send(Message::ShowLicense {
                            text: license.clone(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::Error(err) => {
                    info!("(engine) Sending error message to UI: {}", err);
                    self.to_ui
                        .send(Message::Error { error: err.clone() })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::Quit => {
                    info!("(engine) Sending quit message to UI");
                    self.to_ui
                        .send(Message::Quit)
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
            }
        }
        Ok(())
    }

    /// config the engine
    async fn config(&mut self, config: Box<dyn Config + Send + 'static>) -> Result<(), Error> {
        // invariant: the engine is in the Nil state
        if let Some(State::Nil) = self.state.last() {
            // remove the Nil state
            self.state.pop();
            // initialize the filesystem abstraction
            self.fs.set_data_dir(config.data_dir());
            self.fs.set_pwd(config.pwd());
            self.fs.set_spoken_language(config.spoken_language());
            self.fs
                .set_programming_language(config.programming_language());

            // Nil -> SelectWorkshop
            let workshops = self.fs.get_workshops_data_filtered()?;
            self.state.push(State::SelectWorkshop(workshops));
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "config".to_string(),
            ))
        }
    }

    /// Get the license text
    async fn get_license(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { .. }) = self.state.last() {
            // get the license text
            let license = self.fs.get_license(name).await?;
            // SelectWorkshop -> ShowLicense
            self.state.push(State::ShowLicense(license.clone()));

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "get_license".to_string(),
            ))
        }
    }

    /// change the spoken language
    async fn change_spoken_language(&mut self) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop
        if let Some(State::SelectWorkshop { .. }) = self.state.last() {
            // changing the spoken language while on the workshops screen will change the
            // workshops available to only those that support the spoken language so we compile a
            // list of all spoken languages supported by the currently installed workshops
            let spoken_languages = self.fs.get_workshops_spoken_languages()?;

            // SelectWorkshop or SelectLesson -> SelectSpokenLanguage
            self.state
                .push(State::SelectSpokenLanguage { spoken_languages });

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "change_spoken_language".to_string(),
            ))
        }
    }

    /// select the spoken language
    async fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        // invariant: the engine is in SelectSpokenLanguage state
        if let Some(State::SelectSpokenLanguage { .. }) = self.state.last() {
            // remove the SelectSpokenLanguage state
            self.state.pop();
            // set the spoken language
            self.fs.set_spoken_language(spoken_language);

            // need to refresh the list of workshops based off of the new spoken language
            if let Some(State::SelectWorkshop { .. }) = self.state.last() {
                // remove the old SelectWorkshop state
                self.state.pop();
                // get the new list of workshops
                let workshops = self.fs.get_workshops_data_filtered()?;
                // push the new SelectWorkshop state
                self.state.push(State::SelectWorkshop(workshops));
                // push the set default state
                self.state
                    .push(State::SetSpokenLanguageDefault { spoken_language });
            }

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "set_spoken_language".to_string(),
            ))
        }
    }

    /// change the programming language
    async fn change_programming_language(&mut self) -> Result<(), Error> {
        // invariant: the engine is in
        if let Some(State::SelectWorkshop { .. }) = self.state.last() {
            // changing the programming language while on the workshops screen will change the
            // workshops available to only those that support the programming language so we
            // compile a list of all programming languages supported by the currently installed
            // workshops
            let programming_languages = self.fs.get_workshops_programming_languages()?;

            // SelectWorkshop or SelectLesson -> SelectProgrammingLanguage
            self.state.push(State::SelectProgrammingLanguage {
                programming_languages,
            });

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "change_programming_language".to_string(),
            ))
        }
    }

    /// select the programming language
    async fn set_programming_language(
        &mut self,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        // invariant: the engine is in SelectProgrammingLanguage state
        if let Some(State::SelectProgrammingLanguage { .. }) = self.state.last() {
            // remove the SelectSpokenLanguage state
            self.state.pop();
            // set the spoken language
            self.fs.set_programming_language(programming_language);

            // need to refresh the list of workshops based off of the new spoken language
            if let Some(State::SelectWorkshop { .. }) = self.state.last() {
                // remove the old SelectWorkshop state
                self.state.pop();
                // get the new list of workshops
                let workshops = self.fs.get_workshops_data_filtered()?;
                // push the new SelectWorkshop state
                self.state.push(State::SelectWorkshop(workshops));
                // push the set default state
                self.state.push(State::SetProgrammingLanguageDefault {
                    programming_language,
                });
            }

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "set_programming_language".to_string(),
            ))
        }
    }

    /// set the workshop
    pub async fn set_workshop(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop state
        if let Some(State::SelectWorkshop { .. }) = self.state.last() {
            // set the workshop
            self.fs.set_workshop(Some(name.to_string()));
            // get the selected workshop
            let lessons = self.fs.get_lessons_data_filtered(name)?;

            // SelectWorkshop -> SelectLesson
            self.state.push(State::SelectLesson(lessons));

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "set_workshop".to_string(),
            ))
        }
    }

    /// set the lesson
    pub async fn set_lesson(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectLesson state
        if let Some(State::SelectLesson { .. }) = self.state.last() {
            // set the lesson
            self.fs.set_lesson(Some(name.to_string()));

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "set_lesson".to_string(),
            ))
        }
    }

    /// go back to the previous state
    pub async fn back(&mut self) -> Result<(), Error> {
        // invariant: the engine is in a state
        if self.state.len() > 1 {
            let prev = self.state.last().unwrap().to_string();
            // remove the last state
            self.state.pop();
            let cur = self.state.last().unwrap().to_string();
            info!("(engine) Going back from {} to {}", prev, cur);

            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "back".to_string(),
            ))
        }
    }

    /// quits the engine
    pub async fn quit(&mut self) -> Result<(), Error> {
        self.state.push(State::Quit);
        Ok(())
    }
}
