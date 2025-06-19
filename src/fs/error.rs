/// Errors generated from this module
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Application standard path not found
    #[error("Application standard dirs not found")]
    ApplicationDirsNotFound,

    /// Workshop data directory not found
    #[error("Workshop data directory not found")]
    WorkshopDataDirNotFound,

    /// No Python executable found
    #[error("No Python executable found")]
    NoPythonExecutable,

    /// No Docker Compose executable found
    #[error("No Docker Compose executable found")]
    NoDockerComposeExecutable,

    /// No Git executable found
    #[error("No Git executable found")]
    NoGitExecutable,
}
