#[cfg(test)]
mod tests {
    use storage::paging_btree::{Node, Page, PageLocation, PageStore, PagingBtree};

    use std::{
        io::{Cursor, Seek, SeekFrom},
        path::PathBuf,
    };

    const PAGE_SIZE: usize = 4096;
    const MAX_KEYS_PER_PAGE: usize = 10;

    fn make_tree() -> PagingBtree {
        PagingBtree {
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
