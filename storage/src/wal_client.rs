use serde::{Deserialize, Serialize};

use crate::{
    storage_error::StorageError,
    wal_client::wal::{wal_service_client::WalServiceClient, WriteRequest},
};

pub mod wal {
    tonic::include_proto!("wal");
}

#[derive(Serialize, Deserialize)]
struct WalPayload {
    key: Vec<u8>,
    value: Vec<u8>,
}

pub async fn write(key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
    let mut client = WalServiceClient::connect("http://localhost:50051")
        .await
        .map_err(|_| StorageError::WalServiceNotAvailable())?;

    let wal_payload = WalPayload { key, value };

    let payload = bincode::serialize(&wal_payload).unwrap();

    client
        .write(tonic::Request::new(WriteRequest {
            service_id: 1,
            payload,
        }))
        .await
        .map_err(|_| StorageError::WalWriteFailed())?;

    Ok(())
}
