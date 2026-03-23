use std::error::Error;

use tonic::Request;

use crate::proto::{wal_client::WalClient, WalEntryDto};

pub mod proto {
    tonic::include_proto!("wal");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "http://[::1]:50051";
    let mut client = WalClient::connect(addr).await?;

    let wal_request = WalEntryDto {
        operation: 1,
        key: bincode::serialize("test_key")?,
        value: bincode::serialize("test_value")?,
    };

    let request = Request::new(wal_request);

    let _ = client.write(request).await?;

    let response = client.read(()).await?;

    response.get_ref().entries.iter().for_each(|r| {
        let key: &str = bincode::deserialize(&r.key).unwrap();
        let value: &str = bincode::deserialize(&r.value).unwrap();
        let op: &str = r.operation().as_str_name();

        println!("operation: {}, key: {}, value: {}", op, key, value);
    });

    Ok(())
}
