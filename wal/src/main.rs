use std::{error::Error, sync::Arc};

use proto::wal_server::Wal;

use tonic::transport::Server;
use tonic_reflection::server::Builder;
use wal::{Config, Manifest, WalOperation, WalReader, WalWriter};

use crate::proto::WalReadDto;

pub mod proto {
    tonic::include_proto!("wal");
}

struct WalService {
    writer: WalWriter,
    reader: WalReader,
}

impl WalService {
    pub fn new(writer: WalWriter, reader: WalReader) -> Self {
        Self { writer, reader }
    }
}

#[async_trait::async_trait]
impl Wal for WalService {
    async fn write(
        &self,
        request: tonic::Request<proto::WalEntryDto>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        println!("Got a request: {:?}", request);

        let request = request.get_ref();

        let op =
            WalOperation::try_from(request.operation).map_err(tonic::Status::invalid_argument)?;

        self.writer
            .write(op, request.key.clone(), request.value.clone())
            .map_err(|_| tonic::Status::internal(""))?;

        Ok(tonic::Response::new(()))
    }

    async fn read(
        &self,
        _: tonic::Request<()>,
    ) -> Result<tonic::Response<proto::WalReadDto>, tonic::Status> {
        let result = self
            .reader
            .read()
            .map_err(|_| tonic::Status::internal(""))?;

        let result = result
            .iter()
            .map(|r| proto::WalEntryDto {
                key: r.key.clone(),
                value: r.value.clone(),
                operation: r.operation as i32,
            })
            .collect::<Vec<_>>();

        Ok(tonic::Response::new(WalReadDto { entries: result }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log/config/log4rs.yaml", Default::default())?;

    let config = Arc::new(Config::load("wal/config.toml")?);
    let manifest = Arc::new(Manifest::load(&config.storage)?);

    let wal_writer = WalWriter::new(&config.storage, &manifest);
    let wal_reader = WalReader::new(&config.storage, &manifest);
    let wal_service = WalService::new(wal_writer, wal_reader);

    let addr = "[::1]:50051".parse()?;
    let wal = proto::wal_server::WalServer::new(wal_service);
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
