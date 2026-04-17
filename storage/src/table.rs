use log::info;

use crate::btree::location::Location;
use crate::btree::tree::PagingBtree;
use crate::heap_file::HeapFile;
use crate::record::EngineRecord;
use crate::storage_error::StorageError;
use crate::{config::DirectoriesConfig, record::EngineHeader};

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Table {
    file: HeapFile,
    index_location: PathBuf,
    indexes: HashMap<String, PagingBtree<File>>,
}

impl Table {
    pub fn new(table_name: &str, config: &DirectoriesConfig) -> Result<Self, StorageError> {
        info!("creating table {}", table_name);

        Ok(Self {
            file: HeapFile::new(&config.heap_files, table_name)?,
            index_location: PathBuf::from(&config.index_files).join(table_name),
            indexes: HashMap::new(),
        })
    }

    pub fn insert(
        &mut self,
        index_name: &str,
        (key, value): (&[u8], Vec<u8>),
    ) -> Result<(), StorageError> {
        info!(
            "inserting (key: {:?}, value: {:?}) to index {}",
            key, value, index_name
        );

        let (start_offset, size) = self.file.insert(value);

        let engine_record = EngineRecord {
            version: 1,
            data: EngineHeader { start_offset, size },
        };

        self.indexes
            .get_mut(index_name)
            .ok_or_else(|| StorageError::IndexNotFound(index_name.to_string()))?
            .insert(key, Location::Value(engine_record))?;

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

    pub fn get(&mut self, index_name: &str, key: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        info!("getting index {} key {:?}", index_name, key);

        let location = &self
            .indexes
            .get_mut(index_name)
            .ok_or_else(|| StorageError::IndexNotFound(index_name.to_string()))?
            .get(&key)
            .ok_or(StorageError::IndexKeyNotFound())?;

        if let Location::Value(data) = location {
            let data = &self.file.read(data.data.start_offset, data.data.size);

            Ok(data.clone())
        } else {
            Ok(Vec::new())
        }
    }

    pub fn create_index(&mut self, index_name: &str) -> Result<(), StorageError> {
        if self.indexes.contains_key(index_name) {
            return Err(StorageError::IndexAlreadyExists(index_name.to_string()));
        }

        let index_path = self
            .index_location
            .clone()
            .join(index_name)
            .with_extension("idx");

        info!("creating index {}", index_name);

        self.indexes
            .insert(index_name.to_string(), PagingBtree::open(&index_path));

        info!("created index {}", index_name);

        Ok(())
    }
}
