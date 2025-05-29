use crate::{Config, Error, Fs, Lesson, Message, State, Workshop};
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
    /// The selected workshop
    workshop: Option<String>,
    /// The selected lesson
    lesson: Option<String>,
    /// The selected spoken language
    spoken_language: Option<spoken::Code>,
    /// The selected programming language
    programming_language: Option<programming::Code>,
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
            workshop: None,
            lesson: None,
            spoken_language: None,
            programming_language: None,
        })
    }

    /// Runs the engine
    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            select! {
                // Process messages from the UI
                Some(msg) = self.from_ui.recv() => {
                    info!("(engine) received message: {}", msg);
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
                        Message::GetWorkshopDescription { name } => {
                            info!("(engine) Getting description for: {}", name);
                            self.get_description(&name).await
                        },
                        Message::GetWorkshopSetupInstructions { name } => {
                            info!("(engine) Getting setup instructions for: {}", name);
                            self.get_setup_instructions(&name).await
                        },
                        Message::GetWorkshopSpokenLanguages { name } => {
                            info!("(engine) Getting spoken languages for: {}", name);
                            self.get_spoken_languages(&name).await
                        },
                        Message::GetWorkshopProgrammingLanguages { name } => {
                            info!("(engine) Getting programming languages for: {}", name);
                            self.get_programming_languages(&name).await
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

    async fn send_message(&mut self) -> Result<(), Error> {
        info!(
            "(engine) send_message(), new state: {}",
            self.state
                .iter()
                .map(|s| format!("[{}]", s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if let Some(state) = self.state.last() {
            match state {
                State::Nil => {
                    info!("(engine) Tried sending message when in Nil");
                }
                State::SelectWorkshop { workshops_data } => {
                    info!(
                        "(engine) Sending select workshops with workshops: {:?}",
                        workshops_data.keys().collect::<Vec<_>>()
                    );
                    let mut workshops =
                        HashMap::<String, Workshop>::with_capacity(workshops_data.len());
                    /*
                    let mut descriptions =
                        HashMap::<String, String>::with_capacity(workshops_data.len());
                    let mut setup_instructions =
                        HashMap::<String, String>::with_capacity(workshops_data.len());
                    let mut spoken_languages =
                        HashMap::<String, Vec<spoken::Code>>::with_capacity(workshops_data.len());
                    let mut programming_languages =
                        HashMap::<String, Vec<programming::Code>>::with_capacity(
                            workshops_data.len(),
                        );
                    */
                    for (name, workshop_data) in workshops_data.iter() {
                        workshops.insert(
                            name.clone(),
                            workshop_data.get_metadata(self.spoken_language).await?,
                        );
                        /*
                        descriptions.insert(
                            name.clone(),
                            workshop_data.get_description(self.spoken_language).await?,
                        );
                        setup_instructions.insert(
                            name.clone(),
                            workshop_data
                                .get_setup_instructions(
                                    self.spoken_language,
                                    self.programming_language,
                                )
                                .await?,
                        );
                        spoken_languages
                            .insert(name.clone(), workshop_data.get_all_spoken_languages());
                        programming_languages
                            .insert(name.clone(), workshop_data.get_all_programming_languages());
                        */
                    }
                    self.to_ui
                        .send(Message::SelectWorkshop {
                            workshops,
                            /*
                            descriptions,
                            setup_instructions,
                            spoken_languages,
                            programming_languages,
                            */
                            spoken_language: self.spoken_language,
                            programming_language: self.programming_language,
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::ShowDescription { ref name, ref text } => {
                    info!("(engine) Sending show description message");
                    self.to_ui
                        .send(Message::ShowWorkshopDescription {
                            name: name.to_string(),
                            text: text.to_string(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;

                    // remove the ShowDescription state, go back to SelectWorkshop
                    self.state.pop();
                }
                State::ShowSetupInstructions { ref name, ref text } => {
                    info!("(engine) Sending show setup instructions message");
                    self.to_ui
                        .send(Message::ShowWorkshopSetupInstructions {
                            name: name.to_string(),
                            text: text.to_string(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;

                    // remove the ShowSetupInstructions state, go back to SelectWorkshop
                    self.state.pop();
                }
                State::ShowSpokenLanguages {
                    ref name,
                    ref spoken_languages,
                } => {
                    info!("(engine) Sending show spoken languages message");
                    self.to_ui
                        .send(Message::ShowWorkshopSpokenLanguages {
                            name: name.to_string(),
                            spoken_languages: spoken_languages.to_vec(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;

                    // remove the ShowSpokenLanguages state, go back to SelectWorkshop
                    self.state.pop();
                }
                State::ShowProgrammingLanguages {
                    ref name,
                    ref programming_languages,
                } => {
                    info!("(engine) Sending show programming languages message");
                    self.to_ui
                        .send(Message::ShowWorkshopProgrammingLanguages {
                            name: name.to_string(),
                            programming_languages: programming_languages.to_vec(),
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;

                    // remove the ShowProgrammingLanguages state, go back to SelectWorkshop
                    self.state.pop();
                }
                State::ShowLicense { license_text } => {
                    info!("(engine) Sending show license message");
                    self.to_ui
                        .send(Message::ShowLicense {
                            text: license_text.clone(),
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
                            spoken_language: self.spoken_language,
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
                            programming_language: self.programming_language,
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
                State::SelectLesson { lessons_data } => {
                    info!(
                        "(engine) Sending select lessons to {:?}",
                        lessons_data.keys().collect::<Vec<_>>()
                    );
                    let mut lessons = HashMap::<String, Lesson>::with_capacity(lessons_data.len());
                    let mut lesson_texts =
                        HashMap::<String, String>::with_capacity(lessons_data.len());
                    for (name, lesson_data) in lessons_data.iter() {
                        lessons.insert(name.clone(), lesson_data.get_metadata().await?);
                        lesson_texts.insert(name.clone(), lesson_data.get_lesson_text().await?);
                    }
                    self.to_ui
                        .send(Message::SelectLesson {
                            lessons,
                            lesson_texts,
                            spoken_language: self.spoken_language,
                            programming_language: self.programming_language,
                        })
                        .await
                        .map_err(|_| Error::UiChannelClosed)?;
                }
                State::CheckLesson => {
                    info!("(engine) CheckLesson");
                }
                State::LessonComplete => {
                    info!("(engine) LessonComplete");
                }
                State::LessonIncomplete => {
                    info!("(engine) LessonIncomplete");
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
            self.spoken_language = config.spoken_language();
            self.programming_language = config.programming_language();
            // Nil -> SelectWorkshop
            let workshops_data = self
                .fs
                .get_workshops_data()?
                .into_iter()
                .filter(|(_, workshop_data)| {
                    workshop_data.is_selected(self.spoken_language, self.programming_language)
                })
                .collect::<HashMap<_, _>>();
            self.state.push(State::SelectWorkshop { workshops_data });
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "config".to_string(),
            ))
        }
    }

    /// Get the description text
    async fn get_description(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // get the description text
            let text = workshop_data.get_description(self.spoken_language).await?;
            // SelectWorkshop -> ShowDescription
            self.state.push(State::ShowDescription {
                name: name.to_string(),
                text,
            });
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "get_license".to_string(),
            ))
        }
    }

    /// Get the setup instructions
    async fn get_setup_instructions(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // get the setup instructions text
            let text = workshop_data
                .get_setup_instructions(self.spoken_language, self.programming_language)
                .await?;
            // SelectWorkshop -> ShowSetupInstructions
            self.state.push(State::ShowSetupInstructions {
                name: name.to_string(),
                text,
            });
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "get_license".to_string(),
            ))
        }
    }

    /// Get the spoken languages
    async fn get_spoken_languages(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // get the spoken languages
            let spoken_languages = workshop_data.get_all_spoken_languages();
            // SelectWorkshop -> ShowSpokenLanguages
            self.state.push(State::ShowSpokenLanguages {
                name: name.to_string(),
                spoken_languages,
            });
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "get_license".to_string(),
            ))
        }
    }

    /// Get the programming languages
    async fn get_programming_languages(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // get the programming languages
            let programming_languages = workshop_data.get_all_programming_languages();
            // SelectWorkshop -> ShowProgrammingLanguages
            self.state.push(State::ShowProgrammingLanguages {
                name: name.to_string(),
                programming_languages,
            });
            Ok(())
        } else {
            Err(Error::InvalidStateChange(
                self.state.last().unwrap().to_string(),
                "get_license".to_string(),
            ))
        }
    }

    /// Get the license text
    async fn get_license(&mut self, name: &str) -> Result<(), Error> {
        // invariant: the engine is in SelectWorkshop State
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // get the license text
            let license_text = workshop_data.get_license().await?;
            // SelectWorkshop -> ShowLicense
            self.state.push(State::ShowLicense { license_text });
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
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            // this gathers all of the spoken languages of the workshops selected by the current
            // spoken and programming language setting
            let mut spoken_languages: Vec<spoken::Code> = workshops_data
                .values()
                .flat_map(|workshop_data| workshop_data.get_all_spoken_languages())
                .collect::<Vec<_>>();
            spoken_languages.sort();
            spoken_languages.dedup();
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
            self.spoken_language = spoken_language;
            // need to refresh the list of workshops based off of the new spoken language
            if let Some(State::SelectWorkshop { .. }) = self.state.last() {
                // remove the old SelectWorkshop state
                self.state.pop();
                // gather the set of workshops based on the new spoken language and programming
                // language settings
                let workshops_data = self
                    .fs
                    .get_workshops_data()?
                    .into_iter()
                    .filter(|(_, workshop_data)| {
                        workshop_data.is_selected(self.spoken_language, self.programming_language)
                    })
                    .collect::<HashMap<_, _>>();
                self.state.push(State::SelectWorkshop { workshops_data });
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
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            // this gathers the list of programming languages from the workshops selected by the
            // current spoken and programming language settings
            let mut programming_languages: Vec<programming::Code> = workshops_data
                .values()
                .flat_map(|workshop| workshop.get_all_programming_languages())
                .collect::<Vec<_>>();
            programming_languages.sort();
            programming_languages.dedup();
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
            self.programming_language = programming_language;

            // need to refresh the list of workshops based off of the new spoken language
            if let Some(State::SelectWorkshop { .. }) = self.state.last() {
                // remove the old SelectWorkshop state
                self.state.pop();
                let workshops_data = self
                    .fs
                    .get_workshops_data()?
                    .into_iter()
                    .filter(|(_, workshop_data)| {
                        workshop_data.is_selected(self.spoken_language, self.programming_language)
                    })
                    .collect::<HashMap<_, _>>();
                self.state.push(State::SelectWorkshop { workshops_data });
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
        if let Some(State::SelectWorkshop { workshops_data }) = self.state.last() {
            let workshop_data = workshops_data
                .get(name)
                .ok_or_else(|| Error::WorkshopNotFound(name.to_string()))?;
            // set the workshop
            self.workshop = Some(workshop_data.get_name().to_string());
            // get the selected workshop's lessons
            let lessons_data = workshop_data
                .get_lessons_data(self.spoken_language, self.programming_language)
                .await?;

            // SelectWorkshop -> SelectLesson
            self.state.push(State::SelectLesson { lessons_data });

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
            self.lesson = Some(name.to_string());
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
