use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::database_engine::database_engine_errors::DatabaseEngineError;

#[derive(Serialize, Deserialize)]
struct WalRecord {
    pub version: u8,
    pub check_sum: [u8; 16],
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

impl WalRecordData {
    pub fn generate_checksum(&self) -> [u8; 16] {
        let mut context = md5::Context::new();

        context.consume(&self.key);
        context.consume(&self.value);
        context.consume([self.operation as u8]);

        context.finalize().into()
    }
}

pub fn read_wal<R: Read>(reader: &mut R) -> Result<Vec<WalRecordData>, DatabaseEngineError> {
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

        if record.check_sum != record.data.generate_checksum() {
            return Err(DatabaseEngineError::Wal("invalid check_sum".to_owned()));
        }

        records.push(record.data);
    }

    Ok(records)
}

pub fn write_wal<W: Write>(
    writer: &mut W,
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
        check_sum: wal_record_data.generate_checksum(),
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
