pub mod proto_storage {
    tonic::include_proto!("storage");
}

use std::collections::HashMap;

use storage::{config::Config, storage_error::StorageError, table::Table};

use crate::proto_storage::storage_engine_service_server::{
    StorageEngineService, StorageEngineServiceServer,
};

struct StorageEngine {
    config: Config,
    tables: HashMap<String, Table>,
}

impl StorageEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tables: HashMap::new(),
        }
    }

    pub fn create_table(&mut self, table_name: &str) -> Result<(), StorageError> {
        let table = Table::new(table_name, &self.config.directories)?;
        self.tables.insert(table_name.to_string(), table);
        Ok(())
    }

    pub fn create_index(
        &mut self,
        table_name: &str,
        index_name: Vec<u8>,
    ) -> Result<(), StorageError> {
        let _ = &self
            .tables
            .get_mut(table_name)
            .unwrap()
            .create_index(index_name);

        Ok(())
    }
}

#[tokio::main]
async fn main() {}
