use std::{
    fs::OpenOptions,
    io::{Read, Write},
    sync::Arc,
};

use hmac_sha256::HMAC;
use serde::{Deserialize, Serialize};

use crate::{
    config::StorageConfig,
    database_engine::{database_engine_errors::DatabaseEngineError, manifest::Manifest},
};

#[derive(Serialize, Deserialize)]
struct WalRecord {
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
    pub manifest: Arc<Manifest>,
}

pub struct WalReader {
    pub config: StorageConfig,
    pub manifest: Arc<Manifest>,
}

impl WalReader {
    pub fn new(storage_config: &StorageConfig, manifest: &Arc<Manifest>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub fn read(&self) -> Result<Vec<WalRecordData>, DatabaseEngineError> {
        let file_path = format!(
            "{}/{}.wal",
            &self.config.wal_directory, self.manifest.wal_manifest.active_idx
        );

        let mut reader = OpenOptions::new()
            .read(true)
            .open(file_path)
            .map_err(|_| DatabaseEngineError::Wal("Unable to open WAL file".to_owned()))?;

        let hmac_key = &self.manifest.wal_manifest.hmac_key;

        read_wal(&mut reader, hmac_key)
    }
}

impl WalWriter {
    pub fn new(storage_config: &StorageConfig, manifest: &Arc<Manifest>) -> Self {
        Self {
            config: storage_config.clone(),
            manifest: Arc::clone(manifest),
        }
    }

    pub fn write(
        &self,
        operation: WalOperation,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), DatabaseEngineError> {
        let file_path = format!(
            "{}/{}.wal",
            &self.config.wal_directory, self.manifest.wal_manifest.active_idx
        );

        let mut writer = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(file_path)
            .map_err(|_| DatabaseEngineError::Wal("Unable to open file".to_owned()))?;

        let hmac_key = &self.manifest.wal_manifest.hmac_key;

        write_wal(&mut writer, hmac_key, operation, key, value)?;

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

pub fn read_wal<R: Read>(
    reader: &mut R,
    hmac_key: &[u8],
) -> Result<Vec<WalRecordData>, DatabaseEngineError> {
    let mut records = Vec::new();

    loop {
        let mut size_buf = [0u8; 8];

        match reader.read_exact(&mut size_buf) {
            Ok(_) => {}
            Err(_) => break,
        }

        let size = u64::from_le_bytes(size_buf) as usize;

        let mut payload = vec![0u8; size];

        reader
            .read_exact(&mut payload)
            .expect("Failed to read payload");

        let record: WalRecord =
            bincode::deserialize(&payload).expect("Failed to deserialize WalRecord");

        if record.check_sum != record.data.generate_checksum(hmac_key) {
            return Err(DatabaseEngineError::Wal("invalid check_sum".to_owned()));
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
) -> Result<(), DatabaseEngineError> {
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

    let bytes = bincode::serialize(&wal_record).expect("Unable to serialize WALRecord");

    writer
        .write_all(&(bytes.len() as u64).to_le_bytes())
        .map_err(|_| DatabaseEngineError::Wal("Unable to persist length of payload".to_owned()))?;

    writer
        .write_all(&bytes)
        .map_err(|_| DatabaseEngineError::Wal("Unable to persist the payload".to_owned()))?;

    Ok(())
}
