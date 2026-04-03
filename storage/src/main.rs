pub mod proto_storage {
    tonic::include_proto!("storage");
}

use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

use storage::{config::Config, storage_error::StorageError, table::Table};
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection::server::Builder;

use crate::proto_storage::{
    storage_engine_service_server::{StorageEngineService, StorageEngineServiceServer},
    CreateTableRequest, CreateTableResponse, DropIndexRequest, DropIndexResponse, DropTableRequest,
    DropTableResponse, ReadByIndexRequest, ReadByIndexResponse, RegisterIndexRequest,
    RegisterIndexResponse, WriteRequest, WriteResponse,
};

struct StorageEngine {
    config: Arc<Config>,
    tables: HashMap<String, Table>,
}

impl StorageEngine {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config: config.clone(),
            tables: HashMap::new(),
        }
    }

    pub fn read_data(&self, table_name: &str, index_name: &str, index_key: Vec<u8>) -> Vec<u8> {
        self.tables
            .get(table_name)
            .unwrap()
            .retrieve_data(index_name, index_key)
            .unwrap()
    }

    pub fn insert_to_table(
        &mut self,
        index_name: &str,
        table_name: &str,
        key: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<(), StorageError> {
        let table = self
            .tables
            .get_mut(table_name)
            .expect("Unable to get the table");

        table.insert_data(index_name, (key, data));

        Ok(())
    }

    pub fn create_table(&mut self, table_name: &str) -> Result<(), StorageError> {
        let table = Table::new(table_name, &self.config.directories)?;
        self.tables.insert(table_name.to_string(), table);
        Ok(())
    }

    pub fn create_index(&mut self, table_name: &str, index_name: &str) -> Result<(), StorageError> {
        let _ = &self
            .tables
            .get_mut(table_name)
            .unwrap()
            .create_index(index_name);

        Ok(())
    }
}

struct StorageEngineServer {
    storage_engine: Arc<RwLock<StorageEngine>>,
}

impl StorageEngineServer {
    pub fn new(storage_engine: StorageEngine) -> Self {
        Self {
            storage_engine: Arc::new(RwLock::new(storage_engine)),
        }
    }
}

#[async_trait::async_trait]
impl StorageEngineService for StorageEngineServer {
    async fn write(
        &self,
        request: Request<WriteRequest>,
    ) -> Result<Response<WriteResponse>, Status> {
        let request = request.get_ref();

        let index = request.index_keys.get(0).unwrap();

        self.storage_engine
            .write()
            .unwrap()
            .insert_to_table(
                &index.index_name,
                &request.table,
                index.key.clone(),
                request.row_data.clone(),
            )
            .unwrap();

        Ok(Response::new(WriteResponse { success: false }))
    }

    async fn read_by_index(
        &self,
        request: Request<ReadByIndexRequest>,
    ) -> Result<Response<ReadByIndexResponse>, Status> {
        let request = request.get_ref();

        let data = self.storage_engine.write().unwrap().read_data(
            &request.table,
            &request.index_name,
            request.key.clone(),
        );

        let mut result = Vec::new();
        result.push(data);

        Ok(Response::new(ReadByIndexResponse { data: result }))
    }
    async fn create_table(
        &self,
        request: Request<CreateTableRequest>,
    ) -> Result<Response<CreateTableResponse>, Status> {
        let request = request.get_ref();

        self.storage_engine
            .write()
            .expect("Unable to get write lock")
            .create_table(&request.table)
            .expect("Error creating table");

        Ok(Response::new(CreateTableResponse { success: false }))
    }

    async fn drop_table(
        &self,
        request: Request<DropTableRequest>,
    ) -> Result<Response<DropTableResponse>, Status> {
        let request = request.get_ref();

        Ok(Response::new(DropTableResponse { success: false }))
    }

    async fn register_index(
        &self,
        request: Request<RegisterIndexRequest>,
    ) -> Result<Response<RegisterIndexResponse>, Status> {
        let request = request.get_ref();

        self.storage_engine
            .write()
            .expect("Unable to aquire a write lock")
            .create_index(&request.table, &request.index_name)
            .expect("unable to create index");

        Ok(Response::new(RegisterIndexResponse { success: false }))
    }

    async fn drop_index(
        &self,
        request: Request<DropIndexRequest>,
    ) -> Result<Response<DropIndexResponse>, Status> {
        let request = request.get_ref();

        Ok(Response::new(DropIndexResponse { success: false }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log/config/log4rs.yaml", Default::default())?;

    let config = Arc::new(Config::load("storage/config.toml")?);

    let addr = "[::1]:50052".parse()?;

    let storage_engine = StorageEngine::new(config);
    let storage_engine_server = StorageEngineServer::new(storage_engine);

    let reflection = Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/database.bin"
        )))
        .build_v1()?;

    Server::builder()
        .add_service(StorageEngineServiceServer::new(storage_engine_server))
        .add_service(reflection)
        .serve(addr)
        .await?;

    Ok(())
}
