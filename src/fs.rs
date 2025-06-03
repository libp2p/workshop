pub mod error;
pub use error::Error;

pub mod lazy_loader;
pub use lazy_loader::{LazyLoader, TryLoad};

use crate::{
    languages::{programming, spoken},
    models::workshop,
    Error as WorkshopError,
};
use std::{collections::HashMap, path::Path};

/// Get the complete list of all spoken languages supported by all installed workshops
pub fn get_workshops_spoken_languages<T: AsRef<Path>>(
    data_dir: T,
) -> Result<Vec<spoken::Code>, WorkshopError> {
    let mut spoken_languages: Vec<spoken::Code> = get_workshops_data(data_dir)?
        .values()
        .flat_map(|workshop| workshop.get_all_spoken_languages())
        .collect::<Vec<_>>();
    spoken_languages.sort();
    spoken_languages.dedup();
    Ok(spoken_languages)
}

/// Get the complete list of all programming languages supported by all installed workshops
pub fn get_workshops_programming_languages<T: AsRef<Path>>(
    data_dir: T,
) -> Result<Vec<programming::Code>, WorkshopError> {
    let mut programming_languages: Vec<programming::Code> = get_workshops_data(data_dir)?
        .values()
        .flat_map(|workshop| workshop.get_all_programming_languages())
        .collect::<Vec<_>>();
    programming_languages.sort();
    programming_languages.dedup();
    Ok(programming_languages)
}

/// Get a list of all workshops in the application data directory
pub fn get_workshops_data<T: AsRef<Path>>(
    data_dir: T,
) -> Result<HashMap<String, workshop::WorkshopData>, WorkshopError> {
    Ok(std::fs::read_dir(data_dir.as_ref())
        .map_err(|_| Error::WorkshopDataDirNotFound)?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if e.path().is_dir() {
                    let workshop_data = workshop::Loader::new(&name)
                        .path(data_dir.as_ref())
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
