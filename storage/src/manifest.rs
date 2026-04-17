use std::{
    collections::HashMap,
    fs::{create_dir, exists, read_to_string, rename, write},
};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{config::DirectoriesConfig, table::Table};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u8,
    pub file_path: String,
    pub tables: Vec<TableManifest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableManifest {
    pub table_name: String,
    pub indexes: Vec<String>,
}

impl Manifest {
    pub fn load_tables(&self, config: &DirectoriesConfig) -> HashMap<String, Table> {
        let mut result = HashMap::new();

        for table in &self.tables {
            result.insert(
                table.table_name.clone(),
                Table::load(&table.table_name, table.indexes.as_ref(), config),
            );
        }

        result
    }

    pub fn add_table(&mut self, table_name: &str) {
        self.tables.push(TableManifest {
            table_name: table_name.to_string(),
            indexes: Vec::new(),
        });

        self.save().unwrap();
    }

    pub fn add_index(&mut self, table_name: &str, index_name: &str) {
        self.tables
            .iter_mut()
            .find(|t| t.table_name == table_name)
            .map(|t| t.indexes.push(index_name.to_string()))
            .unwrap();

        self.save().unwrap();
    }

    pub fn remove_index(&mut self, table_name: &str, index_name: &str) {
        self.tables
            .iter_mut()
            .find(|t| t.table_name == table_name)
            .map(|t| t.indexes.remove(0))
            .unwrap();

        self.save().unwrap();
    }

    pub fn load(config: &DirectoriesConfig) -> Result<Self, ManifestError> {
        info!("loading manifest {}", &config.manifest);

        let path = Self::manifest_path(config);

        if !path.exists() {
            info!("loading manifest path does not exist {}", &config.manifest);
            // Generate new manifest with fresh HMAC key
            let manifest = Manifest::new(&config.manifest);
            manifest.save()?;
            return Ok(manifest);
        }

        let content = read_to_string(&path)
            .map_err(|e| ManifestError::ReadError(path.display().to_string(), e))?;

        info!("manifest loaded {} ", content);

        serde_json::from_str(&content).map_err(ManifestError::ParseError)
    }

    pub fn save(&self) -> Result<(), ManifestError> {
        let temp_path = std::path::Path::new(&self.file_path).join("manifest.json.tmp");

        // Ensure directory exists
        if let Some(parent) = temp_path.parent() {
            if !exists(parent).unwrap() {
                create_dir(parent)
                    .map_err(|e| ManifestError::CreateDirError(parent.display().to_string(), e))?;
            }
        }

        let content = serde_json::to_string_pretty(self).map_err(ManifestError::SerializeError)?;

        write(&temp_path, content)
            .map_err(|e| ManifestError::WriteError(temp_path.display().to_string(), e))?;

        let path = std::path::Path::new(&self.file_path).join("manifest.json");
        rename(temp_path, path).map_err(|e| ManifestError::WriteError("_".to_string(), e))?;

        Ok(())
    }

    fn manifest_path(config: &DirectoriesConfig) -> std::path::PathBuf {
        std::path::Path::new(&config.manifest)
            .join("manifest")
            .with_extension("json")
    }

    fn new(file_path: &str) -> Self {
        Manifest {
            version: 1,
            file_path: file_path.to_string(),
            tables: Vec::new(),
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
