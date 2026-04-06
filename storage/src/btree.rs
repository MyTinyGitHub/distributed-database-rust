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
pub struct Page {
    pub nodes: Vec<Node>,
    pub pages: Option<Vec<PageLocation>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub key: Box<[u8]>,
    pub value: Box<[u8]>,
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

    pub fn remove<W: PageStore>(&self, key: &[u8], storage: &mut W) {
        let root_page_location = self.root_page_location.clone();
        let mut root_page: Page = root_page_location.load_page(storage);
        let mut build_path = root_page.build_path(&key, storage, root_page_location);

        let mut needs_rebalancing = false;
        let mut removed = false;
        for (_, page_location) in build_path.iter_mut() {
            let mut page = page_location.load_page(storage);

            if let Some(pages) = &page.pages {}

            if removed && needs_rebalancing == false {
                break;
            }

            if removed && needs_rebalancing {
                page.rebalance(key, storage);
                page_location.write_page(&page, storage);
                needs_rebalancing = page.nodes.len() < MIN_KEYS_PER_PAGE;
                continue;
            }

            if page.contains_match(key).is_some() {
                page.remove(key);
                page_location.write_page(&page, storage);
                needs_rebalancing = page.nodes.len() < MIN_KEYS_PER_PAGE;
                removed = true;
                continue;
            }
        }

        //leaf and num_keys > minimum -> OK
        //leaf and num_keys < minimum && sibling has keys -> Take KEY from sibling with more KEYS and replace it with internal separator and push separator to the node
        //leaf and num_keys < minimum && sibling keys < minimum -> Merge siblings

        //midle of tree -> take key from leaf, so that all keys are smaller/greater rebalance tree if neede
    }

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

    pub fn remove(&mut self, key: &[u8]) -> Node {
        let index = self.index_of(key).unwrap();
        self.nodes.remove(index)
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
                let r = children.split_at(MAX_KEYS_PER_PAGE / 2 + 1);
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

        let index = self.find_position(key);
        if index.is_none() {
            return vec![];
        }

        let index = index.unwrap();

        let child_location = &self.pages.as_mut().unwrap()[index];
        let mut page = child_location.load_page(storage);

        let mut build_path = page.build_path(key, storage, child_location.clone());
        build_path.push((self.clone(), page_location.clone()));

        build_path
    }

