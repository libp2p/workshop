/// Errors generated from this module
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Workshop data directory not found
    #[error("Workshop data directory not found")]
    WorkshopDataDirNotFound,
}
