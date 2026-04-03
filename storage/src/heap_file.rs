use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use std::os::unix::fs::FileExt;

use crate::storage_error::StorageError;

pub struct HeapFile {
    path: PathBuf,
    file: RwLock<File>,
}

impl HeapFile {
    pub fn insert(&mut self, data: Vec<u8>) -> (u64, u64) {
        let file_instance = self.file.get_mut().unwrap();
        let start_offset = file_instance.stream_position().unwrap();
        let size = file_instance.write(&data).unwrap();

        (start_offset, start_offset + size as u64)
    }

    pub fn read(&self, start_offset: u64, end_offset: u64) -> Vec<u8> {
        let file_instance = self.file.read().unwrap();
        // let seeker = SeekFrom::Start(start_offset);
        // let _ = file_instance.seek(seeker);
        let mut buffer = vec![0u8; (start_offset - end_offset) as usize];

        file_instance.read_at(&mut buffer, start_offset).unwrap();

        buffer
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
