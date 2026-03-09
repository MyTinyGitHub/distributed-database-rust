use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::config::StorageConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: usize,
    pub wal_manifest: WalManifest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalManifest {
    pub active_idx: usize,
    pub hmac_key: [u8; 32],
}

impl Manifest {
    pub fn load(config: &StorageConfig) -> Result<Self, ManifestError> {
        let path = Self::manifest_path(config);

        if !path.exists() {
            // Generate new manifest with fresh HMAC key
            let manifest = Manifest::new();
            manifest.save(config)?;
            return Ok(manifest);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| ManifestError::ReadError(path.display().to_string(), e))?;

        serde_json::from_str(&content).map_err(ManifestError::ParseError)
    }

    pub fn save(&self, config: &StorageConfig) -> Result<(), ManifestError> {
        let path = Self::manifest_path(config);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ManifestError::CreateDirError(parent.display().to_string(), e))?;
        }

        let content = serde_json::to_string_pretty(self).map_err(ManifestError::SerializeError)?;

        fs::write(&path, content)
            .map_err(|e| ManifestError::WriteError(path.display().to_string(), e))
    }

    fn manifest_path(config: &StorageConfig) -> std::path::PathBuf {
        std::path::Path::new(&config.config_directory).join("manifest.json")
    }

    fn new() -> Self {
        let mut hmac_key = [0u8; 32];
        rand::thread_rng().fill(&mut hmac_key);

        Manifest {
            version: 1,
            wal_manifest: WalManifest {
                active_idx: 0,
                hmac_key,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("Failed to read manifest file {0}: {1}")]
    ReadError(String, std::io::Error),

    #[error("Failed to parse manifest file: {0}")]
    ParseError(serde_json::Error),

    #[error("Failed to serialize manifest: {0}")]
    SerializeError(serde_json::Error),

    #[error("Failed to write manifest file {0}: {1}")]
    WriteError(String, std::io::Error),

    #[error("Failed to create directory {0}: {1}")]
    CreateDirError(String, std::io::Error),
}
