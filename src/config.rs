use crate::{
    fs,
    languages::{programming, spoken},
    Error,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Represents the application configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    python_minumum_version: String,
    python_executable: Option<String>,
    docker_compose_minimum_version: String,
    docker_compose_executable: Option<String>,
    spoken_language: Option<spoken::Code>,
    programming_language: Option<programming::Code>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            python_minumum_version: "3.10.0".to_string(),
            python_executable: None,
            docker_compose_minimum_version: "2.0.0".to_string(),
            docker_compose_executable: None,
            spoken_language: None,
            programming_language: None,
        }
    }
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

    /// Get the minimum required Python version
    pub fn python_minimum_version(&self) -> &str {
        &self.python_minumum_version
    }

    /// Get the preferred Python executable
    pub fn python_executable(&self) -> Option<String> {
        self.python_executable.clone()
    }

    /// Get the minimum required Docker Compose version
    pub fn docker_compose_minimum_version(&self) -> &str {
        &self.docker_compose_minimum_version
    }

    /// Get the preferred Docker Compose executable
    pub fn docker_compose_executable(&self) -> Option<String> {
        self.docker_compose_executable.clone()
    }

    /// Get the preferred spoken language
    pub fn spoken_language(&self) -> Option<spoken::Code> {
        self.spoken_language
    }

    /// Get the preferred programming language
    pub fn programming_language(&self) -> Option<programming::Code> {
        self.programming_language
    }

    /// Set the preferred Python executable
    pub fn set_python_executable(&mut self, python_executable: &str) {
        self.python_executable = Some(python_executable.to_string());
    }

    /// Set the preferred Docker Compose executable
    pub fn set_docker_compose_executable(&mut self, docker_compose_executable: &str) {
        self.docker_compose_executable = Some(docker_compose_executable.to_string());
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
