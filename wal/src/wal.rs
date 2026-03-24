use std::{
    fs::OpenOptions,
    io::{Read, Write},
    sync::Arc,
};

use hmac_sha256::HMAC;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{config::StorageConfig, manifest::Manifest};

pub mod proto {
    tonic::include_proto!("wal");
}

#[derive(Debug, thiserror::Error)]
pub enum WalError {
    #[error("WAL error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid checksum")]
    InvalidChecksum,
}

impl From<bincode::Error> for WalError {
    fn from(e: bincode::Error) -> Self {
        WalError::Serialization(e.to_string())
    }
}

#[derive(Serialize, Deserialize)]
pub struct WalRecord {
    pub version: u8,
    pub check_sum: [u8; 32],
    pub data: WalRecordData,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[repr(u8)]
pub enum WalOperation {
    Update = 1,
    Delete = 2,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalRecordData {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub operation: WalOperation,
}

pub struct WalWriter {
    pub config: StorageConfig,
    pub manifest: Arc<RwLock<Manifest>>,
}

pub struct WalReader {
    pub config: StorageConfig,
    pub manifest: Arc<RwLock<Manifest>>,
}

impl WalReader {
    pub fn new(storage_config: &StorageConfig, manifest: &Arc<RwLock<Manifest>>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub async fn read(&self, partition_name: &str) -> Result<Vec<WalRecordData>, WalError> {
        let manifest = self.manifest.read().await;
        let wal_files = manifest.pending_file_location(partition_name);
        let mut result: Vec<WalRecordData> = Vec::new();

        for file in wal_files {
            let file_path = format!("{}/{}", &self.config.wal_directory, file);

            info!("reading: from file {:?}", file_path);

            let mut reader = OpenOptions::new().read(true).open(file_path)?;
            let hmac_key = &manifest.wal_manifest.hmac_key;

            result.append(&mut read_wal(&mut reader, hmac_key)?);
        }

        Ok(result)
    }
}

impl WalWriter {
    pub fn new(storage_config: &StorageConfig, manifest: &Arc<RwLock<Manifest>>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub async fn write(
        &self,
        partition_name: &str,
        operation: WalOperation,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), WalError> {
        info!(
            "writing: operation - {:?}, key - {:?}, value - {:?}",
            operation, key, value
        );

        let mut manifest = self.manifest.write().await;

        manifest.wal_partition_init(&partition_name);
        manifest.wal_maybe_increment(partition_name);

        let file_name = manifest.wal_filename(partition_name);

        let file_path = format!("{}/{}", &self.config.wal_directory, file_name);

        let mut writer = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)?;

        let hmac_key = &manifest.wal_manifest.hmac_key;

        write_wal(&mut writer, hmac_key, operation, key, value)?;

        writer.flush()?;

        Ok(())
    }
}

impl WalRecordData {
    pub fn generate_checksum(&self, hmac_key: &[u8]) -> [u8; 32] {
        let mut context = HMAC::new(hmac_key);

        context.update(&self.key);
        context.update(&self.value);
        context.update([self.operation as u8]);

        context.finalize()
    }
}

pub fn read_wal<R: Read>(reader: &mut R, hmac_key: &[u8]) -> Result<Vec<WalRecordData>, WalError> {
    let mut records = Vec::new();

    loop {
        let mut size_buf = [0u8; 8];

        match reader.read_exact(&mut size_buf) {
            Ok(_) => {}
            Err(_) => break,
        }

        let size = u64::from_le_bytes(size_buf) as usize;

        let mut payload = vec![0u8; size];

        reader.read_exact(&mut payload)?;

        let record: WalRecord = bincode::deserialize(&payload)?;

        if record.check_sum != record.data.generate_checksum(hmac_key) {
            return Err(WalError::InvalidChecksum);
        }

        records.push(record.data);
    }

    Ok(records)
}

pub fn write_wal<W: Write>(
    writer: &mut W,
    hmac_key: &[u8],
    operation: WalOperation,
    key: Vec<u8>,
    value: Vec<u8>,
) -> Result<(), WalError> {
    let wal_record_data = WalRecordData {
        operation,
        key,
        value,
    };

    let wal_record = WalRecord {
        version: 1,
        check_sum: wal_record_data.generate_checksum(hmac_key),
        data: wal_record_data,
    };

    let bytes = bincode::serialize(&wal_record)?;

    writer.write_all(&(bytes.len() as u64).to_le_bytes())?;

    writer.write_all(&bytes)?;

    Ok(())
}
