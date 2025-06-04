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
}
