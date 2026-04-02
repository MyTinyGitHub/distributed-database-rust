use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::Mutex,
};

use crate::storage_error::StorageError;

pub struct HeapFile {
    path: PathBuf,
    file: Mutex<File>,
}

impl HeapFile {
    pub fn new(storage_dir: &str, table_name: &str) -> Result<Self, StorageError> {
        let path = PathBuf::from(storage_dir).join(table_name).join("heap.db");

        std::fs::create_dir_all(path.parent().unwrap())?;

        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .map_err(StorageError::Io)?;

        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }
}
