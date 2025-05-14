use crate::Error;
use directories::ProjectDirs;
use languages::{programming, spoken};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// Represents the application configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    config_dir: PathBuf,
    data_dir: PathBuf,
    spoken_language: spoken::Code,
    programming_language: programming::Code,
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

        // Load the config from a file or create a new one
        let config_path = config_dir.join("config.yaml");
        if config_path.exists() {
            info!("Loading config from: {}", config_path.display());
            let config: Config = serde_yaml::from_reader(std::fs::File::open(&config_path)?)?;
            info!("Preferred spoken language: {:?}", config.spoken_language);
            info!(
                "Preferred programming language: {:?}",
                config.programming_language
            );
            Ok(config)
        } else {
            let config = Config {
                config_dir,
                data_dir,
                spoken_language: spoken::Code::default(),
                programming_language: programming::Code::default(),
            };
            serde_yaml::to_writer(std::fs::File::create(&config_path)?, &config)?;
            Ok(config)
        }
    }

    /// Get the path to the application data directory
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }

    /// Get the path to the application config directory
    pub fn config_dir(&self) -> &Path {
        self.config_dir.as_path()
    }

    /// Get the preferred spoken language
    pub fn spoken_language(&self) -> spoken::Code {
        self.spoken_language
    }

    /// Get the preferred programming language
    pub fn programming_language(&self) -> programming::Code {
        self.programming_language
    }
}
