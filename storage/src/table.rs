use std::collections::{BTreeMap, HashMap};
use std::ops::Index;
use std::path::{Path, PathBuf};

use crate::config::DirectoriesConfig;
use crate::heap_file::HeapFile;
use crate::record::EngineRecord;
use crate::storage_error::StorageError;

pub struct Table {
    table_name: String,
    file: HeapFile,
    indexes: HashMap<Vec<u8>, BTreeMap<Vec<u8>, EngineRecord>>,
}

impl Table {
    pub fn new(table_name: &str, config: &DirectoriesConfig) -> Result<Self, StorageError> {
        Ok(Self {
            table_name: table_name.to_string(),
            file: HeapFile::new(&config.heap_files, table_name)?,
            indexes: HashMap::new(),
        })
    }

    pub fn create_index(&mut self, index: Vec<u8>) {
        self.indexes.insert(index, BTreeMap::new());
    }

    pub fn add_index_record(&mut self, index: Vec<u8>, value: Vec<u8>, record: EngineRecord) {
        let _ = self.indexes.get_mut(&index).unwrap().insert(value, record);
    }
}
