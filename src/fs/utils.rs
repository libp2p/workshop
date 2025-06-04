use crate::{
    fs::Error as FsError,
    languages::{programming, spoken},
    models::workshop,
    Error,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

const APPLICATION_PARTS: [&str; 3] = ["io", "libp2p", "workshop"];

pub mod application {
    use super::*;

    /// Get the application data directory. This works on Windows, macOS, and Linux.
    pub fn data_dir() -> Result<PathBuf, Error> {
        let data_dir = directories::ProjectDirs::from(
            APPLICATION_PARTS[0],
            APPLICATION_PARTS[1],
            APPLICATION_PARTS[2],
        )
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or(FsError::ApplicationDirsNotFound)?;

        // create the data directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;

        Ok(data_dir)
    }

    /// Get the application config directory. This works on Windows, macOS, and Linux.
    pub fn config_dir() -> Result<PathBuf, Error> {
        let config_dir = directories::ProjectDirs::from(
            APPLICATION_PARTS[0],
            APPLICATION_PARTS[1],
            APPLICATION_PARTS[2],
        )
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or(FsError::ApplicationDirsNotFound)?;

        // create the config directory if it doesn't exist
        std::fs::create_dir_all(&config_dir)?;

        Ok(config_dir)
    }

    /// Get all of the workshops data objects for all workshops in the application data directory
    pub fn all_workshops() -> Result<HashMap<String, workshop::WorkshopData>, Error> {
        let data_dir = data_dir()?;
        workshops::load_workshop_data(data_dir)
    }

    /// Get all of the workshops in the application data directory, that support the given spoken
    /// and programming languages
    pub fn all_workshops_filtered(
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<HashMap<String, workshop::WorkshopData>, Error> {
        let workshops = all_workshops()?;
        Ok(workshops
            .into_iter()
            .filter(|(_, workshop_data)| {
                workshop_data.is_selected(spoken_language, programming_language)
            })
            .collect())
    }

    /// Get all of the spoken languages supported by all workshops in the application data
    /// directory
    pub fn all_spoken_languages() -> Result<Vec<spoken::Code>, Error> {
        let mut spoken_languages: Vec<spoken::Code> = all_workshops()?
            .values()
            .flat_map(|workshop| workshop.get_all_spoken_languages())
            .collect::<Vec<_>>();
        spoken_languages.sort();
        spoken_languages.dedup();
        Ok(spoken_languages)
    }

    /// Get all of the programming languages supported by all workshops in the application data
    /// directory
    pub fn all_programming_languages() -> Result<Vec<programming::Code>, Error> {
        let mut programming_languages: Vec<programming::Code> = all_workshops()?
            .values()
            .flat_map(|workshop| workshop.get_all_programming_languages())
            .collect::<Vec<_>>();
        programming_languages.sort();
        programming_languages.dedup();
        Ok(programming_languages)
    }

    /// Get all of the spoken to programming language mappings for all workshops in the application
    /// data directory
    pub fn get_all_languages() -> Result<HashMap<spoken::Code, Vec<programming::Code>>, Error> {
        let mut languages: HashMap<spoken::Code, Vec<programming::Code>> = HashMap::new();
        for workshop in all_workshops()?.values() {
            let workshop_languages = workshop.get_languages();
            for (spoken_lang, programming_langs) in workshop_languages {
                languages
                    .entry(*spoken_lang)
                    .and_modify(|langs| {
                        langs.extend(programming_langs.iter());
                        langs.sort();
                        langs.dedup();
                    })
                    .or_insert(programming_langs.clone());
            }
        }
        Ok(languages)
    }
}

pub mod workshops {
    use super::*;

    /// Initialize the present working directory (pwd) by creating a `.workshops` directory, if
    /// missing, and then recursively copying the selected workshop from the application data
    /// directory to the `.workshops` directory. Then return the path to the `.workshops`
    /// directory.
    pub fn init_data_dir(workshop: String) -> Result<PathBuf, Error> {
        // get the pwd
        let pwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let workshops_dir = pwd.join(".workshops");

        // Create the workshops directory if it doesn't exist
        std::fs::create_dir_all(&workshops_dir)?;

        // Copy the selected workshop to the workshops directory
        let data_dir = application::data_dir()?;
        let workshop_path = data_dir.join(&workshop);
        if workshop_path.exists() && workshop_path.is_dir() {
            let target_path = workshops_dir.join(&workshop);
            if !target_path.exists() {
                std::fs::copy(workshop_path, target_path)?;
            }
        } else {
            return Err(FsError::WorkshopDataDirNotFound.into());
        }

        Ok(workshops_dir)
    }

    /// Get the path to the `.workshops` directory by starting in the pwd and searching for the
    /// `.workshops` directory. Recursively search the parent directories until either the
    /// `.workshops` directory is found or the root directory is reached.
    pub fn data_dir() -> Option<PathBuf> {
        let mut current_dir = std::env::current_dir().ok()?;
        loop {
            let workshops_dir = current_dir.join(".workshops");
            if workshops_dir.exists() && workshops_dir.is_dir() {
                return Some(workshops_dir);
            }
            if !current_dir.pop() {
                break; // reached the root directory
            }
        }
        None
    }

    /// Get the given workshop in the `.workshops` directory, if it exists.
    pub fn load(workshop: String) -> Option<workshop::WorkshopData> {
        let workshops_dir = data_dir()?;
        let workshop_path = workshops_dir.join(&workshop);
        if workshop_path.exists() && workshop_path.is_dir() {
            return workshop::Loader::new(&workshop)
                .path(&workshops_dir)
                .try_load()
                .ok();
        }
        None
    }

    /// Get all workshop data objects for workshops in the given folder
    pub fn load_workshop_data<T: AsRef<Path>>(
        data_dir: T,
    ) -> Result<HashMap<String, workshop::WorkshopData>, Error> {
        let data_dir = data_dir.as_ref();
        if !data_dir.exists() || !data_dir.is_dir() {
            return Err(FsError::WorkshopDataDirNotFound.into());
        }

        let mut workshops = HashMap::new();
        for entry in std::fs::read_dir(data_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let workshop_name = entry.file_name().to_string_lossy().to_string();
                let workshop_data = workshop::Loader::new(&workshop_name)
                    .path(data_dir)
                    .try_load()?;
                workshops.insert(workshop_name, workshop_data);
            }
        }
        Ok(workshops)
    }
}
