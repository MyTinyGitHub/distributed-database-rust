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

    use crate::{
        check_is_root_sorted, create_loc, is_loc_equal, make_tree, make_tree_from_stroage,
        MAX_KEYS_PER_PAGE, PAGE_SIZE,
    };

    #[test]
    fn test_write_read_roundtrip() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);

        let page = Page::Leaf(Leaf {
            keys: vec![Box::new([1u8])],
            values: vec![create_loc(1usize)],
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
        let mut tree = make_tree();

        for i in 0u8..6 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let len = storage.seek(SeekFrom::End(0)).unwrap();

        assert!(
            len == PAGE_SIZE as u64,
            "expected no split at 6 inserts but file grew, len: {}",
            len
        );

        let root = tree.root_page_location.load_page(storage);
        assert_eq!(
            root.size(),
            6,
            "root should have 6 keys, got {}",
            root.size()
        );
    }

    #[test]
    fn test_split_on_eleventh_insert() {
        let mut tree = make_tree();

        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len > PAGE_SIZE as u64,
            "expected split but file is still one page len: {}",
            len
        );

        let root = tree.root_page_location.load_page(storage);
        assert_eq!(
            root.size(),
            1,
            "root should have exactly one promoted key after first split"
        );
    }

    #[test]
    fn test_all_keys_findable_after_split() {
        let mut tree = make_tree();

        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        for i in 0u8..11 {
            let result = tree.get(&[i]);
            assert!(result.is_some(), "key {} not found after split", i);
        }
    }

    #[test]
    fn test_exactly_max_keys_no_split() {
        let mut tree = make_tree();

        for i in 0u8..MAX_KEYS_PER_PAGE as u8 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let len = storage.seek(SeekFrom::End(0)).unwrap();

        assert!(
            len == (PAGE_SIZE * 3) as u64,
            "expected no split at exactly MAX_KEYS_PER_PAGE inserts, len: {}",
            len
        );

        let root = tree.root_page_location.load_page(storage);
        assert_eq!(
            root.size(),
            1,
            "root should have exactly {} keys, got {}",
            1,
            root.size()
        );
    }

    #[test]
    fn test_max_keys_plus_one_triggers_split() {
        let mut tree = make_tree();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert!(
            len > PAGE_SIZE as u64,
            "expected split at MAX_KEYS_PER_PAGE + 1 inserts, len: {}",
            len
        );
    }

    #[test]
    fn test_single_insert_and_lookup() {
        let mut tree = make_tree();

        let loc99 = create_loc(99);
        tree.insert(&[42], loc99).expect("insert failed");

        let result = tree.get(&[42]);
        assert!(result.is_some(), "single inserted key not found");
        assert!(
            is_loc_equal(result.unwrap(), loc99),
            "value mismatch for key 42"
        );
    }

    // // ── Correctness ─────────────────────────────────────────────────────────────

    #[test]
    fn test_reverse_order_insert() {
        let mut tree = make_tree();

        for i in (0u8..11).rev() {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        for i in 0u8..11 {
            let result = tree.get(&[i]);
            assert!(
                result.is_some(),
                "key {} not found after reverse order insert",
                i
            );
        }
    }

    #[test]
    fn test_duplicate_key_insert() {
        let mut tree = make_tree();

        let loc10 = create_loc(10);
        tree.insert(&[1], loc10).expect("insert failed");

        let loc20 = create_loc(20);
        tree.insert(&[1], loc20).expect("insert failed");

        let result = tree.get(&[1]);
        assert!(result.is_some(), "key not found after duplicate insert");
    }

    #[test]
    fn test_multiple_splits_all_keys_findable() {
        let mut tree = make_tree();

        let count = (MAX_KEYS_PER_PAGE * 10) as u8;
        for i in 0u8..count {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        for i in 0u8..count {
            let loc = create_loc(i as usize);
            let result = tree.get(&[i]);
            assert!(
                is_loc_equal(loc, result.unwrap()),
                "key {} not found after multiple splits",
                i
            );
        }
    }

    #[test]
    fn test_tree_is_sorted_after_multiple_splits() {
        let mut tree = make_tree();

        let keys: Vec<u8> = vec![5, 3, 8, 1, 9, 2, 7, 4, 6, 0, 11, 10];
        for &k in &keys {
            let loc = create_loc(k as usize);
            tree.insert(&[k], loc).expect("insert failed");
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Split structure ──────────────────────────────────────────────────────────

    #[test]
    fn test_split_children_key_counts() {
        let mut tree = make_tree();

        for i in 0u8..(MAX_KEYS_PER_PAGE as u8) {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let root = tree.root_page_location.load_page(storage);
        assert_eq!(
            root.size(),
            1,
            "root should have 1 promoted key after first split"
        );

        match root {
            Page::Leaf(_) => panic!(),
            Page::Internal(i) => {
                let children = i.pages;
                assert_eq!(children.len(), 2, "root should have exactly 2 children");

                let left = children[0].load_page(storage);
                let right = children[1].load_page(storage);

                let half = MAX_KEYS_PER_PAGE / 2;
                assert_eq!(
                    left.size(),
                    half - 1,
                    "left child should have {} keys, got {}",
                    half,
                    left.size()
                );
                assert_eq!(
                    right.size(),
                    half + 1,
                    "right child should have {} keys, got {}",
                    half,
                    right.size()
                );
            }
        }
    }

    #[test]
    fn test_split_no_key_loss_or_duplication() {
        let mut tree = make_tree();

        let count = MAX_KEYS_PER_PAGE + 1;
        for i in 0u8..count as u8 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        check_is_root_sorted(&mut tree);
    }

    #[test]
    fn test_page_alignment_maintained_after_splits() {
        let mut tree = make_tree();

        for i in 0u8..(MAX_KEYS_PER_PAGE * 3) as u8 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let storage = &mut tree.storage;
        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(
            len % PAGE_SIZE as u64,
            0,
            "file is not page-aligned after splits, len: {}",
            len
        );
    }

    // // ── Storage integrity ───────────────────────────────────────────────────────

    #[test]
    fn test_root_survives_reconstruction_after_split() {
        let mut tree = make_tree();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        let mut storage = tree.storage.clone();
        let mut tree2 = make_tree_from_stroage(tree.storage);
        storage.seek(SeekFrom::Start(0)).unwrap();

        for i in 0u8..=(MAX_KEYS_PER_PAGE as u8) {
            let result = tree2.get(&[i]);
            assert!(result.is_some(), "key {} not found after reconstruction", i);
        }
    }
}
