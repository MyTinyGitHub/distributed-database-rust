use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::storage_error::StorageError;
use serde::{Deserialize, Serialize};

const PAGE_SIZE: usize = 4096;
// TODO: tighten once key/value sizes are bounded
const MAX_KEYS_PER_PAGE: usize = 10;

pub trait PageStore: Read + Write + Seek {}
impl<T: Read + Write + Seek> PageStore for T {}

#[derive(Debug)]
pub struct PaginBtree {
    file_path: PathBuf,
    root_page_location: PageLocation,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Page {
    nodes: Vec<Node>,
    pages: Option<Vec<PageLocation>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    key: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageLocation {
    start_offset: u64,
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

impl PaginBtree {
    pub fn new(file_path: PathBuf) -> Self {
        //create file
        // insert pagefile to it
        Self {
            file_path: file_path.clone(),
            root_page_location: PageLocation { start_offset: 0 },
        }
    }

    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
        let file_path = self.file_path.clone();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)
            .map_err(StorageError::Io)?;

        self.add_node(&mut file, key, value)
    }

    fn add_node<W: PageStore>(
        &mut self,
        storage: &mut W,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), StorageError> {
        let root_page_location = self.root_page_location.clone();
        let mut root_page: Page = root_page_location.load_page(storage);
        let mut build_path = root_page.build_path(&key, storage, root_page_location);

        let mut overflow = None;
        for (page, page_location) in build_path.iter_mut() {
            let result = page.push(key.clone(), value.clone(), overflow, page_location, storage);

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

    pub fn push<W: PageStore>(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        prev_push: Option<PushResult>,
        page_location: &PageLocation,
        storage: &mut W,
    ) -> PushResult {
        let index = self.find_position(&key);

        match prev_push {
            None => {
                self.nodes.insert(index, Node { key, value });
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
        key: &Vec<u8>,
        storage: &mut W,
        page_location: PageLocation,
    ) -> Vec<(Page, PageLocation)> {
        if self.pages.is_none() {
            return vec![(self.clone(), page_location)];
        }

        let index = self.find_position(key);

        let child_location = self.pages.as_mut().unwrap().get(index).unwrap().clone();
        let mut page = child_location.load_page(storage);

        let mut build_path = page.build_path(key, storage, child_location.clone());
        build_path.push((self.clone(), page_location.clone()));

        build_path
    }

    fn find_position(&self, key: &Vec<u8>) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }

        if key < &self.nodes.get(0).unwrap().key {
            return 0;
        }

        for i in 1..self.nodes.len() {
            let cur_node = self.nodes.get(i).unwrap();
            let prev_node = self.nodes.get(i - 1).unwrap();

            if &prev_node.key < key && key <= &cur_node.key {
                return i;
            }
        }

        self.nodes.len()
    }

    pub fn contains_match(&self, key: &Vec<u8>) -> Option<&Node> {
        self.nodes
            .binary_search_by(|n| n.key.cmp(key))
            .ok()
            .map(|i| &self.nodes[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn make_tree() -> PaginBtree {
        PaginBtree {
            file_path: PathBuf::new(), // unused in add_node
            root_page_location: PageLocation { start_offset: 0 },
        }
    }

    #[test]
    fn test_write_read_roundtrip() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);

        let page = Page {
            nodes: vec![Node {
                key: vec![1],
                value: vec![1],
            }],
            pages: None,
        };

        let loc = PageLocation { start_offset: 0 };
        loc.write_page(&page, &mut storage);

        let loaded = loc.load_page(&mut storage);
        assert_eq!(
            loaded.nodes.len(),
            1,
            "roundtrip failed, got {} nodes",
            loaded.nodes.len()
        );
    }

    #[test]
    fn test_no_split_before_threshold() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..6 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len == PAGE_SIZE as u64,
            "expected no split at 6 inserts but file grew, len: {}",
            len
        );

        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(
            root.nodes.len(),
            6,
            "root should have 6 keys, got {}",
            root.nodes.len()
        );
        assert!(
            root.pages.is_none(),
            "root should still be a leaf at 6 inserts"
        );
    }

    #[test]
    fn test_split_on_eleventh_insert() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]); // pre-allocate root page
        let mut tree = make_tree();

