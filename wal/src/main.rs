use std::{error::Error, sync::Arc};

use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response};
use tonic_reflection::server::Builder;
use wal::{Config, Manifest, WalOperation, WalReader, WalWriter};

use crate::proto::{
    wal_service_server::{WalService, WalServiceServer},
    Entry, ReadRequest, ReadResponse, WriteResponse,
};

pub mod proto {
    tonic::include_proto!("wal");
}

struct WalSrv {
    writer: WalWriter,
    reader: WalReader,
}

impl WalSrv {
    pub fn new(writer: WalWriter, reader: WalReader) -> Self {
        Self { writer, reader }
    }
}

#[async_trait::async_trait]
impl WalService for WalSrv {
    async fn write(
        &self,
        request: Request<proto::WriteRequest>,
    ) -> Result<Response<WriteResponse>, tonic::Status> {
        println!("Got a request: {:?}", request);

        let request = request.get_ref();

        let op =
            WalOperation::try_from(request.operation).map_err(tonic::Status::invalid_argument)?;

        self.writer
            .write(
                &request.partition_name,
                op,
                request.key.clone(),
                request.value.clone(),
            )
            .await
            .map_err(|_| tonic::Status::internal(""))?;

        Ok(Response::new(WriteResponse {}))
    }

    async fn read(
        &self,
        request: Request<ReadRequest>,
    ) -> Result<Response<ReadResponse>, tonic::Status> {
        let result = self
            .reader
            .read(&request.get_ref().partition_name)
            .await
            .map_err(|_| tonic::Status::internal(""))?;

        let result = result
            .iter()
            .map(|r| Entry {
                key: r.key.clone(),
                value: r.value.clone(),
                operation: r.operation as i32,
            })
            .collect::<Vec<_>>();

        Ok(Response::new(ReadResponse { entries: result }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log/config/log4rs.yaml", Default::default())?;

    let config = Arc::new(Config::load("wal/config.toml")?);
    let manifest = Arc::new(RwLock::new(Manifest::load(&config.directories)?));

    let wal_writer = WalWriter::new(&config.directories, &manifest);
    let wal_reader = WalReader::new(&config.directories, &manifest);
    let wal_service = WalSrv::new(wal_writer, wal_reader);

    let addr = "[::1]:50051".parse()?;
    let wal = WalServiceServer::new(wal_service);
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
