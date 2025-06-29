use crate::{
    fs::{LazyLoader, TryLoad},
    languages::{programming, spoken},
    models::Error as ModelError,
    Error,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::trace;

/// Represents the status of a Lesson
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum Status {
    /// The lesson is not started
    #[default]
    NotStarted,
    /// The lesson is in progress
    InProgress,
    /// The lesson is completed
    Completed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::NotStarted => write!(f, "Not Started"),
            Status::InProgress => write!(f, "In Progress"),
            Status::Completed => write!(f, "Completed"),
        }
    }
}

/// Represents a workshop's metadata
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Lesson {
    pub title: String,
    pub description: String,
    pub status: Status,
}

#[async_trait::async_trait]
impl TryLoad for Lesson {
    type Error = Error;
    async fn try_load(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

pub type Metadata = Arc<RwLock<LazyLoader<Lesson>>>;
pub type LessonText = Arc<RwLock<LazyLoader<String>>>;

#[derive(Clone, Debug)]
pub struct LessonData {
    name: String,
    path: PathBuf,
    spoken_language: spoken::Code,
    programming_language: programming::Code,
    lesson_text: LessonText,
    metadata: Metadata,
}

impl LessonData {
    /// returns the name of the lesson
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// returns the path to the lesson data directory
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// returns the spoken language of the lesson
    pub fn get_spoken_language(&self) -> spoken::Code {
        self.spoken_language
    }

    /// returns the programming language of the lesson
    pub fn get_programming_language(&self) -> programming::Code {
        self.programming_language
    }

    /// returns the lesson text
    pub async fn get_text(&self) -> Result<String, Error> {
        let mut lesson_text = self
            .lesson_text
            .write() // get a write lock on the Arc<RwLock<LazyLoader<String>>>
            .await;
        // try to load the lesson text, if it fails, return the error
        lesson_text.try_load().await.cloned()
    }

    /// returns the metadata for the lesson
    pub async fn get_metadata(&self) -> Result<Lesson, Error> {
        let mut metadata = self
            .metadata
            .write() // get a write lock on the Arc<RwLock<LazyLoader<Workshop>>>
            .await;
        // try to load the metadata, if it fails, return the error
        metadata.try_load().await.cloned()
    }

    /// updates the lesson status and saves it to the lesson.yaml file
    pub async fn update_status(&self, new_status: Status) -> Result<(), Error> {
        let mut metadata = self.metadata.write().await;

        // Ensure metadata is loaded
        let mut lesson = metadata.try_load().await.cloned()?;
        lesson.status = new_status;

        // Save the updated metadata back to the file
        let lesson_yaml_path = self.path.join("lesson.yaml");
        let content = serde_yaml::to_string(&lesson)?;
        std::fs::write(&lesson_yaml_path, content)?;

        // Update the cached metadata
        *metadata = crate::fs::LazyLoader::Loaded(lesson);

        Ok(())
    }
}

#[async_trait::async_trait]
impl TryLoad for LessonData {
    type Error = Error;

    async fn try_load(path: &Path) -> Result<Self, Self::Error> {
        // try to get the spoken and programming languages from the path
        trace!(
            "Getting name, spoken, and programming from path: {}",
            path.display()
        );
        let (name, spoken_language, programming_language) = {
            let mut path = path.to_path_buf();
            let name = path
                .file_name()
                .and_then(|p| p.to_str())
                .ok_or::<Error>(ModelError::LessonDataDirNotFound.into())?
                .to_string();
            path.pop();
            trace!("Lesson name: {name}, rest: {}", path.display());
            let programming_language = path
                .file_name()
                .and_then(|p| programming::Code::try_from(p.to_string_lossy().as_ref()).ok())
                .ok_or::<Error>(ModelError::NoProgrammingLanguageSpecified.into())?;
            trace!("Programming language: {programming_language}");
            path.pop();
            let spoken_language = path
                .file_name()
                .and_then(|p| spoken::Code::try_from(p.to_string_lossy().as_ref()).ok())
                .ok_or::<Error>(ModelError::NoSpokenLanguageSpecified.into())?;
            trace!(
                "Spoken language: {spoken_language}, rest: {}",
                path.display()
            );
            (name, spoken_language, programming_language)
        };

        let loader = Loader::new(&name)
            .path(path)
            .spoken_language(spoken_language)
            .programming_language(programming_language);

        loader.try_load()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Loader {
    name: String,
    path: Option<PathBuf>,
    spoken_language: Option<spoken::Code>,
    programming_language: Option<programming::Code>,
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

    pub fn spoken_language(self, spoken_language: spoken::Code) -> Self {
        Self {
            spoken_language: Some(spoken_language),
            ..self
        }
    }

    pub fn programming_language(self, programming_language: programming::Code) -> Self {
        Self {
            programming_language: Some(programming_language),
            ..self
        }
    }

    fn try_load_lesson_text(&self, lesson_dir: &Path) -> Result<LessonText, Error> {
        let lesson_text_path = lesson_dir.join("lesson.md");
        if !lesson_text_path.exists() {
            return Err(ModelError::LessonTextFileMissing.into());
        }
        Ok(Arc::new(RwLock::new(LazyLoader::NotLoaded(
            lesson_text_path,
        ))))
    }

    fn try_load_metadata(&self, lesson_dir: &Path) -> Result<Metadata, Error> {
        let metadata_path = lesson_dir.join("lesson.yaml");
        if !metadata_path.exists() {
            return Err(ModelError::LessonMetadataFileMissing.into());
        }
        Ok(Arc::new(RwLock::new(LazyLoader::NotLoaded(metadata_path))))
    }

    pub fn try_load(&self) -> Result<LessonData, Error> {
        let name = self.name.clone();
        let path = self
            .path
            .clone()
            .ok_or::<Error>(ModelError::LessonDataDirNotFound.into())?;
        path.exists()
            .then_some(())
            .ok_or::<Error>(ModelError::LessonDataDirNotFound.into())?;
        let spoken_language = self
            .spoken_language
            .ok_or::<Error>(ModelError::NoSpokenLanguageSpecified.into())?;
        let programming_language = self
            .programming_language
            .ok_or::<Error>(ModelError::NoProgrammingLanguageSpecified.into())?;
        let lesson_text = self.try_load_lesson_text(&path)?;
        let metadata = self.try_load_metadata(&path)?;

        Ok(LessonData {
            name,
            path,
            spoken_language,
            programming_language,
            lesson_text,
            metadata,
        })
    }
}
