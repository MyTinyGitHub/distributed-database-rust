use std::{
    fs::OpenOptions,
    io::{Read, Write},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use hmac_sha256::HMAC;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{config::DirectoriesConfig, manifest::Manifest};

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
    pub lsn: u64,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

pub struct WalWriter {
    pub config: DirectoriesConfig,
    pub manifest: Arc<RwLock<Manifest>>,
}

pub struct WalReader {
    pub config: DirectoriesConfig,
    pub manifest: Arc<RwLock<Manifest>>,
}

impl WalReader {
    pub fn new(storage_config: &DirectoriesConfig, manifest: &Arc<RwLock<Manifest>>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub async fn read(&self, service_id: u8) -> Result<Vec<Vec<u8>>, WalError> {
        let manifest = self.manifest.read().await;
        let wal_files = manifest.pending_file_location(service_id);
        let mut result: Vec<Vec<u8>> = Vec::new();

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
    pub fn new(storage_config: &DirectoriesConfig, manifest: &Arc<RwLock<Manifest>>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub async fn write(&self, service_id: u8, payload: Vec<u8>) -> Result<(), WalError> {
        info!("writing: operation - {}", service_id);

        let mut manifest = self.manifest.write().await;

        manifest.wal_partition_init(service_id);
        manifest.wal_maybe_increment(service_id);

        let file_name = manifest.wal_filename(service_id);

        let file_path = format!("{}/{}", &self.config.wal_directory, file_name);

        let mut writer = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)?;

        let hmac_key = &manifest.wal_manifest.hmac_key;

        write_wal(&mut writer, hmac_key, payload)?;

        writer.flush()?;

        Ok(())
    }
}

pub fn read_wal<R: Read>(reader: &mut R, hmac_key: &[u8]) -> Result<Vec<Vec<u8>>, WalError> {
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

        //if record.check_sum != record.data.generate_checksum(hmac_key) {
        //   return Err(WalError::InvalidChecksum);
        //}

        records.push(record.data);
    }

    Ok(records)
}

pub fn write_wal<W: Write>(
    writer: &mut W,
    hmac_key: &[u8],
    payload: Vec<u8>,
) -> Result<(), WalError> {
    let version = 1;
    let data = payload;
    let lsn = 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let check_sum = [1; 32];

    let wal_record = WalRecord {
        version,
        lsn,
        timestamp,
        check_sum,
        data,
    };

    let bytes = bincode::serialize(&wal_record)?;

    writer.write_all(&(bytes.len() as u64).to_le_bytes())?;

    writer.write_all(&bytes)?;

    Ok(())
}
