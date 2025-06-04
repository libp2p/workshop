use crate::{
    fs,
    languages::{programming, spoken},
    Error,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Represents the application configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    spoken_language: Option<spoken::Code>,
    programming_language: Option<programming::Code>,
}

impl Config {
    /// Load the Config from a file, createing it if necessary
    pub fn load() -> Result<Self, Error> {
        // Load the config from a file or create a new one
        let config_path = fs::application::config_dir()?.join("config.yaml");
        if config_path.exists() {
            info!("Loading config from: {}", config_path.display());
            Ok(serde_yaml::from_reader(std::fs::File::open(&config_path)?)?)
        } else {
            info!("Creating config at: {}", config_path.display());
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save the config to a file
    pub fn save(&self) -> Result<(), Error> {
        let config_path = fs::application::config_dir()?.join("config.yaml");
        serde_yaml::to_writer(std::fs::File::create(&config_path).unwrap(), &self)?;
        info!("Config saved to: {}", config_path.display());
        Ok(())
    }

    /// Get the preferred spoken language
    pub fn spoken_language(&self) -> Option<spoken::Code> {
        self.spoken_language
    }

    /// Get the preferred programming language
    pub fn programming_language(&self) -> Option<programming::Code> {
        self.programming_language
    }

    /// Set the spoken language
    pub fn set_spoken_language(&mut self, spoken_language: Option<spoken::Code>) {
        self.spoken_language = spoken_language;
    }

    /// Set the programming language
    pub fn set_programming_language(&mut self, programming_language: Option<programming::Code>) {
        self.programming_language = programming_language;
    }
}
