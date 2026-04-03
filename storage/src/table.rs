use crate::heap_file::HeapFile;
use crate::record::EngineRecord;
use crate::storage_error::StorageError;
use crate::{config::DirectoriesConfig, record::EngineHeader};
use std::collections::{BTreeMap, HashMap};

pub struct Table {
    table_name: String,
    file: HeapFile,
    indexes: HashMap<String, BTreeMap<Vec<u8>, EngineRecord>>,
}

impl Table {
    pub fn new(table_name: &str, config: &DirectoriesConfig) -> Result<Self, StorageError> {
        Ok(Self {
            table_name: table_name.to_string(),
            file: HeapFile::new(&config.heap_files, table_name)?,
            indexes: HashMap::new(),
        })
    }

    pub fn insert_data(&mut self, index_name: &str, (key, value): (Vec<u8>, Vec<u8>)) {
        let engine_record = EngineRecord {
            version: 1,
            data: EngineHeader { data: value },
        };

        let _ = self
            .indexes
            .get_mut(index_name)
            .unwrap()
            .insert(key, engine_record);
    }

    pub fn retrieve_data(&self, index_name: &str, key: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        let result = self
            .indexes
            .get(index_name)
            .unwrap()
            .get(&key)
            .unwrap()
            .data
            .data
            .clone();

        Ok(result)
    }

    pub fn create_index(&mut self, index: &str) {
        self.indexes.insert(index.to_string(), BTreeMap::new());
    }
}
