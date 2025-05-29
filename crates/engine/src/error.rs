/// Errors generated from this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlParsing(#[from] serde_yaml::Error),

    #[error("Invalid engine state change {0} -> {1}")]
    InvalidStateChange(String, String),

    #[error("Workshop not found: {0}")]
    WorkshopNotFound(String),

    #[error("Workshop name not found")]
    WorkshopNameNotFound,

    #[error("Workshop data directory not found")]
    WorkshopDataDirNotFound,

    #[error("Workshop data directory for spoken langage {0} not found")]
    WorkshopDataSpokenDirNotFound(String),

    #[error("Workshop spoken language not found: {0}")]
    WorkshopSpokenLanguageNotFound(String),

    #[error("Workshop data directory for programming language {0} not found")]
    WorkshopDataProgrammingDirNotFound(String),

    #[error("Workshop programming language not found: {0}")]
    WorkshopProgrammingLanguageNotFound(String),

    #[error("Workshop metadata not found: {0}")]
    WorkshopMetadataNotFound(String),

    #[error("Workshop license not found: {0}")]
    WorkshopLicenseNotFound(String),

    #[error("Workshop setup not found: {0} ({1}, {2})")]
    WorkshopSetupNotFound(String, String, String),

    #[error("Workshop description not found: {0} ({1})")]
    WorkshopDescriptionNotFound(String, String),

    #[error("Lesson not found: {0}")]
    LessonNotFound(String),

    #[error("Lesson data directory not found")]
    LessonDataDirNotFound,

    #[error("Lesson text file missing")]
    LessonTextFileMissing,

    #[error("Lesson metadata file missing")]
    LessonMetadataFileMissing,

    #[error("No spoken language specified")]
    NoSpokenLanguageSpecified,

    #[error("No programming language specified")]
    NoProgrammingLanguageSpecified,

    #[error("Failed to run dependency check: {0}")]
    DependencyCheckFailed(String),

    #[error("Failed to run solution check: {0}")]
    SolutionCheckFailed(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("UI Channel Closed")]
    UiChannelClosed,
}