        for i in 0u8..11 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        // after 11 inserts a split must have occurred
        // file should have more than one page
        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len > PAGE_SIZE as u64,
            "expected split but file is still one page len: {}",
            len
        );

        // root should be findable and contain the promoted key
        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(
            root.nodes.len(),
            1,
            "root should have exactly one promoted key after first split"
        );
        assert!(
            root.pages.is_some(),
            "root should have child pointers after split"
        );
        assert_eq!(
            root.pages.as_ref().unwrap().len(),
            2,
            "root should have exactly two children"
        );
    }

    #[test]
    fn test_all_keys_findable_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..11 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        // every inserted key must be findable by walking the tree
        println!("{:?}", tree.root_page_location);
        for i in 0u8..11 {
            let root = tree.root_page_location.load_page(&mut storage);
            let found = find_key(&mut storage, &root, &vec![i]);
            assert!(found, "key {} not found after split", i);
        }
    }

    #[test]
    fn test_exactly_max_keys_no_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..MAX_KEYS_PER_PAGE as u8 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len == PAGE_SIZE as u64,
            "expected no split at exactly MAX_KEYS_PER_PAGE inserts, len: {}",
            len
        );

        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(
            root.nodes.len(),
            MAX_KEYS_PER_PAGE,
            "root should have exactly {} keys, got {}",
            MAX_KEYS_PER_PAGE,
            root.nodes.len()
        );
        assert!(
            root.pages.is_none(),
            "should still be a leaf at MAX_KEYS_PER_PAGE"
        );
    }

    #[test]
    fn test_max_keys_plus_one_triggers_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len > PAGE_SIZE as u64,
            "expected split at MAX_KEYS_PER_PAGE + 1 inserts, len: {}",
            len
        );
    }

    #[test]
    fn test_single_insert_and_lookup() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, vec![42], vec![99])
            .expect("insert failed");

        let root = tree.root_page_location.load_page(&mut storage);
        let found = find_key(&mut storage, &root, &vec![42]);
        assert!(found, "single inserted key not found");
    }

    // ── Correctness ─────────────────────────────────────────────────────────────

    #[test]
    fn test_reverse_order_insert() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // insert in descending order
        for i in (0u8..11).rev() {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        for i in 0u8..11 {
            let root = tree.root_page_location.load_page(&mut storage);
            let found = find_key(&mut storage, &root, &vec![i]);
            assert!(found, "key {} not found after reverse order insert", i);
        }
    }

    #[test]
    fn test_duplicate_key_insert() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, vec![1], vec![10])
            .expect("insert failed");
        tree.add_node(&mut storage, vec![1], vec![20])
            .expect("insert failed");

        // key should still be findable regardless of upsert vs duplicate semantics
        let root = tree.root_page_location.load_page(&mut storage);
        let found = find_key(&mut storage, &root, &vec![1]);
        assert!(found, "key not found after duplicate insert");
    }

    #[test]
    fn test_multiple_splits_all_keys_findable() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // 5x MAX_KEYS_PER_PAGE to force multiple splits across multiple levels
        let count = (MAX_KEYS_PER_PAGE * 5) as u8;
        for i in 0u8..count {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        for i in 0u8..count {
            let root = tree.root_page_location.load_page(&mut storage);
            let found = find_key(&mut storage, &root, &vec![i]);
            assert!(found, "key {} not found after multiple splits", i);
        }
    }

    #[test]
    fn test_tree_is_sorted_after_multiple_splits() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // insert in random-ish order
        let keys: Vec<u8> = vec![5, 3, 8, 1, 9, 2, 7, 4, 6, 0, 11, 10];
        for &k in &keys {
            tree.add_node(&mut storage, vec![k], vec![k])
                .expect("insert failed");
        }

        // collect all keys by walking the tree in order
        let root = tree.root_page_location.load_page(&mut storage);
        let mut collected = Vec::new();
        collect_keys_in_order(&mut storage, &root, &mut collected);

        let mut sorted = collected.clone();
        sorted.sort();
        assert_eq!(collected, sorted, "tree keys are not in sorted order");
    }

    // ── Split structure ──────────────────────────────────────────────────────────

    #[test]
    fn test_split_children_key_counts() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // exactly MAX_KEYS_PER_PAGE + 1 to get a clean first split
        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(
            root.nodes.len(),
            1,
            "root should have 1 promoted key after first split"
        );

        let children = root.pages.as_ref().expect("root should have children");
        assert_eq!(children.len(), 2, "root should have exactly 2 children");

        let left = children[0].load_page(&mut storage);
        let right = children[1].load_page(&mut storage);

        let half = MAX_KEYS_PER_PAGE / 2;
        assert_eq!(
            left.nodes.len(),
            half,
            "left child should have {} keys, got {}",
            half,
            left.nodes.len()
        );
        assert_eq!(
            right.nodes.len(),
            half,
            "right child should have {} keys, got {}",
            half,
            right.nodes.len()
        );
    }

    #[test]
    fn test_split_no_key_loss_or_duplication() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        let count = MAX_KEYS_PER_PAGE + 1;
        for i in 0u8..count as u8 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        // collect all keys across the entire tree
        let root = tree.root_page_location.load_page(&mut storage);
        let mut all_keys = Vec::new();
        collect_keys_in_order(&mut storage, &root, &mut all_keys);

        // no duplicates
        let mut deduped = all_keys.clone();
        deduped.dedup();
        assert_eq!(all_keys, deduped, "found duplicate keys after split");

        // no missing keys — every inserted key must appear exactly once
        assert_eq!(
            all_keys.len(),
            count,
            "expected {} keys in tree, found {}",
            count,
            all_keys.len()
        );
    }

    #[test]
    fn test_page_alignment_maintained_after_splits() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..(MAX_KEYS_PER_PAGE * 3) as u8 {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(
            len % PAGE_SIZE as u64,
            0,
            "file is not page-aligned after splits, len: {}",
            len
        );
    }

    // ── Storage integrity ────────────────────────────────────────────────────────

    #[test]
    #[should_panic] // expected to fail until superblock/root persistence is implemented
    fn test_root_survives_reconstruction_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            tree.add_node(&mut storage, vec![i], vec![i])
                .expect("insert failed");
        }

        // simulate reopening — new tree always starts root at offset 0
        // but after a split, root has moved away from offset 0
        let tree2 = make_tree(); // root_page_location hardcoded to offset 0
        storage.seek(SeekFrom::Start(0)).unwrap();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            let root = tree2.root_page_location.load_page(&mut storage);
            let found = find_key(&mut storage, &root, &vec![i]);
            assert!(found, "key {} not found after reconstruction", i);
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────────

    fn collect_keys_in_order<S: PageStore>(storage: &mut S, page: &Page, out: &mut Vec<Vec<u8>>) {
        match &page.pages {
            None => {
                // leaf — collect all keys in order
                for node in &page.nodes {
                    out.push(node.key.clone());
                }
            }
            Some(children) => {
                // internal node — interleave children and keys
                // child[0], key[0], child[1], key[1], ..., child[n]
                for (i, child_loc) in children.iter().enumerate() {
                    let child = child_loc.load_page(storage);
                    collect_keys_in_order(storage, &child, out);
                    if i < page.nodes.len() {
                        out.push(page.nodes[i].key.clone());
                    }
                }
            }
        }
    }
    // walks the tree to find a key
    fn find_key<S: PageStore>(storage: &mut S, page: &Page, key: &Vec<u8>) -> bool {
        if page.contains_match(key).is_some() {
            return true;
        }

        if page.pages.is_none() {
            return page.contains_match(key).is_some();
        }

        let index = page.find_position(key);
        let child_location = &page.pages.as_ref().unwrap()[index];
        let child = child_location.load_page(storage);
        find_key(storage, &child, key)
    }
}
