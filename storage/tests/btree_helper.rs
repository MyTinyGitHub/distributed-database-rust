use std::{fs::File, io::Cursor, path::PathBuf};

use storage::{
    btree::{
        leaf_page::Leaf,
        location::{Location, PageStore, RefPageLocation},
        page::Page,
        tree::PagingBtree,
    },
    record::{EngineHeader, EngineRecord},
};

pub const PAGE_SIZE: usize = 4096;
pub const MAX_KEYS_PER_PAGE: usize = 10;

fn create_loc(value: usize) -> Location {
    Location::Value(EngineRecord {
        version: 1,
        data: EngineHeader {
            start_offset: value as u64,
            size: value,
        },
    })
}

fn is_loc_equal(l_loc: Location, r_loc: Location) -> bool {
    match (l_loc, r_loc) {
        (Location::Value(l_v), Location::Value(r_v)) => {
            l_v.data.size == r_v.data.size && l_v.data.start_offset == r_v.data.start_offset
        }
        (Location::Page(l_p), Location::Page(r_p)) => l_p.start_offset == r_p.start_offset,
        _ => false,
    }
}

fn collect_all<R: PageStore>(page: &Page, storage: &mut R, result: &mut Vec<Box<[u8]>>) {
    match page {
        Page::Leaf(leaf) => {
            for (key, _) in leaf.keys.iter().zip(leaf.values.iter()) {
                result.push(key.clone());
            }
        }
        Page::Internal(internal) => {
            for page_loc in &internal.pages {
                let child = page_loc.load_page(storage);
                collect_all(&child, storage, result);
            }
        }
    }
}

pub fn check_is_root_sorted(tree: &mut PagingBtree<Cursor<Vec<u8>>>) {
    let storage = &mut tree.storage;
    let root_page = tree.root_page_location.load_page(storage);

    let mut collected = Vec::new();
    collect_all(&root_page, storage, &mut collected);
    let mut sorted = collected.clone();

    sorted.sort();
    assert_eq!(collected, sorted);
}

pub fn make_tree_from_stroage(storage: Cursor<Vec<u8>>) -> PagingBtree<Cursor<Vec<u8>>> {
    let location = RefPageLocation { start_offset: 0 };

    PagingBtree {
        storage,
        root_page_location: location,
    }
}

pub fn make_tree() -> PagingBtree<Cursor<Vec<u8>>> {
    let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    let location = RefPageLocation { start_offset: 0 };

    let root = Page::Leaf(Leaf {
        keys: Vec::new(),
        values: Vec::new(),
    });

    location.write_page(&root, &mut storage);

    PagingBtree {
        storage,
        root_page_location: location,
    }
}
