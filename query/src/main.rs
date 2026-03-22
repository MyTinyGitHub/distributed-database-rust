use std::error::Error;

use tonic::Request;

use crate::proto::{wal_client::WalClient, WalRequest};

pub mod proto {
    tonic::include_proto!("wal");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "http://[::1]:50051";
    let mut client = WalClient::connect(addr).await?;

    let wal_request = WalRequest {
        operation: 1,
        key: "test".as_bytes().to_vec(),
        value: "test".as_bytes().to_vec(),
    };

    let request = Request::new(wal_request);
    let response = client.write(request).await?;

    println!("{:?}", response);

    Ok(())
}
