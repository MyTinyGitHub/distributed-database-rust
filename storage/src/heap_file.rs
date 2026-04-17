use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, Write},
    path::PathBuf,
    sync::RwLock,
};

use std::os::unix::fs::FileExt;

use crate::storage_error::StorageError;

#[derive(Debug)]
pub struct HeapFile {
    path: PathBuf,
    file: RwLock<File>,
}

impl HeapFile {
    pub fn insert(&mut self, data: Vec<u8>) -> (u64, usize) {
        let file_instance = self.file.get_mut().unwrap();
        let start_offset = file_instance.stream_position().unwrap();
        let size = file_instance.write(&data).unwrap();

        (start_offset, size)
    }

    pub fn read(&self, start_offset: u64, size: usize) -> Vec<u8> {
        let file_instance = self.file.read().unwrap();
        let mut buffer = vec![0u8; size];

        file_instance.read_at(&mut buffer, start_offset).unwrap();

        buffer
    }

    pub fn delete(&self) {
        let _ = fs::remove_dir_all(self.path.parent().unwrap());
    }

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
            file: RwLock::new(file),
        })
    }
}
