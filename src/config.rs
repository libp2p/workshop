use crate::{
    languages::{programming, spoken},
    Error,
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

const DEFAULT_MAX_LOG_LINES: usize = 1000;

/// Represents the application configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    config_dir: PathBuf,
    data_dir: PathBuf,
    max_log_lines: Option<usize>,
    spoken_language: Option<spoken::Code>,
    programming_language: Option<programming::Code>,
    #[serde(skip)]
    pwd: PathBuf,
}

impl Config {
    /// Load the Config from a file, createing it if necessary
    pub fn load() -> Result<Self, Error> {
        // Get the application data directory
        let project_dirs = ProjectDirs::from("io", "libp2p", "workshop").ok_or_else(|| {
            Error::ProjectDirs("Could not determine project directories".to_string())
        })?;

        // create the data directory if needed
        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        // create the config directory if needed
        let config_dir = project_dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;

        // get the pwd
        let pwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Load the config from a file or create a new one
        let config_path = config_dir.join("config.yaml");
        if config_path.exists() {
            info!("Loading config from: {}", config_path.display());
            let mut config: Config = serde_yaml::from_reader(std::fs::File::open(&config_path)?)?;
            config.pwd = pwd.clone();
            Ok(config)
        } else {
            info!("Creating config at: {}", config_path.display());
            let config = Config {
                config_dir,
                data_dir,
                max_log_lines: Some(DEFAULT_MAX_LOG_LINES),
                pwd,
                spoken_language: None,
                programming_language: None,
            };
            config.save()?;
            Ok(config)
        }
    }

    /// Set the spoken language
    pub fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_language = spoken_language;
        self.save()?;
        Ok(())
    }

    /// Set the programming language
    pub fn set_programming_language(
        &mut self,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.programming_language = programming_language;
        self.save()?;
        Ok(())
    }

    /// Get the path to the application config directory
    pub fn config_dir(&self) -> &Path {
        self.config_dir.as_path()
    }

    /// Get the max log lines
    pub fn max_log_lines(&self) -> usize {
        match self.max_log_lines {
            Some(lines) => lines,
            None => DEFAULT_MAX_LOG_LINES,
        }
    }

    /// Save the config to a file
    pub fn save(&self) -> Result<(), Error> {
        let config_path = self.config_dir.join("config.yaml");
        serde_yaml::to_writer(std::fs::File::create(&config_path).unwrap(), &self).unwrap();
        info!("Config saved to: {}", config_path.display());
        Ok(())
    }

    /// Get the path to the application data directory
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }

    /// Get the present working directory
    pub fn pwd(&self) -> &Path {
        self.pwd.as_path()
    }

    /// Get the preferred spoken language
    pub fn spoken_language(&self) -> Option<spoken::Code> {
        self.spoken_language
    }

    /// Get the preferred programming language
    pub fn programming_language(&self) -> Option<programming::Code> {
        self.programming_language
    }
}
