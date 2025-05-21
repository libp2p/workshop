use crate::{Error, LazyLoader, TryLoad};
use languages::{programming, spoken};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;

/// Represents a workshop's metadata
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workshop {
    pub title: String,
    pub authors: Vec<String>,
    pub copyright: String,
    pub license: String,
    pub homepage: String,
    pub difficulty: String,
}

/// Represents the default spoken and programming language for a workshop
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Defaults {
    pub spoken_language: spoken::Code,
    pub programming_language: programming::Code,
}

#[async_trait::async_trait]
impl TryLoad for Workshop {
    type Error = Error;
    async fn try_load(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

#[async_trait::async_trait]
impl TryLoad for String {
    type Error = Error;
    async fn try_load(path: &Path) -> Result<Self, Error> {
        Ok(std::fs::read_to_string(path)?)
    }
}

pub(crate) type SetupInstructionsMap =
    HashMap<spoken::Code, HashMap<programming::Code, Arc<RwLock<LazyLoader<String>>>>>;
pub(crate) type DescriptionsMap = HashMap<spoken::Code, Arc<RwLock<LazyLoader<String>>>>;
pub(crate) type LicenseLoader = Arc<RwLock<LazyLoader<String>>>;
pub(crate) type MetadataMap = HashMap<spoken::Code, Arc<RwLock<LazyLoader<Workshop>>>>;

#[derive(Clone, Debug)]
pub(crate) struct WorkshopData {
    //pub name: String,
    //pub dir: PathBuf,
    defaults: Defaults,
    descriptions: DescriptionsMap,
    setup_instructions: SetupInstructionsMap,
    license: LicenseLoader,
    metadata: MetadataMap,
}

impl WorkshopData {
    /// returns the set of spoken languages the workshop has been translated to
    pub fn get_all_spoken_languages(&self) -> Vec<spoken::Code> {
        self.descriptions.keys().cloned().collect()
    }

    /// returns the set of programming languages the workshop has been ported to
    pub fn get_all_programming_languages(&self) -> Vec<programming::Code> {
        let mut programming_languages: Vec<programming::Code> = self
            .setup_instructions
            .values()
            .flat_map(|langs| langs.keys().cloned())
            .collect();
        programming_languages.sort();
        programming_languages.dedup();
        programming_languages
    }

    /// returns the set of programming languages given a spoken language
    pub fn get_programming_languages_for_spoken_language(
        &self,
        spoken_language: spoken::Code,
    ) -> Result<Vec<programming::Code>, Error> {
        self.setup_instructions
            .get(&spoken_language)
            .ok_or(Error::WorkshopSpokenLanguageNotFound(
                spoken_language.get_name_in_english().to_string(),
            ))
            .map(|langs| langs.keys().cloned().collect())
    }

    /*
    /// returns the set of spoken languages give a programming language
    pub fn get_spoken_languages_for_programming_language(
        &self,
        programming_language: programming::Code,
    ) -> Result<Vec<spoken::Code>, Error> {
        Ok(self
            .setup_instructions
            .iter()
            .filter_map(|(lang, langs)| {
                if langs.contains_key(&programming_language) {
                    Some(lang.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
    */

    /*
    /// returns the description for the workshop in the given spoken language
    pub async fn get_description(&self, spoken_language: spoken::Code) -> Result<String, Error> {
        let mut description = self
            .descriptions
            .get(&spoken_language)
            .ok_or(Error::WorkshopSpokenLanguageNotFound(
                spoken_language.get_name_in_english().to_string(),
            ))?
            .write()
            .await;
        description.try_load().await.map(|d| d.clone())
    }
    */

    /*
    /// returns the setup instructions for the workshop in the given spoken language and
    /// programming language
    pub async fn get_setup_instructions(
        &self,
        spoken_language: spoken::Code,
        programming_language: programming::Code,
    ) -> Result<String, Error> {
        let mut setup = self
            .setup_instructions
            .get(&spoken_language)
            .ok_or(Error::WorkshopSpokenLanguageNotFound(
                spoken_language.get_name_in_english().to_string(),
            ))?
            .get(&programming_language)
            .ok_or(Error::WorkshopProgrammingLanguageNotFound(
                programming_language.get_name().to_string(),
            ))?
            .write()
            .await;
        setup.try_load().await.map(|s| s.clone())
    }
    */

    /// returns the license text for the workshop
    pub async fn get_license(&self) -> Result<String, Error> {
        let mut license = self.license.write().await;
        license.try_load().await.cloned()
    }

    /// returns the metadata for the workshop in the given spoken language
    pub async fn get_metadata(
        &self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<Workshop, Error> {
        let spoken_language = spoken_language.unwrap_or(self.defaults.spoken_language);
        let mut metadata = self
            .metadata
            .get(&spoken_language)
            .ok_or(Error::WorkshopSpokenLanguageNotFound(
                spoken_language.get_name_in_english().to_string(),
            ))?
            .write()
            .await;
        metadata.try_load().await.cloned()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Loader {
    name: String,
    dir: Option<PathBuf>,
}

impl Loader {
    pub(crate) fn name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn path(self, path: &Path) -> Self {
        Self {
            dir: Some(path.to_path_buf()),
            ..self
        }
    }

    fn try_load_descriptions(self, workshop_dir: &Path) -> Result<DescriptionsMap, Error> {
        let descriptions = std::fs::read_dir(workshop_dir)
            .map_err(|_| Error::WorkshopDataDirNotFound)?
            .filter_map(|entry| {
                if let Ok(e) = entry {
                    if let Ok(code) =
                        spoken::Code::try_from(e.file_name().to_string_lossy().as_ref())
                    {
                        let description_path = e.path().join("description.md");
                        Some((
                            code,
                            Arc::new(RwLock::new(LazyLoader::NotLoaded(description_path))),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(descriptions)
    }

    fn try_load_setup_instructions(
        self,
        workshop_dir: &Path,
        spoken_languages: Vec<spoken::Code>,
    ) -> Result<SetupInstructionsMap, Error> {
        let mut setup_instructions: SetupInstructionsMap = SetupInstructionsMap::new();

        for spoken in spoken_languages {
            let programming_languages: HashMap<programming::Code, Arc<RwLock<LazyLoader<String>>>> =
                std::fs::read_dir(workshop_dir.join(spoken.to_string()))
                    .map_err(|_| {
                        Error::WorkshopDataSpokenDirNotFound(
                            spoken.get_name_in_english().to_string(),
                        )
                    })?
                    .filter_map(|entry| {
                        if let Ok(e) = entry {
                            let name = e.file_name().to_string_lossy().to_string();
                            if let Ok(code) = programming::Code::try_from(name.as_str()) {
                                let setup_path = e.path().join("setup.md");
                                Some((
                                    code,
                                    Arc::new(RwLock::new(LazyLoader::NotLoaded(setup_path))),
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

            setup_instructions.insert(spoken, programming_languages);
        }

        Ok(setup_instructions)
    }

    fn try_load_license(self, workshop_dir: &Path) -> Result<LicenseLoader, Error> {
        let license_path = workshop_dir.join("LICENSE");
        if !license_path.exists() {
            return Err(Error::WorkshopLicenseNotFound(self.name.clone()));
        }
        Ok(Arc::new(RwLock::new(LazyLoader::NotLoaded(license_path))))
    }

    fn try_load_defaults(self, workshop_dir: &Path) -> Result<Defaults, Error> {
        let defaults_path = workshop_dir.join("defaults.yaml");
        if !defaults_path.exists() {
            return Err(Error::WorkshopLicenseNotFound(self.name.clone()));
        }
        let defaults = std::fs::read_to_string(defaults_path)?;
        Ok(serde_yaml::from_str(&defaults)?)
    }

    fn try_load_metadata(self, workshop_dir: &Path) -> Result<MetadataMap, Error> {
        let metadata = std::fs::read_dir(workshop_dir)
            .map_err(|_| Error::WorkshopDataDirNotFound)?
            .filter_map(|entry| {
                if let Ok(e) = entry {
                    if let Ok(code) =
                        spoken::Code::try_from(e.file_name().to_string_lossy().as_ref())
                    {
                        let license_path = e.path().join("workshop.yaml");
                        Some((
                            code,
                            Arc::new(RwLock::new(LazyLoader::NotLoaded(license_path))),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        Ok(metadata)
    }

    pub(crate) fn try_load(&self) -> Result<WorkshopData, Error> {
        let name = self.name.clone();
        let dir = self.dir.clone().ok_or(Error::WorkshopDataDirNotFound)?;
        let workshop_path = dir.join(&name);
        if !workshop_path.exists() {
            return Err(Error::WorkshopNotFound(name));
        }

        let defaults = self.clone().try_load_defaults(&workshop_path)?;
        let descriptions = self.clone().try_load_descriptions(&workshop_path)?;
        let spoken_languages = descriptions.keys().cloned().collect::<Vec<_>>();
        let setup_instructions = self
            .clone()
            .try_load_setup_instructions(&workshop_path, spoken_languages)?;
        let license = self.clone().try_load_license(&workshop_path)?;
        let metadata = self.clone().try_load_metadata(&workshop_path)?;

        Ok(WorkshopData {
            //name,
            //dir,
            defaults,
            descriptions,
            setup_instructions,
            license,
            metadata,
        })
    }
}
