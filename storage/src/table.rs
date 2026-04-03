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
        let (start_offset, end_offset) = self.file.insert(value);
        let engine_record = EngineRecord {
            version: 1,
            data: EngineHeader {
                start_offset,
                end_offset,
            },
        };

        let _ = self
            .indexes
            .get_mut(index_name)
            .unwrap()
            .insert(key, engine_record);
    }

    pub fn retrieve_data(&self, index_name: &str, key: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        let engine_header = &self
            .indexes
            .get(index_name)
            .unwrap()
            .get(&key)
            .unwrap()
            .data;

        let data = &self
            .file
            .read(engine_header.start_offset, engine_header.end_offset);

        Ok(data.clone())
    }

    pub fn create_index(&mut self, index: &str) {
        self.indexes.insert(index.to_string(), BTreeMap::new());
    }
}
