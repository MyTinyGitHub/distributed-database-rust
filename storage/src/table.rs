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

    pub fn insert_data(
        &mut self,
        index_name: &str,
        (key, value): (Vec<u8>, Vec<u8>),
    ) -> Result<(), StorageError> {
        let (start_offset, size) = self.file.insert(value);
        let engine_record = EngineRecord {
            version: 1,
            data: EngineHeader { start_offset, size },
        };

        self.indexes
            .get_mut(index_name)
            .ok_or_else(|| StorageError::IndexNotFound(index_name.to_string()))?
            .insert(key, engine_record)
            .ok_or_else(|| StorageError::IndexKeyNotFound())?;

        Ok(())
    }

    pub fn drop(&self) {
        self.file.delete();
    }

    pub fn drop_index(&mut self, index: &str) -> Result<(), StorageError> {
        self.indexes
            .remove(index)
            .ok_or_else(|| StorageError::IndexNotFound(index.to_string()))?;

        Ok(())
    }

    pub fn retrieve_data(&self, index_name: &str, key: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        let engine_header = &self
            .indexes
            .get(index_name)
            .ok_or_else(|| StorageError::IndexNotFound(index_name.to_string()))?
            .get(&key)
            .ok_or_else(|| StorageError::IndexKeyNotFound())?
            .data;

        let data = &self
            .file
            .read(engine_header.start_offset, engine_header.size);

        Ok(data.clone())
    }

    pub fn create_index(&mut self, index: &str) -> Result<(), StorageError> {
        if self.indexes.contains_key(index) {
            return Err(StorageError::IndexAlreadyExists(index.to_string()));
        }

        self.indexes.insert(index.to_string(), BTreeMap::new());

        Ok(())
    }
}
