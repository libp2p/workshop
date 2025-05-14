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
    TokioChannel(#[from] tokio::sync::mpsc::error::SendError<engine::Message>),

    /// Engine error
    #[error(transparent)]
    Engine(#[from] engine::Error),

    /// Language error
    #[error(transparent)]
    Languages(#[from] languages::Error),

    /// TUI error
    #[error("TUI error: {0}")]
    Tui(String),

    /// Project directories error
    #[error("Project directories error: {0}")]
    ProjectDirs(String),
}
