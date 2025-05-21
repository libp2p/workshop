use crate::{
    workshop::{self, WorkshopData},
    Error, Lesson,
};
use languages::{programming, spoken};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::info;

/// The Filesystem wrapper
#[derive(Clone, Debug, Default)]
pub struct Fs {
    /// The data directory
    pub data_dir: PathBuf,
    /// The present working directory
    pub pwd: PathBuf,
    /// The local workshops directory
    //pub workshops_dir: Option<PathBuf>,
    /// The selected workshop
    pub workshop: Option<String>,
    /// The selected lesson
    pub lesson: Option<String>,
    /// The selected spoken language
    pub spoken_language: Option<spoken::Code>,
    /// The selected programming language
    pub programming_language: Option<programming::Code>,
}

impl Fs {
    /// Sets the data directory
    pub fn set_data_dir(&mut self, data_dir: &Path) {
        self.data_dir = data_dir.to_path_buf();
    }

    /// Sets the present working directory
    pub fn set_pwd(&mut self, pwd: &Path) {
        self.pwd = pwd.to_path_buf();
    }

    /*
    /// Sets the local workshops directory
    pub fn set_workshops_dir(&mut self, workshops_dir: PathBuf) {
        self.workshops_dir = Some(workshops_dir);
    }
    */

    /// Sets the selected spoken language
    pub fn set_spoken_language(&mut self, spoken_language: Option<spoken::Code>) {
        self.spoken_language = spoken_language;
    }

    /// Gets the selected spoken language
    pub fn get_spoken_language(&self) -> Option<spoken::Code> {
        self.spoken_language
    }

    /// Sets the selected programming language
    pub fn set_programming_language(&mut self, programming_language: Option<programming::Code>) {
        self.programming_language = programming_language;
    }

    /// Gets the selected programming language
    pub fn get_programming_language(&self) -> Option<programming::Code> {
        self.programming_language
    }

    /// Set the selected workshop
    pub fn set_workshop(&mut self, workshop: Option<String>) {
        self.workshop = workshop;
    }

    /// Set the selected lesson
    pub fn set_lesson(&mut self, lesson: Option<String>) {
        self.lesson = lesson;
    }

    /// Get the license text for a workshop
    pub async fn get_license(&self, name: &str) -> Result<String, Error> {
        let workshop = self.get_workshop_data(name)?;
        workshop.get_license().await
    }

    /*
    /// utility function searches for the ".workshops" directory
    fn find_workshops_dir(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();
        loop {
            let workshops_dir = current.join(".workshops");
            if workshops_dir.is_dir() {
                return Some(workshops_dir);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }
    */

    /// Get the complete list of all spoken languages supported by all installed workshops
    pub fn get_workshops_spoken_languages(&self) -> Result<Vec<spoken::Code>, Error> {
        let mut spoken_languages: Vec<spoken::Code> = self
            .get_workshops_data()?
            .values()
            .flat_map(|workshop| workshop.get_all_spoken_languages())
            .collect::<Vec<_>>();
        spoken_languages.sort();
        spoken_languages.dedup();
        Ok(spoken_languages)
    }

    /// Get the complete list of all programming languages supported by all installed workshops
    pub fn get_workshops_programming_languages(&self) -> Result<Vec<programming::Code>, Error> {
        let mut programming_languages: Vec<programming::Code> = self
            .get_workshops_data()?
            .values()
            .flat_map(|workshop| workshop.get_all_programming_languages())
            .collect::<Vec<_>>();
        programming_languages.sort();
        programming_languages.dedup();
        Ok(programming_languages)
    }

    /// Get a list of all workshops in the application data directory
    pub(crate) fn get_workshops_data(&self) -> Result<HashMap<String, WorkshopData>, Error> {
        Ok(std::fs::read_dir(&self.data_dir)
            .map_err(|_| Error::WorkshopDataDirNotFound)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() {
                        Some((name.clone(), self.get_workshop_data(&name).ok()?))
                    } else {
                        None
                    }
                })
            })
            .collect())
    }

    /// Get a specific workshop by name
    pub(crate) fn get_workshop_data(&self, name: &str) -> Result<WorkshopData, Error> {
        workshop::Loader::default()
            .name(name)
            .path(&self.data_dir)
            .try_load()
    }

    /// Get all workshops that support the given spoken and programming languages
    pub(crate) fn get_workshops_data_filtered(
        &self,
    ) -> Result<HashMap<String, WorkshopData>, Error> {
        Ok(std::fs::read_dir(&self.data_dir)
            .map_err(|_| Error::WorkshopDataDirNotFound)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() {
                        Some((name.clone(), self.get_workshop_data_filtered(&name).ok()?))
                    } else {
                        None
                    }
                })
            })
            .collect())
    }

    /// Get a workshop by name, iff it supports the given spoken and programming languages
    pub(crate) fn get_workshop_data_filtered(&self, name: &str) -> Result<WorkshopData, Error> {
        info!("(engine) get_workshop_data_filtered: {}", name);
        let workshop = self.get_workshop_data(name)?;
        if let Some(spoken) = self.spoken_language {
            info!("(engine) - spoken: {}", spoken.get_name_in_english());
            if !workshop.get_all_spoken_languages().contains(&spoken) {
                info!("(engine)   - not a supported spoken language");
                return Err(Error::WorkshopNotFound(name.to_string()));
            }
            info!("(engine)   - a supported spoken language");
            if let Some(programming) = self.programming_language {
                info!("(engine) - programming: {}", programming.get_name());
                if !workshop
                    .get_programming_languages_for_spoken_language(spoken)?
                    .contains(&programming)
                {
                    info!("(engine)   - not a supported programming language");
                    return Err(Error::WorkshopNotFound(name.to_string()));
                }
                info!("(engine)   - a supported programming language");
            } else {
                info!("(engine) - programming: Any");
            }
        } else {
            info!("(engine) - spoken: Any");
            if let Some(programming) = self.programming_language {
                info!("(engine) - programming: {}", programming.get_name());
                if !workshop
                    .get_all_programming_languages()
                    .contains(&programming)
                {
                    info!("(engine)   - not a supported programming language");
                    return Err(Error::WorkshopNotFound(name.to_string()));
                }
                info!("(engine)   - a supported programming language");
            }
        }
        Ok(workshop)
    }

    pub fn get_lessons_data_filtered(&self, _name: &str) -> Result<Vec<Lesson>, Error> {
        Ok(Vec::new())
    }
}
