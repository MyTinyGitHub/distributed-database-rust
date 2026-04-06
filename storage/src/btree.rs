use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::storage_error::StorageError;
use serde::{Deserialize, Serialize};

const PAGE_SIZE: usize = 4096;
const MAX_KEYS_PER_PAGE: usize = 10;

pub trait PageStore: Read + Write + Seek {}
impl<T: Read + Write + Seek> PageStore for T {}

#[derive(Debug)]
pub struct PagingBtree {
    pub file_path: PathBuf,
    pub root_page_location: PageLocation,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Page {
    pub nodes: Vec<Node>,
    pub pages: Option<Vec<PageLocation>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub key: Box<[u8]>,
    pub value: Box<[u8]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageLocation {
    pub start_offset: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PushResult {
    Inserted,
    Overflow(OverFlowElement),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OverFlowElement {
    node: Node,
    l_split: Option<PageLocation>,
    r_split: Option<PageLocation>,
}

impl PagingBtree {
    pub fn new(file_path: PathBuf) -> Self {
        //create file
        // insert pagefile to it
        Self {
            file_path: file_path.clone(),
            root_page_location: PageLocation { start_offset: 0 },
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<Box<[u8]>> {
        let root_page = self.root_page_location.load_page(storage);
        root_page.get(key, storage)
    }

    pub fn remove<R: PageStore>(&self, key: &[u8], storage: &mut R) {}

    pub fn add_node<W: PageStore>(
        &mut self,
        storage: &mut W,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), StorageError> {
        let root_page_location = self.root_page_location.clone();
        let mut root_page: Page = root_page_location.load_page(storage);
        let mut build_path = root_page.build_path(&key, storage, root_page_location);

        let mut overflow = None;
        for (page, page_location) in build_path.iter_mut() {
            let result = page.push(key, value, overflow, page_location, storage);

            if let PushResult::Inserted = result {
                overflow = None;
                break;
            }

            overflow = Some(result);
        }

        // root split — create a new root pointing to the two halves
        if let Some(PushResult::Overflow(overflow)) = overflow {
            let new_root_location = PageLocation::alloc(storage).map_err(StorageError::Io)?;
            let new_root = Page {
                nodes: vec![overflow.node],
                pages: Some(vec![overflow.l_split.unwrap(), overflow.r_split.unwrap()]),
            };
            new_root_location.write_page(&new_root, storage);
            self.root_page_location = new_root_location;
        }

        Ok(())
    }
}

impl PageLocation {
    pub fn alloc<W: PageStore>(storage: &mut W) -> std::io::Result<Self> {
        let offset = storage.seek(SeekFrom::End(0))?;
        debug_assert_eq!(offset % PAGE_SIZE as u64, 0, "file is not page-aligned");
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

impl Page {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            pages: None,
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<Box<[u8]>> {
        if let Some(node) = self.contains_match(key) {
            return Some(node.value.clone());
        }

        match &self.pages {
            None => None,
            Some(page) => {
                if let Some(index) = self.find_position(key) {
                    let child = &page[index];
                    let child = child.load_page(storage);
                    return child.get(key, storage);
                }
                None
            }
        }
    }

    pub fn push<W: PageStore>(
        &mut self,
        key: &[u8],
        value: &[u8],
        prev_push: Option<PushResult>,
        page_location: &PageLocation,
        storage: &mut W,
    ) -> PushResult {
        let index = self.find_position(&key).unwrap_or_else(|| 0);

        match prev_push {
            None => {
                self.nodes.insert(
                    index,
                    Node {
                        key: key.into(),
                        value: value.into(),
                    },
                );
            }
            Some(prev) => match prev {
                PushResult::Inserted => {
                    unreachable!("This method should not be called when previous statys was success without overflow")
                }
                PushResult::Overflow(overflow_element) => {
                    self.nodes.insert(index, overflow_element.node);
                    let pages = self.pages.as_mut().unwrap();

                    pages.remove(index);
                    pages.insert(index, overflow_element.r_split.unwrap());
                    pages.insert(index, overflow_element.l_split.unwrap());
                }
            },
        }

        if self.nodes.iter().len() <= MAX_KEYS_PER_PAGE {
            page_location.write_page(self, storage);
            return PushResult::Inserted;
        }

        let overflow = self.split(page_location, storage);

        PushResult::Overflow(overflow)
    }

    pub fn split<W: PageStore>(
        &mut self,
        page_location: &PageLocation,
        storage: &mut W,
    ) -> OverFlowElement {
        let (l_node_split, r_node_split) = self.nodes.split_at(MAX_KEYS_PER_PAGE / 2);
        let (overflow_node, r_node_split) = r_node_split.split_at(1);

        let (l_page_split, r_page_split) = match self.pages.take() {
            None => (None, None),
            Some(children) => {
                let r = children.split_at(MAX_KEYS_PER_PAGE / 2);
                (Some(Vec::from(r.0)), Some(Vec::from(r.1)))
            }
        };

        let l_location = page_location.clone();
        let r_location = PageLocation::alloc(storage).unwrap();

        let l_page = Page {
            nodes: Vec::from(l_node_split),
            pages: l_page_split,
        };

        let r_page = Page {
            nodes: Vec::from(r_node_split),
            pages: r_page_split,
        };

        l_location.write_page(&l_page, storage);
        r_location.write_page(&r_page, storage);

        OverFlowElement {
            node: overflow_node.get(0).unwrap().clone(),
            l_split: Some(l_location),
            r_split: Some(r_location),
        }
    }

    pub fn build_path<W: PageStore>(
        &mut self,
        key: &[u8],
        storage: &mut W,
        page_location: PageLocation,
    ) -> Vec<(Page, PageLocation)> {
        if self.pages.is_none() {
            return vec![(self.clone(), page_location)];
        }

        let index = self.find_position(key).unwrap();

        let child_location = &self.pages.as_mut().unwrap()[index];
        let mut page = child_location.load_page(storage);

        let mut build_path = page.build_path(key, storage, child_location.clone());
        build_path.push((self.clone(), page_location.clone()));

        build_path
    }

    pub fn find_position(&self, key: &[u8]) -> Option<usize> {
        if self.nodes.is_empty() {
            return None;
        }

        if key < self.nodes[0].key.as_ref() {
            return Some(0);
        }

        for i in 1..self.nodes.len() {
            let cur_node = self.nodes.get(i).unwrap();
            let prev_node = self.nodes.get(i - 1).unwrap();

            if prev_node.key.as_ref() < key && key <= cur_node.key.as_ref() {
                return Some(i);
            }
        }

        Some(self.nodes.len())
    }

    pub fn contains_match(&self, key: &[u8]) -> Option<&Node> {
        self.nodes
            .binary_search_by(|n| n.key.as_ref().cmp(key))
            .ok()
            .map(|i| &self.nodes[i])
    }
}
