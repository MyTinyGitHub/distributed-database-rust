use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub directories: DirectoriesConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DirectoriesConfig {
    pub heap_files: String,
}
