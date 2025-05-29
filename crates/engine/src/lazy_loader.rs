use std::path::{Path, PathBuf};
use tracing::info;

/// Trait that types must implement to be loadable
#[async_trait::async_trait]
pub trait TryLoad: Send + Sync {
    type Error;
    async fn try_load(path: &Path) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Enum to represent the lazy loader's state
#[derive(Clone, Debug)]
pub enum LazyLoader<T>
where
    T: TryLoad,
{
    NotLoaded(PathBuf),
    Loaded(T),
}

impl<T> LazyLoader<T>
where
    T: TryLoad,
{
    /// Attempts to load the data, returning a Result
    pub async fn try_load(&mut self) -> Result<&T, T::Error> {
        // Match on the current state
        match self {
            LazyLoader::NotLoaded(path) => {
                // Clone the path before borrowing self
                let path_clone = path.clone();
                info!(
                    "(lazy loader) attempting to load from path: {}",
                    path_clone.display()
                );
                // Attempt to load the data using the TryLoad trait
                let loaded = T::try_load(&path_clone).await?;
                // Transition to Loaded state
                *self = LazyLoader::Loaded(loaded);
                // Return a reference to the loaded data
                if let LazyLoader::Loaded(data) = self {
                    info!(
                        "(lazy loader) loaded data from path: {}",
                        path_clone.display()
                    );
                    Ok(data)
                } else {
                    unreachable!("Just set to Loaded, this should not happen")
                }
            }
            LazyLoader::Loaded(data) => {
                info!("(lazy loader) returning cached value from lazy loader");
                // If already loaded, return a reference to the data
                Ok(data)
            }
        }
    }
}

impl<T> From<&Path> for LazyLoader<T>
where
    T: TryLoad,
{
    fn from(path: &Path) -> Self {
        // Initialize the loader with the path
        LazyLoader::NotLoaded(path.to_path_buf())
    }
}
