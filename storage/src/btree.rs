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
    pub fn add<W: PageStore>(&mut self, node: Node, storage: &mut W) -> PushResult {
        let index = self.index_of(&node);
        let mut page = self.pages[index].load_page(storage);
        let result = page.add(node.clone(), storage);

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

    fn split(&mut self) -> (Page, Box<[u8]>) {
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

    fn index_of(&self, adding: &Node) -> usize {
        self.separators.partition_point(|sep| *sep <= adding.key)
    }
}

impl LeafNode {
    pub fn add(&mut self, node: Node) -> PushResult {
        let index = self.index_of(&node);
        self.nodes.insert(index, node.clone());
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

    fn index_of(&self, adding: &Node) -> usize {
        self.nodes.partition_point(|sep| sep.key <= adding.key)
    }
}

impl Page {
    fn add<W: PageStore>(&mut self, adding: Node, storage: &mut W) -> PushResult {
        match self {
            Page::Internal(node) => node.add(adding, storage),
            Page::Leaf(node) => node.add(adding),
        }
    }
}

pub trait AddPage {
    fn add<W: PageStore>(node: Node, storage: &mut W);
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

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<Box<[u8]>> {
        unimplemented!()
    }

    pub fn remove<W: PageStore>(&self, key: &[u8], storage: &mut W) {
        unimplemented!();
    }

    pub fn add_node<W: PageStore>(
        &mut self,
        node: Node,
        storage: &mut W,
    ) -> Result<(), StorageError> {
        let mut root_page = self.root_page_location.load_page(storage);

        root_page.add(node, storage);

        Ok(())
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
