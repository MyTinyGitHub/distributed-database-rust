use std::{fs, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub directories: DirectoriesConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DirectoriesConfig {
    pub heap_files: String,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(path.display().to_string(), e))?;

        toml::from_str(&content).map_err(ConfigError::ParseError)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {0}: {1}")]
    ReadError(String, std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(toml::de::Error),
}