    pub fn index_of(&self, key: &[u8]) -> Option<usize> {
        for i in 0..self.nodes.len() {
            if self.nodes[i].key.as_ref() == key {
                return Some(i);
            }
        }

        None
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

    pub fn rebalance<W: PageStore>(&mut self, key: &[u8], storage: &mut W) {
        assert!(self.pages.is_some());

        let index = self.find_position(key).unwrap();
        let m_n = self.pages.as_ref().unwrap()[index];
        let mut m_page = m_n.load_page(storage);

        let neighbours = self.child_neighbours(index);

        match neighbours {
            (None, None) => unreachable!("Should not happend that there is only one child"),
            (None, Some(r_n)) => {
                let mut r_page = r_n.load_page(storage);
                if r_page.nodes.len() > MIN_KEYS_PER_PAGE {
                    self.rebalance_right_page(&mut m_page, &mut r_page);
                } else {
                    self.merge_right_page(&mut m_page, &mut r_page);
                }

                m_n.write_page(&m_page, storage);
                r_n.write_page(&r_page, storage);
            }
            (Some(l_n), None) => {
                let mut l_page = l_n.load_page(storage);

                if l_page.nodes.len() > MIN_KEYS_PER_PAGE {
                    self.rebalance_left_page(&mut m_page, &mut l_page);
                } else {
                    self.merge_left_page(index, &mut m_page, &mut l_page);
                }

                m_n.write_page(&m_page, storage);
                l_n.write_page(&l_page, storage);
            }
            (Some(l_n), Some(r_n)) => {
                let mut l_page = l_n.load_page(storage);
                let mut r_page = r_n.load_page(storage);

                if r_page.nodes.len() > MIN_KEYS_PER_PAGE {
                    let r_node = r_page.nodes.remove(0);
                    let sep_node = self.nodes.remove(index);

                    self.nodes.insert(index, r_node);
                    m_page.nodes.push(sep_node);

                    if r_page.pages.is_some() {
                        let r_page = r_page.pages.as_mut().unwrap().remove(0);
                        // let sep_page = self.pages.as_mut().unwrap().remove(index + 1);
                        // self.pages.as_mut().unwrap().insert(index + 1, r_page);
                        m_page.pages.as_mut().unwrap().push(r_page);
                    }
                } else if l_page.nodes.len() > MIN_KEYS_PER_PAGE {
                    let l_node = l_page.nodes.remove(l_page.nodes.len() - 1);
                    let sep_node = self.nodes.remove(index - 1);

                    self.nodes.insert(index - 1, l_node);
                    m_page.nodes.insert(0, sep_node);

                    if let Some(pages) = l_page.pages.as_mut() {
                        let l_page_index = pages.len() - 1;
                        let l_page = pages.remove(l_page_index);
                        m_page.pages.as_mut().unwrap().insert(0, l_page);
                    }
                } else {
                    // let sep_node = self.nodes.remove(index);

                    // m_page.nodes.push(sep_node);
                    // m_page.nodes.append(&mut r_page.nodes);

                    // if let Some(pages) = m_page.pages.as_mut() {
                    //     pages.append(r_page.pages.as_mut().unwrap());
                    // }

                    // self.pages.as_mut().unwrap().remove(index + 1);
                    self.merge_left_page(index, &mut m_page, &mut l_page);
                }

                m_n.write_page(&m_page, storage);
                l_n.write_page(&l_page, storage);
                r_n.write_page(&r_page, storage);
            }
        };
    }

    fn merge_left_page(&mut self, index: usize, m_page: &mut Page, l_page: &mut Page) {
        let sep_node = self.nodes.remove(index - 1);
        l_page.nodes.push(sep_node);
        l_page.nodes.append(&mut m_page.nodes);

        if let Some(pages) = l_page.pages.as_mut() {
            pages.append(m_page.pages.as_mut().unwrap());
        }

        self.pages.as_mut().unwrap().remove(index);
    }

    fn merge_right_page(&mut self, m_page: &mut Page, r_page: &mut Page) {
        let sep_node = self.nodes.remove(0);
        m_page.nodes.push(sep_node);
        m_page.nodes.append(&mut r_page.nodes);

        if let Some(pages) = m_page.pages.as_mut() {
            pages.append(r_page.pages.as_mut().unwrap());
        }

        self.pages.as_mut().unwrap().remove(1);
    }

    fn rebalance_right_page(&mut self, m_page: &mut Page, r_page: &mut Page) {
        let r_node = r_page.nodes.remove(0);
        let sep_node = self.nodes.remove(0);

        if r_page.pages.is_some() {
            let r_page = r_page.pages.as_mut().unwrap().remove(0);
            m_page.pages.as_mut().unwrap().push(r_page);
        }

        self.nodes.insert(0, r_node);
        m_page.nodes.push(sep_node);
    }

    fn rebalance_left_page(&mut self, m_page: &mut Page, l_page: &mut Page) {
        let l_node = l_page.nodes.remove(l_page.nodes.len() - 1);
        let sep_node = self.nodes.remove(self.nodes.len() - 1);

        self.nodes.push(l_node);
        m_page.nodes.insert(0, sep_node);

        if let Some(pages) = l_page.pages.as_mut() {
            let l_page_index = pages.len() - 1;
            let l_page = pages.remove(l_page_index);
            m_page.pages.as_mut().unwrap().insert(0, l_page);
        }
    }

    pub fn child_neighbours(&self, index: usize) -> (Option<PageLocation>, Option<PageLocation>) {
        let pages = self.pages.as_ref().unwrap();

        let l_page = if index > 0 {
            let l_index = index - 1;
            Some(pages[l_index].clone())
        } else {
            None
        };

        let r_page = if index + 1 < pages.len() {
            let r_index = index + 1;
            Some(pages[r_index].clone())
        } else {
            None
        };

        (l_page, r_page)
    }

    pub fn contains_match(&self, key: &[u8]) -> Option<&Node> {
        self.nodes
            .binary_search_by(|n| n.key.as_ref().cmp(key))
            .ok()
            .map(|i| &self.nodes[i])
    }

    pub fn is_leaf(&self) -> bool {
        self.pages.is_none()
    }
}
