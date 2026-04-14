include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    // // ── Basic push ───────────────────────────────────────────────────────────────

    use std::io::{Cursor, Seek, SeekFrom};

    use storage::btree::{
        leaf_page::Leaf,
        location::{Location, RefPageLocation},
        page::Page,
    };

    use crate::{create_loc, make_tree, PAGE_SIZE};

    #[test]
    fn test_write_read_roundtrip() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);

        let page = Page::Leaf(Leaf {
            keys: vec![Box::new([1 as u8])],
            values: vec![create_loc(1 as usize)],
        });

        let loc = Location::Page(RefPageLocation { start_offset: 0 });

        loc.write_page(&page, &mut storage);

        let loaded = loc.load_page(&mut storage);
        assert_eq!(
            loaded.size(),
            1,
            "roundtrip failed, got {} nodes",
            loaded.size()
        );
    }

    #[test]
    fn test_no_split_before_threshold() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        for i in 0u8..6 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();

        assert!(
            len == PAGE_SIZE as u64,
            "expected no split at 6 inserts but file grew, len: {}",
            len
        );

        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(
            root.size(),
            6,
            "root should have 6 keys, got {}",
            root.size()
        );
    }

    // #[test]
    // fn test_split_on_eleventh_insert() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..11 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let len = storage.seek(SeekFrom::End(0)).unwrap();
    //     assert!(
    //         len > PAGE_SIZE as u64,
    //         "expected split but file is still one page len: {}",
    //         len
    //     );

    //     let root = tree.root_page_location.load_page(&mut storage);
    //     assert_eq!(
    //         root.nodes.len(),
    //         1,
    //         "root should have exactly one promoted key after first split"
    //     );
    //     assert!(
    //         root.pages.is_some(),
    //         "root should have child pointers after split"
    //     );
    //     assert_eq!(
    //         root.pages.as_ref().unwrap().len(),
    //         2,
    //         "root should have exactly two children"
    //     );
    // }

    // #[test]
    // fn test_all_keys_findable_after_split() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..11 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     for i in 0u8..11 {
    //         let result = tree.get(&[i], &mut storage);
    //         assert!(result.is_some(), "key {} not found after split", i);
    //     }
    // }

    // #[test]
    // fn test_exactly_max_keys_no_split() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..MAX_KEYS_PER_PAGE as u8 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let len = storage.seek(SeekFrom::End(0)).unwrap();
    //     assert!(
    //         len == PAGE_SIZE as u64,
    //         "expected no split at exactly MAX_KEYS_PER_PAGE inserts, len: {}",
    //         len
    //     );

    //     let root = tree.root_page_location.load_page(&mut storage);
    //     assert_eq!(
    //         root.nodes.len(),
    //         MAX_KEYS_PER_PAGE,
    //         "root should have exactly {} keys, got {}",
    //         MAX_KEYS_PER_PAGE,
    //         root.nodes.len()
    //     );
    //     assert!(
    //         root.pages.is_none(),
    //         "should still be a leaf at MAX_KEYS_PER_PAGE"
    //     );
    // }

    // #[test]
    // fn test_max_keys_plus_one_triggers_split() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let len = storage.seek(SeekFrom::End(0)).unwrap();
    //     assert!(
    //         len > PAGE_SIZE as u64,
    //         "expected split at MAX_KEYS_PER_PAGE + 1 inserts, len: {}",
    //         len
    //     );
    // }

    // #[test]
    // fn test_single_insert_and_lookup() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     tree.add_node(&mut storage, &[42], &[99])
    //         .expect("insert failed");

    //     let result = tree.get(&[42], &mut storage);
    //     assert!(result.is_some(), "single inserted key not found");
    //     assert_eq!(result.unwrap().as_ref(), &[99], "value mismatch for key 42");
    // }

    // // ── Correctness ─────────────────────────────────────────────────────────────

    // #[test]
    // fn test_reverse_order_insert() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in (0u8..11).rev() {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     for i in 0u8..11 {
    //         let result = tree.get(&[i], &mut storage);
    //         assert!(
    //             result.is_some(),
    //             "key {} not found after reverse order insert",
    //             i
    //         );
    //     }
    // }

    // #[test]
    // fn test_duplicate_key_insert() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     tree.add_node(&mut storage, &[1], &[10])
    //         .expect("insert failed");
    //     tree.add_node(&mut storage, &[1], &[20])
    //         .expect("insert failed");

    //     let result = tree.get(&[1], &mut storage);
    //     assert!(result.is_some(), "key not found after duplicate insert");
    // }

    // #[test]
    // fn test_multiple_splits_all_keys_findable() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     let count = (MAX_KEYS_PER_PAGE * 5) as u8;
    //     for i in 0u8..count {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     for i in 0u8..count {
    //         let result = tree.get(&[i], &mut storage);
    //         assert!(
    //             result.is_some(),
    //             "key {} not found after multiple splits",
    //             i
    //         );
    //     }
    // }

    // #[test]
    // fn test_tree_is_sorted_after_multiple_splits() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     let keys: Vec<u8> = vec![5, 3, 8, 1, 9, 2, 7, 4, 6, 0, 11, 10];
    //     for &k in &keys {
    //         tree.add_node(&mut storage, &[k], &[k])
    //             .expect("insert failed");
    //     }

    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut collected = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut collected);

    //     let mut sorted = collected.clone();
    //     sorted.sort();
    //     assert_eq!(collected, sorted, "tree keys are not in sorted order");
    // }

    // // ── Split structure ──────────────────────────────────────────────────────────

    // #[test]
    // fn test_split_children_key_counts() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let root = tree.root_page_location.load_page(&mut storage);
    //     assert_eq!(
    //         root.nodes.len(),
    //         1,
    //         "root should have 1 promoted key after first split"
    //     );

    //     let children = root.pages.as_ref().expect("root should have children");
    //     assert_eq!(children.len(), 2, "root should have exactly 2 children");

    //     let left = children[0].load_page(&mut storage);
    //     let right = children[1].load_page(&mut storage);

    //     let half = MAX_KEYS_PER_PAGE / 2;
    //     assert_eq!(
    //         left.nodes.len(),
    //         half,
    //         "left child should have {} keys, got {}",
    //         half,
    //         left.nodes.len()
    //     );
    //     assert_eq!(
    //         right.nodes.len(),
    //         half,
    //         "right child should have {} keys, got {}",
    //         half,
    //         right.nodes.len()
    //     );
    // }

    // #[test]
    // fn test_split_no_key_loss_or_duplication() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     let count = MAX_KEYS_PER_PAGE + 1;
    //     for i in 0u8..count as u8 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut all_keys = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut all_keys);

    //     let mut deduped = all_keys.clone();
    //     deduped.dedup();
    //     assert_eq!(all_keys, deduped, "found duplicate keys after split");
    //     assert_eq!(
    //         all_keys.len(),
    //         count,
    //         "expected {} keys in tree, found {}",
    //         count,
    //         all_keys.len()
    //     );
    // }

    // #[test]
    // fn test_page_alignment_maintained_after_splits() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..(MAX_KEYS_PER_PAGE * 3) as u8 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let len = storage.seek(SeekFrom::End(0)).unwrap();
    //     assert_eq!(
    //         len % PAGE_SIZE as u64,
    //         0,
    //         "file is not page-aligned after splits, len: {}",
    //         len
    //     );
    // }

    // // ── Storage integrity ───────────────────────────────────────────────────────

    // #[test]
    // #[should_panic]
    // fn test_root_survives_reconstruction_after_split() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     let tree2 = make_tree();
    //     storage.seek(SeekFrom::Start(0)).unwrap();

    //     for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
    //         let result = tree2.get(&[i], &mut storage);
    //         assert!(result.is_some(), "key {} not found after reconstruction", i);
    //     }
    // }
}
