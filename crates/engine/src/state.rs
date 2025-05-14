use crate::{Error, Fs, Message};
use languages::{programming, spoken};
use std::path::Path;
use tokio::sync::mpsc::Sender;
use tracing::info;

/// the engine state
#[derive(Clone, Debug)]
pub enum State {
    /// uninitialized
    Uninitialized(Sender<Message>),
    /// select the workshop
    SelectWorkshop {
        /// channel to the UI
        to_ui: Sender<Message>,
        /// fs instance
        fs: Fs,
    },
    /// select the spoken language
    SelectSpokenLanguage {
        /// channel to the UI
        to_ui: Sender<Message>,
        /// fs instance
        fs: Fs,
        /// previous state
        previous_state: Box<State>,
    },
    /// select the programming language
    SelectProgrammingLanguage {
        /// channel to the UI
        to_ui: Sender<Message>,
        /// fs instance
        fs: Fs,
        /// previous state
        previous_state: Box<State>,
    },
    /// Error state
    Error {
        /// channel to the UI
        to_ui: Sender<Message>,
        /// fs instance
        fs: Fs,
        /// error message
        error: String,
    },
    /// quit
    Quit {
        /// channel to the UI
        to_ui: Sender<Message>,
        /// fs instance
        fs: Fs,
    },
}

impl State {
    fn get_sender(&self) -> Sender<Message> {
        match self {
            State::Uninitialized(to_ui) => to_ui.clone(),
            State::SelectSpokenLanguage { to_ui, .. } => to_ui.clone(),
            State::SelectProgrammingLanguage { to_ui, .. } => to_ui.clone(),
            State::SelectWorkshop { to_ui, .. } => to_ui.clone(),
            State::Error { to_ui, .. } => to_ui.clone(),
            State::Quit { to_ui, .. } => to_ui.clone(),
        }
    }

    fn get_fs(&self) -> Fs {
        match self {
            State::Uninitialized(_) => {
                Fs::new(Path::new("").to_path_buf(), Path::new("").to_path_buf())
            }
            State::SelectWorkshop { fs, .. } => fs.clone(),
            State::SelectSpokenLanguage { fs, .. } => fs.clone(),
            State::SelectProgrammingLanguage { fs, .. } => fs.clone(),
            State::Error { fs, .. } => fs.clone(),
            State::Quit { fs, .. } => fs.clone(),
        }
    }

    /// send the correct message to the UI to transition to the state
    pub async fn send_message(&self) -> Result<(), Error> {
        match self {
            State::SelectWorkshop { to_ui, fs, .. } => {
                let workshops = fs.get_workshops()?;
                info!("Sending {} workshops to UI", workshops.len());
                to_ui
                    .send(Message::SelectWorkshop { workshops })
                    .await
                    .map_err(|_| Error::UiChannelClosed)?;
                Ok(())
            }
            State::SelectSpokenLanguage { to_ui, .. } => to_ui
                .send(Message::SelectSpokenLanguage {
                    spoken_languages: spoken::Code::default().into_iter().collect(),
                })
                .await
                .map_err(|_| Error::UiChannelClosed),
            State::SelectProgrammingLanguage { to_ui, .. } => to_ui
                .send(Message::SelectProgrammingLanguage {
                    programming_languages: programming::Code::default().into_iter().collect(),
                })
                .await
                .map_err(|_| Error::UiChannelClosed),
            State::Error { to_ui, error, .. } => to_ui
                .send(Message::Error {
                    error: error.clone(),
                })
                .await
                .map_err(|_| Error::UiChannelClosed),
            _ => Ok(()),
        }
    }

    /// config the engine
    pub async fn config(
        &self,
        data_dir: &Path,
        pwd: &Path,
        spoken_language: spoken::Code,
        programming_language: programming::Code,
    ) -> Result<Self, Error> {
        let to_ui = self.get_sender();
        let mut fs = Fs::new(data_dir.to_path_buf(), pwd.to_path_buf());
        let next = if let Self::Uninitialized(..) = self {
            fs.set_spoken_language(spoken_language);
            fs.set_programming_language(programming_language);
            Self::SelectWorkshop { to_ui, fs }
        } else {
            Self::Error {
                to_ui,
                fs,
                error: format!("Invalid state change {} -> SelectWorkshop", self),
            }
        };
        next.send_message().await?;
        Ok(next)
    }

    /// change the spoken language
    pub async fn change_spoken_language(&self) -> Result<Self, Error> {
        let to_ui = self.get_sender();
        let fs = self.get_fs();
        let next = if let Self::SelectWorkshop { .. } = self {
            Self::SelectSpokenLanguage {
                to_ui,
                fs,
                previous_state: Box::new(self.clone()),
            }
        } else {
            Self::Error {
                to_ui,
                fs,
                error: format!("Invalid state change {} -> select_spoken_language", self),
            }
        };
        next.send_message().await?;
        Ok(next)
    }

    /// select the spoken language
    pub async fn set_spoken_language(&self, lang: spoken::Code) -> Result<Self, Error> {
        let to_ui = self.get_sender();
        let mut fs = self.get_fs();
        let next = if let Self::SelectSpokenLanguage { previous_state, .. } = self {
            fs.set_spoken_language(lang);
            *previous_state.clone()
        } else {
            Self::Error {
                to_ui,
                fs,
                error: format!("Invalid state change {} -> set_spoken_language", self),
            }
        };
        next.send_message().await?;
        Ok(next)
    }

    /// change the programming language
    pub async fn change_programming_language(&self) -> Result<Self, Error> {
        let to_ui = self.get_sender();
        let fs = self.get_fs();
        let next = if let Self::SelectWorkshop { .. } = self {
            Self::SelectProgrammingLanguage {
                to_ui,
                fs,
                previous_state: Box::new(self.clone()),
            }
        } else {
            Self::Error {
                to_ui,
                fs,
                error: format!(
                    "Invalid state change {} -> select_programming_language",
                    self
                ),
            }
        };
        next.send_message().await?;
        Ok(next)
    }

    /// select the programming language
    pub async fn set_programming_language(&self, lang: programming::Code) -> Result<Self, Error> {
        let to_ui = self.get_sender();
        let mut fs = self.get_fs();
        let next = if let Self::SelectProgrammingLanguage { previous_state, .. } = self {
            fs.set_programming_language(lang);
            *previous_state.clone()
        } else {
            Self::Error {
                to_ui,
                fs,
                error: format!("Invalid state change {} -> set_programming_language", self),
            }
        };
        next.send_message().await?;
        Ok(next)
    }

    /// quits the engine
    pub async fn quit(&self) -> Result<Self, Error> {
        Ok(State::Quit {
            to_ui: self.get_sender(),
            fs: self.get_fs(),
        })
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Uninitialized(_) => write!(f, "Uninitialized"),
            State::SelectWorkshop { .. } => write!(f, "SelectWorkshop"),
            State::SelectSpokenLanguage { .. } => write!(f, "SelectSpokenLanguage"),
            State::SelectProgrammingLanguage { .. } => write!(f, "SelectProgrammingLanguage"),
            State::Error { error, .. } => write!(f, "Error: {}", error),
            State::Quit { .. } => write!(f, "Quit"),
        }
    }
}
