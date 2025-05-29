use crate::{
    workshop::{self, WorkshopData},
    Error,
};
use languages::{programming, spoken};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// The Filesystem wrapper
#[derive(Clone, Debug, Default)]
pub struct Fs {
    /// The data directory
    pub data_dir: PathBuf,
    /// The present working directory
    pub pwd: PathBuf,
    // The local workshops directory
    //pub workshops_dir: Option<PathBuf>,
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
                        let workshop_data = workshop::Loader::new(&name)
                            .path(&self.data_dir)
                            .try_load()
                            .ok()?;
                        Some((name.clone(), workshop_data))
                    } else {
                        None
                    }
                })
            })
            .collect())
    }
}
