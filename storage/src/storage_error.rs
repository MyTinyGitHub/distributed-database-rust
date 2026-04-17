use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Index Key not found: ")]
    IndexKeyNotFound(),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Index already exists: {0}")]
    IndexAlreadyExists(String),

    #[error("Table already exists: {0}")]
    TableAlreadyExists(String),

    #[error("Wal service is not available")]
    WalServiceNotAvailable(),

    #[error("Unable to write payload to wal service")]
    WalWriteFailed(),
}
