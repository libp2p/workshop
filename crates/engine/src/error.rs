/// Errors generated from this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlParsing(#[from] serde_yaml::Error),

    #[error("Workshop not found: {0}")]
    WorkshopNotFound(String),

    #[error("Workshop license not found: {0}")]
    WorkshopLicenseNotFound(String),

    #[error("Workshop setup not found: {0} ({1}, {2})")]
    WorkshopSetupNotFound(String, String, String),

    #[error("Workshop description not found: {0} ({1})")]
    WorkshopDescriptionNotFound(String, String),

    #[error("Lesson not found: {0}")]
    LessonNotFound(String),

    #[error("Failed to run dependency check: {0}")]
    DependencyCheckFailed(String),

    #[error("Failed to run solution check: {0}")]
    SolutionCheckFailed(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("UI Channel Closed")]
    UiChannelClosed,
}
