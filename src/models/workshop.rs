use crate::{
    fs::{Error as FsError, LazyLoader, TryLoad},
    languages::{programming, spoken},
    models::{Error as ModelError, LessonData},
    Error,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::info;

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

pub type SetupInstructionsMap =
    HashMap<spoken::Code, HashMap<programming::Code, Arc<RwLock<LazyLoader<String>>>>>;
pub type DescriptionsMap = HashMap<spoken::Code, Arc<RwLock<LazyLoader<String>>>>;
pub type LicenseLoader = Arc<RwLock<LazyLoader<String>>>;
pub type MetadataMap = HashMap<spoken::Code, Arc<RwLock<LazyLoader<Workshop>>>>;
pub type LessonsDataMap =
    HashMap<spoken::Code, HashMap<programming::Code, Vec<Arc<RwLock<LazyLoader<LessonData>>>>>>;

#[derive(Clone, Debug)]
pub struct WorkshopData {
    name: String,
    path: PathBuf,
    defaults: Defaults,
    descriptions: DescriptionsMap,
    setup_instructions: SetupInstructionsMap,
    license: LicenseLoader,
    metadata: MetadataMap,
    lessons_data: LessonsDataMap,
    languages: HashMap<spoken::Code, Vec<programming::Code>>,
}

impl WorkshopData {
    /// returns the workshop name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// returns the path to the workshop root directory
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// returns the default languages for this workshop
    pub fn get_defaults(&self) -> &Defaults {
        &self.defaults
    }

    /// returns the set of spoken languages the workshop has been translated to
    pub fn get_all_spoken_languages(&self) -> Vec<spoken::Code> {
        self.languages.keys().cloned().collect::<Vec<_>>()
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

    /// returns the mapping of spoken languages to programming languages
    pub fn get_languages(&self) -> &HashMap<spoken::Code, Vec<programming::Code>> {
        &self.languages
    }

    /// returns the set of programming languages given a spoken language
    pub fn get_programming_languages_for_spoken_language(
        &self,
        spoken_language: spoken::Code,
    ) -> Vec<programming::Code> {
        match self.setup_instructions.get(&spoken_language) {
            Some(langs) => langs.keys().cloned().collect(),
            None => Vec::new(), // return an empty vector if the spoken language is not found
        }
    }

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
                    Some(*lang)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }

    /// test if this workshop is selected with the given spoken and programming language
    pub fn is_selected(
        &self,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> bool {
        let name = self.get_name();
        info!("(engine) WorkshopData::is_selected: {}", name);
        if let Some(spoken) = spoken_language {
            info!("(engine) - spoken: {}", spoken.get_name_in_english());
            if !self.get_all_spoken_languages().contains(&spoken) {
                info!("(engine)   - not a supported spoken language");
                return false;
            }
            info!("(engine)   - a supported spoken language");
            if let Some(programming) = programming_language {
                info!("(engine) - programming: {}", programming.get_name());
                if !self
                    .get_programming_languages_for_spoken_language(spoken)
                    .contains(&programming)
                {
                    info!("(engine)   - not a supported programming language");
                    return false;
                }
                info!("(engine)   - a supported programming language");
            } else {
                info!("(engine) - programming: Any");
            }
        } else {
            info!("(engine) - spoken: Any");
            if let Some(programming) = programming_language {
                info!("(engine) - programming: {}", programming.get_name());
                if !self.get_all_programming_languages().contains(&programming) {
                    info!("(engine)   - not a supported programming language");
                    return false;
                }
                info!("(engine)   - a supported programming language");
            }
        }
        true
    }

    /// returns the description for the workshop in the given spoken language
    pub async fn get_description(
        &self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<String, Error> {
        info!(
            "(engine) WorkshopData::get_description({})",
            spoken_language.map_or("Any".to_string(), |s| s.get_name_in_english().to_string())
        );

        if self.descriptions.is_empty() {
            return Err(ModelError::WorkshopNoDescriptions.into());
        }

        let spoken_language = {
            let spoken = spoken_language.unwrap_or(self.defaults.spoken_language);
            if self.setup_instructions.contains_key(&spoken) {
                spoken
            } else {
                *self.setup_instructions.keys().next().ok_or::<Error>(
                    ModelError::WorkshopSpokenLanguageNotFound(
                        spoken.get_name_in_english().to_string(),
                    )
                    .into(),
                )?
            }
        };

        info!(
            "(engine) WorkshopData::get_description: {}",
            spoken_language
        );
        let mut description = self
            .descriptions
            .get(&spoken_language)
            .ok_or::<Error>(
                ModelError::WorkshopSpokenLanguageNotFound(
                    spoken_language.get_name_in_english().to_string(),
                )
                .into(),
            )?
            .write() // get a write lock on the Arc<RwLock<LazyLoader<String>>>
            .await;
        // try to load the description, if it fails, return the error
        description.try_load().await.cloned()
    }

    /// returns the setup instructions for the workshop in the given spoken language and
    /// programming language
    pub async fn get_setup_instructions(
        &self,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<String, Error> {
        info!(
            "(engine) WorkshopData::get_setup_instructions({}, {})",
            spoken_language.map_or("Any".to_string(), |s| s.get_name_in_english().to_string()),
            programming_language.map_or("Any".to_string(), |p| p.get_name().to_string())
        );

        if self.setup_instructions.is_empty() {
            return Err(ModelError::WorkshopNoSetupInstructions.into());
        }

        let spoken_language = {
            let spoken = spoken_language.unwrap_or(self.defaults.spoken_language);
            if self.setup_instructions.contains_key(&spoken) {
                spoken
            } else {
                *self.setup_instructions.keys().next().ok_or::<Error>(
                    ModelError::WorkshopSpokenLanguageNotFound(
                        spoken.get_name_in_english().to_string(),
                    )
                    .into(),
                )?
            }
        };

        let mut setup = {
            let spoken = self
                .setup_instructions
                .get(&spoken_language)
                .ok_or::<Error>(
                    ModelError::WorkshopSpokenLanguageNotFound(
                        spoken_language.get_name_in_english().to_string(),
                    )
                    .into(),
                )?;

            if spoken.is_empty() {
                return Err(ModelError::WorkshopNoProgrammingLanguagesForSpokenLanguage(
                    spoken_language.get_name_in_english().to_string(),
                )
                .into());
            }

            let programming_language = {
                let programming =
                    programming_language.unwrap_or(self.defaults.programming_language);
                if spoken.contains_key(&programming) {
                    programming
                } else {
                    *spoken.keys().next().ok_or::<Error>(
                        ModelError::WorkshopProgrammingLanguageNotFound(
                            programming.get_name().to_string(),
                        )
                        .into(),
                    )?
                }
            };

            info!(
                "(engine) WorkshopData::get_setup_instructions: {} + {}",
                spoken_language, programming_language
            );

            spoken.get(&programming_language).ok_or::<Error>(
                ModelError::WorkshopProgrammingLanguageNotFound(
                    programming_language.get_name().to_string(),
                )
                .into(),
            )?
        }
        .write()
        .await;

        // try to load the setup instructions, if it fails, return the error
        setup.try_load().await.cloned()
    }

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
        info!(
            "(engine) WorkshopData::get_metadata({})",
            spoken_language.map_or("Any".to_string(), |s| s.get_name_in_english().to_string())
        );
        if self.metadata.is_empty() {
            return Err(ModelError::WorkshopNoMetadata.into());
        }

        let spoken_language = {
            let spoken = spoken_language.unwrap_or(self.defaults.spoken_language);
            if self.metadata.contains_key(&spoken) {
                spoken
            } else {
                *self.metadata.keys().next().ok_or::<Error>(
                    ModelError::WorkshopSpokenLanguageNotFound(
                        spoken.get_name_in_english().to_string(),
                    )
                    .into(),
                )?
            }
        };

        let mut metadata = self
            .metadata
            .get(&spoken_language)
            .ok_or::<Error>(
                ModelError::WorkshopSpokenLanguageNotFound(
                    spoken_language.get_name_in_english().to_string(),
                )
                .into(),
            )?
            .write() // get a write lock on the Arc<RwLock<LazyLoader<Workshop>>>
            .await;
        // try to load the metadata, if it fails, return the error
        metadata.try_load().await.cloned()
    }

    /// returns the list of LessonData structs for the given spoken and programming language
    pub async fn get_lessons_data(
        &self,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<HashMap<String, LessonData>, Error> {
        info!(
            "(engine) WorkshopData::get_lessons_data({}, {})",
            spoken_language.map_or("Any".to_string(), |s| s.get_name_in_english().to_string()),
            programming_language.map_or("Any".to_string(), |p| p.get_name().to_string())
        );

        if self.lessons_data.is_empty() {
            return Err(ModelError::WorkshopNoLessonsData.into());
        }

        let spoken_language = {
            let spoken = spoken_language.unwrap_or(self.defaults.spoken_language);
            if self.lessons_data.contains_key(&spoken) {
                spoken
            } else {
                *self.lessons_data.keys().next().ok_or::<Error>(
                    ModelError::WorkshopSpokenLanguageNotFound(
                        spoken.get_name_in_english().to_string(),
                    )
                    .into(),
                )?
            }
        };

        let lessons = {
            let spoken = self.lessons_data.get(&spoken_language).ok_or::<Error>(
                ModelError::WorkshopSpokenLanguageNotFound(
                    spoken_language.get_name_in_english().to_string(),
                )
                .into(),
            )?;

            if spoken.is_empty() {
                return Err(ModelError::WorkshopNoProgrammingLanguagesForSpokenLanguage(
                    spoken_language.get_name_in_english().to_string(),
                )
                .into());
            }

            let programming_language = {
                let programming =
                    programming_language.unwrap_or(self.defaults.programming_language);
                if spoken.contains_key(&programming) {
                    programming
                } else {
                    *spoken.keys().next().ok_or::<Error>(
                        ModelError::WorkshopProgrammingLanguageNotFound(
                            programming.get_name().to_string(),
                        )
                        .into(),
                    )?
                }
            };

            info!(
                "(engine) WorkshopData::get_lessons_data: {} + {}",
                spoken_language, programming_language
            );

            spoken.get(&programming_language).ok_or::<Error>(
                ModelError::WorkshopProgrammingLanguageNotFound(
                    programming_language.get_name().to_string(),
                )
                .into(),
            )?
        };

        let mut lessons_data: HashMap<String, LessonData> = HashMap::new();
        for lesson in lessons.iter() {
            info!("(engine) Loading lesson data: {:?}", lesson);
            let lesson_data = lesson.write().await.try_load().await.cloned()?;
            lessons_data.insert(lesson_data.get_name().to_string(), lesson_data);
        }
        Ok(lessons_data)
    }

    /// Calculate the path to the deps.py script using status languages or defaults
    pub fn get_deps_script_path(
        &self,
        status_spoken: Option<spoken::Code>,
        status_programming: Option<programming::Code>,
    ) -> Result<PathBuf, Error> {
        // Use status languages or fall back to defaults
        let spoken = status_spoken.unwrap_or(self.defaults.spoken_language);
        let programming = status_programming.unwrap_or(self.defaults.programming_language);

        // Construct path: {workshop_data_dir}/{workshop_name}/{spoken}/{programming}/deps.py
        let data_dir =
            crate::fs::workshops::data_dir().ok_or_else(|| ModelError::WorkshopDataDirNotFound)?;

        Ok(data_dir
            .join(&self.name)
            .join(spoken.to_string())
            .join(programming.to_string())
            .join("deps.py"))
    }

    /// Calculate the path to the check.py script for a specific lesson using status languages or defaults
    pub fn get_check_script_path(
        &self,
        lesson_name: &str,
        status_spoken: Option<spoken::Code>,
        status_programming: Option<programming::Code>,
    ) -> Result<PathBuf, Error> {
        // Use status languages or fall back to defaults
        let spoken = status_spoken.unwrap_or(self.defaults.spoken_language);
        let programming = status_programming.unwrap_or(self.defaults.programming_language);

        // Construct path: {workshop_data_dir}/{workshop_name}/{spoken}/{programming}/{lesson}/check.py
        let data_dir =
            crate::fs::workshops::data_dir().ok_or_else(|| ModelError::WorkshopDataDirNotFound)?;

        Ok(data_dir
            .join(&self.name)
            .join(spoken.to_string())
            .join(programming.to_string())
            .join(lesson_name)
            .join("check.py"))
    }

    /// Calculate the directory path for a specific lesson using status languages or defaults
    pub fn get_lesson_dir_path(
        &self,
        lesson_name: &str,
        status_spoken: Option<spoken::Code>,
        status_programming: Option<programming::Code>,
    ) -> Result<PathBuf, Error> {
        // Use status languages or fall back to defaults
        let spoken = status_spoken.unwrap_or(self.defaults.spoken_language);
        let programming = status_programming.unwrap_or(self.defaults.programming_language);

        // Construct path: {workshop_data_dir}/{workshop_name}/{spoken}/{programming}/{lesson}/
        let data_dir =
            crate::fs::workshops::data_dir().ok_or_else(|| ModelError::WorkshopDataDirNotFound)?;

        Ok(data_dir
            .join(&self.name)
            .join(spoken.to_string())
            .join(programming.to_string())
            .join(lesson_name))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Loader {
    name: String,
    path: Option<PathBuf>,
}

impl Loader {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn path(self, path: &Path) -> Self {
        Self {
            path: Some(path.to_path_buf()),
            ..self
        }
    }

    fn try_load_descriptions(&self, workshop_dir: &Path) -> Result<DescriptionsMap, Error> {
        let descriptions = std::fs::read_dir(workshop_dir)
            .map_err(|_| FsError::WorkshopDataDirNotFound)?
            .filter_map(|entry| {
                if let Ok(e) = entry {
                    if let Ok(code) =
                        spoken::Code::try_from(e.file_name().to_string_lossy().as_ref())
                    {
                        info!(
                            "(engine) Found setup description under {}: {}",
                            workshop_dir.display(),
                            code
                        );
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
        &self,
        workshop_dir: &Path,
        spoken_languages: &Vec<spoken::Code>,
    ) -> Result<SetupInstructionsMap, Error> {
        let mut setup_instructions: SetupInstructionsMap = SetupInstructionsMap::new();

        for spoken in spoken_languages {
            let programming_languages: HashMap<programming::Code, Arc<RwLock<LazyLoader<String>>>> =
                std::fs::read_dir(workshop_dir.join(spoken.to_string()))
                    .map_err(|_| {
                        ModelError::WorkshopDataSpokenDirNotFound(
                            spoken.get_name_in_english().to_string(),
                        )
                    })?
                    .filter_map(|entry| {
                        if let Ok(e) = entry {
                            let name = e.file_name().to_string_lossy().to_string();
                            if let Ok(code) = programming::Code::try_from(name.as_str()) {
                                info!(
                                    "(engine) Found setup instructions under {}: {} + {}",
                                    workshop_dir.display(),
                                    spoken,
                                    code
                                );
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

            setup_instructions.insert(*spoken, programming_languages);
        }

        Ok(setup_instructions)
    }

    fn try_load_license(&self, workshop_dir: &Path) -> Result<LicenseLoader, Error> {
        let license_path = workshop_dir.join("LICENSE");
        if !license_path.exists() {
            return Err(ModelError::WorkshopLicenseNotFound(self.name.clone()).into());
        }
        Ok(Arc::new(RwLock::new(LazyLoader::NotLoaded(license_path))))
    }

    fn try_load_defaults(&self, workshop_dir: &Path) -> Result<Defaults, Error> {
        let defaults_path = workshop_dir.join("defaults.yaml");
        if !defaults_path.exists() {
            return Err(ModelError::WorkshopDefaultsNotFound(self.name.clone()).into());
        }
        let defaults = std::fs::read_to_string(defaults_path)?;
        Ok(serde_yaml::from_str(&defaults)?)
    }

    fn try_load_metadata(&self, workshop_dir: &Path) -> Result<MetadataMap, Error> {
        let metadata = std::fs::read_dir(workshop_dir)
            .map_err(|_| FsError::WorkshopDataDirNotFound)?
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

    fn try_load_lessons_data(
        &self,
        workshop_dir: &Path,
        spoken_languages: &Vec<spoken::Code>,
    ) -> Result<LessonsDataMap, Error> {
        info!(
            "(engine) WorkshopData::try_load_lessons_data: {}, {:?}",
            workshop_dir.display(),
            spoken_languages
        );
        let mut lessons_data: LessonsDataMap = LessonsDataMap::new();

        for spoken in spoken_languages {
            let programming_languages: HashMap<
                programming::Code,
                Vec<Arc<RwLock<LazyLoader<LessonData>>>>,
            > = std::fs::read_dir(workshop_dir.join(spoken.to_string()))
                .map_err(|_| {
                    ModelError::WorkshopDataSpokenDirNotFound(
                        spoken.get_name_in_english().to_string(),
                    )
                })?
                .filter_map(|entry| {
                    if let Ok(e) = entry {
                        let name = e.file_name().to_string_lossy().to_string();
                        if let Ok(code) = programming::Code::try_from(name.as_str()) {
                            // create a Vec of lazy loaders for each lesson
                            let lessons_data: Vec<Arc<RwLock<LazyLoader<LessonData>>>> =
                                std::fs::read_dir(e.path())
                                    .map_err(|_| {
                                        ModelError::WorkshopDataProgrammingDirNotFound(
                                            code.get_name().to_string(),
                                        )
                                    })
                                    .ok()?
                                    .filter_map(|entry| {
                                        if let Ok(e) = entry {
                                            if e.path().is_dir() {
                                                info!(
                                                    "(engine) Found lesson data under {}: {} + {}: {}",
                                                    workshop_dir.display(),
                                                    spoken,
                                                    code,
                                                    e.path().display()
                                                );
                                                return Some(Arc::new(RwLock::new(
                                                    LazyLoader::NotLoaded(e.path()),
                                                )));
                                            }
                                        }
                                        None
                                    })
                                    .collect();
                            Some((code, lessons_data))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            lessons_data.insert(*spoken, programming_languages);
        }

        Ok(lessons_data)
    }

    pub fn try_load(&self) -> Result<WorkshopData, Error> {
        let name = self.name.clone();
        let path = self
            .path
            .clone()
            .ok_or::<Error>(FsError::WorkshopDataDirNotFound.into())?;
        let workshop_path = path.join(&name);
        workshop_path
            .exists()
            .then_some(())
            .ok_or::<Error>(ModelError::WorkshopNotFound(name.clone()).into())?;

        let defaults = self.try_load_defaults(&workshop_path)?;
        let descriptions = self.try_load_descriptions(&workshop_path)?;
        let mut spoken_languages = descriptions.keys().cloned().collect::<Vec<_>>();
        spoken_languages.sort();
        let setup_instructions =
            self.try_load_setup_instructions(&workshop_path, &spoken_languages)?;
        let languages = setup_instructions
            .iter()
            .map(|(spoken, langs)| {
                (
                    *spoken,
                    langs.keys().cloned().collect::<Vec<programming::Code>>(),
                )
            })
            .collect::<HashMap<spoken::Code, Vec<programming::Code>>>();
        let license = self.try_load_license(&workshop_path)?;
        let metadata = self.try_load_metadata(&workshop_path)?;
        let lessons_data = self.try_load_lessons_data(&workshop_path, &spoken_languages)?;

        Ok(WorkshopData {
            name,
            path,
            defaults,
            descriptions,
            setup_instructions,
            license,
            metadata,
            lessons_data,
            languages,
        })
    }
}
