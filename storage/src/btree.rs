use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::storage_error::StorageError;
use serde::{Deserialize, Serialize};

const PAGE_SIZE: usize = 4096;
const MAX_KEYS_PER_PAGE: usize = 10;
const MIN_KEYS_PER_PAGE: usize = MAX_KEYS_PER_PAGE / 2 - 1;

pub trait PageStore: Read + Write + Seek {}
impl<T: Read + Write + Seek> PageStore for T {}

#[derive(Debug)]
pub struct PagingBtree {
    pub file_path: PathBuf,
    pub root_page_location: PageLocation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub key: Box<[u8]>,
    pub value_location: ValuePageLocation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuePageLocation {
    offset: u64,
    size: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PageLocation {
    pub start_offset: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PushResult {
    Inserted,
    Overflow(OverFlowElement),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum RemoveResult {
    Removed,
    NotFound,
    Underflow,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OverFlowElement {
    key: Box<[u8]>,
    page: Page,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Page {
    Internal(InternalNode),
    Leaf(LeafNode),
}

impl InternalNode {
    pub fn remove<W: PageStore>(&self, key: &[u8], storage: &mut W) -> RemoveResult {
        let index = self.index_of(key);
        let page_loc = self.pages[index];
        let mut page = page_loc.load_page(storage);
        let result = page.remove(key, storage);
        page_loc.write_page(&page, storage);

        match result {
            RemoveResult::NotFound => RemoveResult::NotFound,
            RemoveResult::Removed => RemoveResult::Removed,
            RemoveResult::Underflow => {
                if self.pages.len() > MIN_KEYS_PER_PAGE {
                    RemoveResult::Removed
                } else {
                    RemoveResult::Underflow
                }
            }
        }
    }

    pub fn get<W: PageStore>(&self, key: &[u8], storage: &mut W) -> Option<ValuePageLocation> {
        let index = self.index_of(key);
        self.pages[index].load_page(storage).get(key, storage)
    }

    pub fn add<W: PageStore>(&mut self, node: Node, storage: &mut W) -> PushResult {
        let index = self.index_of(&node.key);

        let page_loc = self.pages[index];
        let mut page = page_loc.load_page(storage);
        let result = page.add(node.clone(), storage);
        page_loc.write_page(&page, storage);

        match result {
            PushResult::Inserted => PushResult::Inserted,
            PushResult::Overflow(overflow) => {
                self.separators.insert(index + 1, overflow.key);

                let p_location = PageLocation::alloc(storage).unwrap();
                p_location.write_page(&overflow.page, storage);

                self.pages.insert(index + 1, p_location);

                if self.separators.len() >= MAX_KEYS_PER_PAGE {
                    let (page, key) = self.split();
                    PushResult::Overflow(OverFlowElement { key, page })
                } else {
                    PushResult::Inserted
                }
            }
        }
    }

    pub fn split(&mut self) -> (Page, Box<[u8]>) {
        let r_separators = self.separators.split_off(self.separators.len() / 2);
        let r_pages = self.pages.split_off(self.pages.len() / 2);

        let key = r_separators[0].clone();
        return (
            Page::Internal(InternalNode {
                separators: r_separators,
                pages: r_pages,
            }),
            key,
        );
    }

    fn index_of(&self, key: &[u8]) -> usize {
        self.separators.partition_point(|sep| sep.as_ref() <= key)
    }
}

impl LeafNode {
    pub fn remove(&mut self, key: &[u8]) -> RemoveResult {
        let index = self.nodes.partition_point(|n| n.key.as_ref() < key);
        if index < self.nodes.len() && self.nodes[index].key.as_ref() == key {
            self.nodes.remove(index);
            if self.nodes.len() > MIN_KEYS_PER_PAGE {
                RemoveResult::Removed
            } else {
                RemoveResult::Underflow
            }
        } else {
            RemoveResult::NotFound
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<ValuePageLocation> {
        for node in &self.nodes {
            if node.key.as_ref() == key {
                return Some(node.value_location.clone());
            }
        }
        None
    }

    pub fn add(&mut self, node: Node) -> PushResult {
        let index = self.index_of(&node.key);
        self.nodes.insert(index, node);
        if self.nodes.len() >= MAX_KEYS_PER_PAGE {
            let (page, key) = self.split();
            return PushResult::Overflow(OverFlowElement { key, page });
        }
        PushResult::Inserted
    }

    fn split(&mut self) -> (Page, Box<[u8]>) {
        let r_nodes = self.nodes.split_off(MAX_KEYS_PER_PAGE / 2);
        let m_node = r_nodes[0].clone();
        return (Page::Leaf(LeafNode { nodes: r_nodes }), m_node.key);
    }

    fn index_of(&self, key: &[u8]) -> usize {
        self.nodes.partition_point(|sep| sep.key.as_ref() <= key)
    }
}

impl Page {
    pub fn add<W: PageStore>(&mut self, adding: Node, storage: &mut W) -> PushResult {
        match self {
            Page::Internal(node) => node.add(adding, storage),
            Page::Leaf(node) => node.add(adding),
        }
    }

    pub fn split(&mut self) -> (Page, Box<[u8]>) {
        match self {
            Page::Internal(internal) => internal.split(),
            Page::Leaf(leaf) => leaf.split(),
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<ValuePageLocation> {
        match self {
            Page::Internal(internal) => internal.get(key, storage),
            Page::Leaf(leaf) => leaf.get(key),
        }
    }

    pub fn remove<W: PageStore>(&mut self, key: &[u8], storage: &mut W) -> RemoveResult {
        match self {
            Page::Internal(internal) => internal.remove(key, storage),
            Page::Leaf(leaf) => leaf.remove(key),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InternalNode {
    separators: Vec<Box<[u8]>>,
    pages: Vec<PageLocation>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeafNode {
    nodes: Vec<Node>,
}

impl PagingBtree {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path: file_path.clone(),
            root_page_location: PageLocation { start_offset: 0 },
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<ValuePageLocation> {
        let page = self.root_page_location.load_page(storage);
        page.get(key, storage)
    }

    pub fn remove<W: PageStore>(&self, key: &[u8], storage: &mut W) -> Result<(), StorageError> {
        let mut root_page = self.root_page_location.load_page(storage);

        root_page.remove(key, storage);

        Ok(())
    }

    pub fn add_node<W: PageStore>(
        &mut self,
        node: Node,
        storage: &mut W,
    ) -> Result<(), StorageError> {
        let mut root_page = self.root_page_location.load_page(storage);

        let result = root_page.add(node, storage);
        match result {
            PushResult::Overflow(overflow) => {
                let right_page_loc = PageLocation::alloc(storage)?;
                right_page_loc.write_page(&overflow.page, storage);

                let left_page_loc = PageLocation::alloc(storage)?;
                left_page_loc.write_page(&root_page, storage);

                let new_root_page = Page::Internal(InternalNode {
                    separators: vec![overflow.key],
                    pages: vec![left_page_loc, right_page_loc],
                });

                self.root_page_location.write_page(&new_root_page, storage);

                Ok(())
            }
            PushResult::Inserted => {
                self.root_page_location.write_page(&root_page, storage);
                Ok(())
            }
        }
    }
}

impl PageLocation {
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
