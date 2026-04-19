use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

use crate::config::DirectoriesConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u8,
    pub file_path: String,
    pub wal_manifest: WalManifest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalManifest {
    pub partiton: HashMap<u8, WalPartition>,
    pub max_size: u32,
    pub hmac_key: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalPartition {
    pub file_idx: u32,
    pub last_idx: u64,
    pub ack_idx: u64,
}

impl Manifest {
    pub fn pending_file_location(&self, service_id: u8) -> Vec<String> {
        let partition = &self.wal_manifest.partiton[&service_id];

        let ack_file = partition.ack_idx / self.wal_manifest.max_size as u64;
        let last_file = partition.last_idx / self.wal_manifest.max_size as u64;

        (ack_file as u32..=last_file as u32)
            .into_iter()
            .map(|idx| format!("{}_{}.wal", service_id, idx))
            .collect()
    }

    pub fn wal_partition_init(&mut self, service_id: u8) {
        if self.wal_manifest.partiton.get(&service_id).is_none() {
            self.wal_manifest.partiton.insert(
                service_id,
                WalPartition {
                    file_idx: 0,
                    last_idx: 0,
                    ack_idx: 0,
                },
            );
        };

        let _ = self.save();
    }

    pub fn wal_maybe_increment(&mut self, service_id: u8) {
        let partition = self.wal_manifest.partiton.get_mut(&service_id).unwrap();

        partition.last_idx += 1;
        partition.ack_idx += 1;

        if partition.last_idx % self.wal_manifest.max_size as u64 == 0 {
            partition.file_idx += 1;
        }
    }

    pub fn wal_filename(&mut self, service_id: u8) -> String {
        let partition = self.wal_manifest.partiton.get_mut(&service_id).unwrap();
        format!("{}_{}.wal", service_id, partition.file_idx)
    }

    pub fn load(config: &DirectoriesConfig) -> Result<Self, ManifestError> {
        let path = Self::manifest_path(config);

        if !path.exists() {
            // Generate new manifest with fresh HMAC key
            let manifest = Manifest::new(&config.config_directory);
            manifest.save()?;
            return Ok(manifest);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| ManifestError::ReadError(path.display().to_string(), e))?;

        serde_json::from_str(&content).map_err(ManifestError::ParseError)
    }

    pub fn save(&self) -> Result<(), ManifestError> {
        let temp_path = std::path::Path::new(&self.file_path).join("manifest.json.tmp");

        // Ensure directory exists
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ManifestError::CreateDirError(parent.display().to_string(), e))?;
        }

        let content = serde_json::to_string_pretty(self).map_err(ManifestError::SerializeError)?;

        fs::write(&temp_path, content)
            .map_err(|e| ManifestError::WriteError(temp_path.display().to_string(), e))?;

        let path = std::path::Path::new(&self.file_path).join("manifest.json");
        fs::rename(temp_path, path).map_err(|e| ManifestError::WriteError("_".to_string(), e))?;

        Ok(())
    }

    fn manifest_path(config: &DirectoriesConfig) -> std::path::PathBuf {
        std::path::Path::new(&config.config_directory).join("manifest.json")
    }

    fn new(file_path: &str) -> Self {
        let mut hmac_key = [0u8; 32];
        rand::thread_rng().fill(&mut hmac_key);

        Manifest {
            version: 1,
            file_path: file_path.to_owned(),
            wal_manifest: WalManifest {
                hmac_key,
                max_size: 10,
                partiton: HashMap::new(),
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
