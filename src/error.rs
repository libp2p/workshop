/// Errors generated from this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParsing(#[from] serde_yaml::Error),

    /// Tokio JoinError
    #[error("Tokio JoinError: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Tokio Channel error
    #[error("Tokio Channel error: {0}")]
    TokioChannel(#[from] tokio::sync::mpsc::error::SendError<crate::ui::tui::screens::Event>),

    /// Language error
    #[error(transparent)]
    Languages(#[from] crate::languages::Error),

    /// Models error
    #[error(transparent)]
    Models(#[from] crate::models::Error),

    /// Fs error
    #[error(transparent)]
    Fs(#[from] crate::fs::Error),

    /// Status mutex lock error
    #[error("Status mutex lock error: {0}")]
    StatusLock(String),

    /// TUI error
    #[error("TUI error: {0}")]
    Tui(String),

    /// Project directories error
    #[error("Project directories error: {0}")]
    ProjectDirs(String),

    /// No Python executable found
    #[error("No Python executable found")]
    NoPythonExecutable,

    /// Command error
    #[error("Command error: {0}")]
    Command(String),
}
