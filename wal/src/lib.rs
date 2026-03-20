pub mod config;
pub mod errors;
pub mod manifest;
pub mod wal;

pub use config::{Config, ConfigError, StorageConfig};
pub use errors::{DatabaseError, WalError};
pub use manifest::{Manifest, ManifestError, WalManifest};
pub use wal::{read_wal, write_wal, WalOperation, WalReader, WalRecord, WalRecordData, WalWriter};

impl TryFrom<i32> for WalOperation {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(WalOperation::Update),
            2 => Ok(WalOperation::Delete),
            _ => Err(format!("invalid operation: {}", value)),
        }
    }
}
