use crate::{Error, Workshop};
use languages::{programming, spoken};
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

/// The Filesystem wrapper
#[derive(Clone, Debug)]
pub struct Fs {
    /// The data directory
    pub data_dir: PathBuf,
    /// The present working directory
    pub pwd: PathBuf,
    /// The local workshops directory
    pub workshops: Option<PathBuf>,
    /// The selected spoken language
    pub spoken_language: Option<spoken::Code>,
    /// The selected programming language
    pub programming_language: Option<programming::Code>,
}

impl Fs {
    /// Creates a new instance of the filesystem wrapper
    pub fn new(data_dir: PathBuf, pwd: PathBuf) -> Self {
        Fs {
            data_dir,
            pwd,
            workshops: None,
            spoken_language: None,
            programming_language: None,
        }
    }

    /// Sets the local workshops directory
    pub fn set_workshops(&mut self, workshops: PathBuf) {
        self.workshops = Some(workshops);
    }

    /// Sets the selected spoken language
    pub fn set_spoken_language(&mut self, spoken_language: spoken::Code) {
        self.spoken_language = Some(spoken_language);
    }

    /// Gets the selected spoken language
    pub fn get_spoken_language(&self) -> spoken::Code {
        self.spoken_language.unwrap_or_default()
    }

    /// Sets the selected programming language
    pub fn set_programming_language(&mut self, programming_language: programming::Code) {
        self.programming_language = Some(programming_language);
    }

    /// Gets the selected programming language
    pub fn get_programming_language(&self) -> programming::Code {
        self.programming_language.unwrap_or_default()
    }

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

    /// Get a list of all workshops in the application data directory
    pub fn get_workshops(&self) -> Result<HashMap<String, Workshop>, Error> {
        let mut workshops = HashMap::new();

        // Read all directories in the data directory
        for entry in std::fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let dir_name = entry.file_name();
                let workshop_name = dir_name.to_string_lossy().to_string();
                let workshop = self.get_workshop(workshop_name.as_str())?;
                workshops.insert(workshop_name, workshop);
            }
        }

        Ok(workshops)
    }

    /// Get a specific workshop by name
    pub fn get_workshop(&self, name: &str) -> Result<Workshop, Error> {
        let workshop_path = self.data_dir.join(name).join("workshop.yaml");
        let workshop_content =
            read_to_string(workshop_path).map_err(|_| Error::WorkshopNotFound(name.to_string()))?;
        let mut workshop: Workshop = serde_yaml::from_str(&workshop_content)?;
        workshop.description = self.get_workshop_description(name, self.get_spoken_language())?;
        workshop.setup = self.get_workshop_setup(
            name,
            self.get_spoken_language(),
            self.get_programming_language(),
        )?;
        let license_path = self.data_dir.join(name).join("LICENSE");
        workshop.license_text = read_to_string(license_path)
            .map_err(|_| Error::WorkshopLicenseNotFound(name.to_string()))?;

        Ok(workshop)
    }

    /// Get the description of the workshop in the selected spoken language
    pub fn get_workshop_description(
        &self,
        name: &str,
        spoken: spoken::Code,
    ) -> Result<String, Error> {
        let mut description_path = self.data_dir.join(name);
        description_path = if description_path.join(spoken.to_string()).is_dir() {
            description_path
                .join(spoken.to_string())
                .join("workshop.md")
        } else if description_path.join(spoken::Code::en.to_string()).is_dir() {
            description_path
                .join(spoken::Code::en.to_string())
                .join("workshop.md")
        } else {
            return Err(Error::WorkshopDescriptionNotFound(
                name.to_string(),
                spoken.get_name_in_english().to_string(),
            ));
        };

        let description_content = read_to_string(description_path)
            .map_err(|_| Error::WorkshopNotFound(name.to_string()))?;
        Ok(description_content)
    }

    /// Get the setup of the workshop in the selected spoken and programming language
    pub fn get_workshop_setup(
        &self,
        name: &str,
        spoken: spoken::Code,
        programming: programming::Code,
    ) -> Result<String, Error> {
        let mut setup_path = self.data_dir.join(name);
        if setup_path.join(spoken.to_string()).is_dir() {
            setup_path = setup_path.join(spoken.to_string());
        } else if setup_path.join(spoken::Code::en.to_string()).is_dir() {
            setup_path = setup_path.join(spoken::Code::en.to_string());
        } else {
            return Err(Error::WorkshopSetupNotFound(
                name.to_string(),
                spoken.get_name_in_english().to_string(),
                programming.get_name().to_string(),
            ));
        }

        if setup_path.join(programming.to_string()).is_dir() {
            setup_path = setup_path.join(programming.to_string());
        } else if setup_path.join(programming::Code::rs.to_string()).is_dir() {
            setup_path = setup_path.join(programming::Code::rs.to_string());
        } else {
            return Err(Error::WorkshopSetupNotFound(
                name.to_string(),
                spoken.get_name_in_english().to_string(),
                programming.get_name().to_string(),
            ));
        }

        setup_path = setup_path.join("setup.md");

        let setup_content = read_to_string(setup_path).map_err(|_| {
            Error::WorkshopSetupNotFound(
                name.to_string(),
                spoken.get_name_in_english().to_string(),
                programming.get_name().to_string(),
            )
        })?;
        Ok(setup_content)
    }
}
