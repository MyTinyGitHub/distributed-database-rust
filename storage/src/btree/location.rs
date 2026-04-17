use std::io::{Read, Seek, SeekFrom, Write};

use serde::{Deserialize, Serialize};

use crate::{btree::page::Page, record::EngineRecord};

pub trait PageStore: Read + Write + Seek {}
impl<T: Read + Write + Seek> PageStore for T {}

const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Location {
    Value(EngineRecord),
    Page(RefPageLocation),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RefValueLocation {
    pub start_offset: u64,
    pub size: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RefPageLocation {
    pub start_offset: u64,
}

impl Location {
    pub fn load_page<R: PageStore>(&self, storage: &mut R) -> Page {
        match self {
            Location::Page(p) => p.load_page(storage),
            Location::Value(_) => unimplemented!(),
        }
    }

    pub fn write_page<W: PageStore>(&self, page: &Page, storage: &mut W) {
        match self {
            Location::Page(p) => p.write_page(page, storage),
            Location::Value(_) => unimplemented!(),
        }
    }
}

impl RefPageLocation {
    pub fn alloc<W: PageStore>(storage: &mut W) -> std::io::Result<Self> {
        let offset = storage.seek(SeekFrom::End(0))?;
        Ok(Self {
            start_offset: offset,
        })
    }

    pub fn load_page<R: PageStore>(&self, file: &mut R) -> Page {
        let mut buffer = vec![0u8; PAGE_SIZE];

        file.seek(SeekFrom::Start(self.start_offset))
            .expect("seek failed");

        file.read_exact(&mut buffer).expect("failed to read page");

        bincode::deserialize(&buffer).unwrap()
    }

    pub fn write_page<W: PageStore>(&self, page: &Page, storage: &mut W) {
        let encoded = bincode::serialize(page).expect("failed to serialize page");
        assert!(
            encoded.len() <= PAGE_SIZE,
            "page exceeds PAGE_SIZE: {} bytes",
            encoded.len()
        );

        let mut buf = vec![0u8; PAGE_SIZE];
        buf[..encoded.len()].copy_from_slice(&encoded);

        storage
            .seek(SeekFrom::Start(self.start_offset))
            .expect("seek failed");

        storage.write_all(&buf).expect("failed to write page");
    }
}
