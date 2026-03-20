//! Unified error types for database-core crate.

pub use crate::config::ConfigError;
pub use crate::manifest::ManifestError;
pub use crate::wal::WalError;

/// Database-level error that can encompass multiple error types.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("WAL error: {0}")]
    Wal(#[from] WalError),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<bincode::Error> for DatabaseError {
    fn from(e: bincode::Error) -> Self {
        DatabaseError::Serialization(e.to_string())
    }
}
