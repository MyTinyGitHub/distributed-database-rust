use crate::{
    storage_error::StorageError,
    wal_client::wal::{wal_service_client::WalServiceClient, WalOperation, WriteRequest},
};

pub mod wal {
    tonic::include_proto!("wal");
}

pub async fn write(key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
    let mut client = WalServiceClient::connect("http://localhost:50051")
        .await
        .map_err(|_| StorageError::WalServiceNotAvailable())?;

    client
        .write(tonic::Request::new(WriteRequest {
            partition_name: "db1".to_string(),
            operation: WalOperation::Put as i32,
            key,
            value,
        }))
        .await
        .map_err(|_| StorageError::WalWriteFailed())?;

    Ok(())
}
