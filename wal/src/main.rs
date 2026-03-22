use std::{error::Error, sync::Arc};

use proto::wal_server::Wal;

use tonic::transport::Server;
use tonic_reflection::server::Builder;
use wal::{Config, Manifest, WalOperation, WalWriter};

pub mod proto {
    tonic::include_proto!("wal");
}

struct WalService {
    writer: WalWriter,
}

impl WalService {
    pub fn new(writer: WalWriter) -> Self {
        Self { writer }
    }
}

#[async_trait::async_trait]
impl Wal for WalService {
    async fn write(
        &self,
        request: tonic::Request<proto::WalRequest>,
    ) -> Result<tonic::Response<proto::WalResponse>, tonic::Status> {
        println!("Got a request: {:?}", request);

        let request = request.get_ref();

        let op =
            WalOperation::try_from(request.operation).map_err(tonic::Status::invalid_argument)?;

        let _ = self
            .writer
            .write(op, request.key.clone(), request.value.clone());

        Ok(tonic::Response::new(proto::WalResponse {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log/config/log4rs.yaml", Default::default())?;

    let config = Arc::new(Config::load("wal/config.toml")?);
    let manifest = Arc::new(Manifest::load(&config.storage)?);

    let wal_writer = WalWriter::new(&config.storage, &manifest);

    let addr = "[::1]:50051".parse()?;
    let wal = proto::wal_server::WalServer::new(WalService::new(wal_writer));
    let reflection = Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/wal_descriptor.bin"
        )))
        .build_v1()?;

    Server::builder()
        .add_service(wal)
        .add_service(reflection)
        .serve(addr)
        .await?;

    Ok(())
}
