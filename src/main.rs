mod config;
mod coordinator;
mod database_engine;
mod gateway;

use std::sync::Arc;

use config::Config;
use database_engine::manifest::Manifest;
use database_engine::wal::WalOperation;

use crate::database_engine::wal::{WalReader, WalWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::load("config.toml")?);

    println!("Config directory: {}", config.storage.config_directory);
    println!("WAL directory: {}", config.storage.wal_directory);
    println!("SST directory: {}", config.storage.sst_directory);

    // Load or create manifest
    let manifest = Arc::new(Manifest::load(&config.storage)?);

    println!("Manifest version: {}", manifest.version);
    println!("Active WAL index: {}", manifest.wal_manifest.active_idx);
    println!("HMAC key: {:x?}", &manifest.wal_manifest.hmac_key[..8]);

    let wal_writer = WalWriter::new(&config.storage, &manifest);
    let wal_reader = WalReader::new(&config.storage, &manifest);

    wal_writer.write(
        WalOperation::Update,
        bincode::serialize("test-key")?,
        bincode::serialize("test-value")?,
    )?;

    let values = wal_reader.read()?;
    for value in values {
        let deserialized_key: String = bincode::deserialize(&value.key).unwrap();
        let deserialized_value: String = bincode::deserialize(&value.value).unwrap();

        println!("{}:{}", deserialized_key, deserialized_value);
    }

    println!("WAL write successful!");

    Ok(())
}
