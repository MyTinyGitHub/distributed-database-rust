use std::error::Error;

use tonic::Request;

use crate::wal::{wal_service_client::WalServiceClient, ReadRequest, WriteRequest};

pub mod wal {
    tonic::include_proto!("wal");
}

pub mod storage {
    tonic::include_proto!("storage");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "http://[::1]:50051";
    let mut client = WalServiceClient::connect(addr).await?;

    let write_request = WriteRequest {
        partition_name: "abc".to_owned(),
        operation: 1,
        key: bincode::serialize("test_key")?,
        value: bincode::serialize("test_value")?,
    };

    let request = Request::new(write_request);

    let _ = client.write(request).await?;

    let response = client
        .read(ReadRequest {
            partition_name: "abc".to_string(),
        })
        .await?;

    response.get_ref().entries.iter().for_each(|r| {
        let key: &str = bincode::deserialize(&r.key).unwrap();
        let value: &str = bincode::deserialize(&r.value).unwrap();
        let op: &str = r.operation().as_str_name();

        println!("operation: {}, key: {}, value: {}", op, key, value);
    });

    Ok(())
}
