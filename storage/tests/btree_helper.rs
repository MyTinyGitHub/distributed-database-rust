use std::path::PathBuf;
use storage::btree::{Page, PageLocation, PageStore, PagingBtree};

pub const PAGE_SIZE: usize = 4096;
pub const MAX_KEYS_PER_PAGE: usize = 10;

pub fn make_tree() -> PagingBtree {
    PagingBtree {
        file_path: PathBuf::new(),
        root_page_location: PageLocation { start_offset: 0 },
    }
}

pub fn collect_keys_in_order<S: PageStore>(storage: &mut S, page: &Page, out: &mut Vec<Vec<u8>>) {
    match &page.pages {
        None => {
            for node in &page.nodes {
                out.push(node.key.clone().into());
            }
        }
        Some(children) => {
            for (i, child_loc) in children.iter().enumerate() {
                let child = child_loc.load_page(storage);
                collect_keys_in_order(storage, &child, out);
                if i < page.nodes.len() {
                    out.push(page.nodes[i].key.clone().into());
                }
            }
        }
    }
}
