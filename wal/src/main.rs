use std::sync::Arc;

use wal::{Config, Manifest, WalOperation, WalReader, WalWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::load("config.toml")?);

    println!("Config directory: {}", config.storage.config_directory);
    println!("WAL directory: {}", config.storage.wal_directory);
    println!("SST directory: {}", config.storage.sst_directory);

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
        let deserialized_key: String = bincode::deserialize(&value.key)?;
        let deserialized_value: String = bincode::deserialize(&value.value)?;

        println!("{}:{}", deserialized_key, deserialized_value);
    }

    println!("WAL write successful!");

    Ok(())
}
