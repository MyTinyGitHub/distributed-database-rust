mod config;
mod coordinator;
mod database_engine;
mod gateway;

use config::Config;
use database_engine::manifest::Manifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load("config.toml")?;

    println!("Config directory: {}", config.storage.config_directory);
    println!("WAL directory: {}", config.storage.wal_directory);
    println!("SST directory: {}", config.storage.sst_directory);

    // Load or create manifest
    let manifest = Manifest::load(&config.storage)?;

    println!("Manifest version: {}", manifest.version);
    println!("Active WAL index: {}", manifest.wal_manifest.active_idx);
    println!("HMAC key: {:x?}", &manifest.wal_manifest.hmac_key[..8]);

    Ok(())
}
