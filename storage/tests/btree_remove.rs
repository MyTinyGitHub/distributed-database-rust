include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    use storage::btree::page;

    use super::*;
    use std::io::Cursor;

    // // ── Basic remove ───────────────────────────────────────────────────────────────

    use crate::make_tree;

    #[test]
    fn test_remove_existing_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc10 = create_loc(10);
        tree.add(&[1], loc10, &mut storage).expect("insert failed");
        let loc20 = create_loc(20);
        tree.add(&[2], loc20, &mut storage).expect("insert failed");
        let loc30 = create_loc(30);
        tree.add(&[3], loc30, &mut storage).expect("insert failed");

        // Remove the middle key
        tree.remove(&[2], &mut storage);

        // Key should no longer be findable
        assert!(
            tree.get(&[2], &mut storage).is_none(),
            "removed key should not be found"
        );

        // Other keys should still be present
        let r_loc1 = tree.get(&[1], &mut storage).unwrap();
        let r_loc3 = tree.get(&[3], &mut storage).unwrap();
        assert!(is_loc_equal(loc10, r_loc1));
        assert!(is_loc_equal(loc30, r_loc3));
    }

    #[test]
    fn test_remove_non_existent_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc10 = create_loc(10);
        tree.add(&[1], loc10, &mut storage).expect("insert failed");

        // Try to remove a key that doesn't exist
        tree.remove(&[99], &mut storage);

        // Original key should still be there
        let r_loc1 = tree.get(&[1], &mut storage).unwrap();
        assert!(is_loc_equal(loc10, r_loc1));
    }

    #[test]
    fn test_remove_from_single_node_leaf() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc50 = create_loc(50);
        tree.add(&[5], loc50, &mut storage).expect("insert failed");

        tree.remove(&[5], &mut storage);

        assert!(
            tree.get(&[5], &mut storage).is_none(),
            "key should be removed"
        );
    }

    // // ── Remove with tree restructuring ───────────────────────────────────────────

    #[test]
    fn test_remove_triggers_underflow() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        // Fill a page exactly to MAX_KEYS_PER_PAGE, then remove some
        // to trigger underflow (below MAX_KEYS_PER_PAGE/2 = 4)
        for i in 0u8..6 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        // Remove enough keys to cause underflow
        // After: [0, 1, 4, 5] - 4 keys is min threshold
        tree.remove(&[1], &mut storage);
        tree.remove(&[2], &mut storage);
        tree.remove(&[3], &mut storage);

        // Remaining keys should still be findable
        assert!(tree.get(&[0], &mut storage).is_some());
        assert!(tree.get(&[4], &mut storage).is_some());
        assert!(tree.get(&[5], &mut storage).is_some());
    }

    #[test]
    fn test_remove_all_keys() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        for i in 0u8..5 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        // Remove all keys
        for i in 0u8..5 {
            tree.remove(&[i], &mut storage);
        }

        // Tree should be empty
        for i in 0u8..5 {
            assert!(
                tree.get(&[i], &mut storage).is_none(),
                "key {} should be removed",
                i
            );
        }
    }

    #[test]
    fn test_remove_all_keys_backwards() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        for i in 0u8..100 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // Remove all keys
        for i in 0u8..100 {
            println!("removing {}", i);
            tree.root_page_location.load_page(storage).print(storage);
            tree.remove(&[100 - i - 1], &mut storage);
        }

        // Tree should be empty
        for i in 0u8..5 {
            assert!(
                tree.get(&[i], &mut storage).is_none(),
                "key {} should be removed",
                i
            );
        }
    }

    #[test]
    fn test_remove_then_get_returns_none() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc99 = create_loc(99);
        tree.add(&[42], loc99, &mut storage).expect("insert failed");

        assert!(tree.get(&[42], &mut storage).is_some());

        tree.remove(&[42], &mut storage);

        assert!(tree.get(&[42], &mut storage).is_none());
    }

    // // ── Multiple removes ──────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_removes_maintain_order() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        // Insert many keys
        for i in 0u8..20 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        tree.root_page_location.load_page(storage).print(storage);

        // Remove even numbers
        for i in (0u8..20).step_by(2) {
            tree.remove(&[i], storage).unwrap();
            println!("removing {}", i);
            tree.root_page_location.load_page(storage).print(storage);
        }

        // Verify odd keys still exist and are sorted
        check_is_root_sorted(&mut tree, storage);

        // Check only odd keys remain
        for i in 0u8..20 {
            let result = tree.get(&[i], &mut storage);
            if i % 2 == 0 {
                assert!(result.is_none(), "even key {} should be removed", i);
            } else {
                assert!(result.is_some(), "odd key {} should remain", i);
            }
        }
    }

    #[test]
    fn test_remove_first_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc10 = create_loc(10);
        tree.add(&[1], loc10, &mut storage).expect("insert failed");

        let loc20 = create_loc(20);
        tree.add(&[2], loc20, &mut storage).expect("insert failed");

        let loc30 = create_loc(30);
        tree.add(&[3], loc30, &mut storage).expect("insert failed");

        tree.remove(&[1], &mut storage);

        assert!(tree.get(&[1], &mut storage).is_none());

        let r_loc2 = tree.get(&[2], &mut storage).unwrap();
        let r_loc3 = tree.get(&[3], &mut storage).unwrap();

        assert!(is_loc_equal(r_loc2, loc20));
        assert!(is_loc_equal(r_loc3, loc30));
    }

    #[test]
    fn test_remove_last_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc10 = create_loc(10);
        tree.add(&[1], loc10, &mut storage).expect("insert failed");

        let loc20 = create_loc(20);
        tree.add(&[2], loc20, &mut storage).expect("insert failed");

        let loc30 = create_loc(30);
        tree.add(&[3], loc30, &mut storage).expect("insert failed");

        tree.remove(&[3], &mut storage);

        let r_loc1 = tree.get(&[1], &mut storage).unwrap();
        let r_loc2 = tree.get(&[2], &mut storage).unwrap();
        assert!(is_loc_equal(r_loc1, loc10));
        assert!(is_loc_equal(r_loc2, loc20));
        assert!(tree.get(&[3], &mut storage).is_none());
    }

    // // ── Remove after splits ──────────────────────────────────────────────────────

    #[test]
    fn test_remove_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..15 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        tree.root_page_location
            .load_page(&mut storage)
            .print(&mut storage);

        // Remove some keys
        println!("removing 2");
        tree.remove(&[2], &mut storage);

        tree.root_page_location
            .load_page(&mut storage)
            .print(&mut storage);

        println!("removing 13");
        tree.remove(&[13], &mut storage);

        tree.root_page_location
            .load_page(&mut storage)
            .print(&mut storage);

        // Verify remaining keys
        assert!(tree.get(&[2], &mut storage).is_none());
        assert!(tree.get(&[13], &mut storage).is_none());

        for i in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14] {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "key {} should exist",
                i
            );
        }
    }

    // #[test]
    fn test_remove_after_splits() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..100 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        // Remove some keys
        tree.remove(&[0], &mut storage).unwrap();
        tree.remove(&[2], &mut storage).unwrap();
        tree.remove(&[4], &mut storage).unwrap();
        tree.remove(&[6], &mut storage).unwrap();
        tree.remove(&[8], &mut storage).unwrap();
        tree.remove(&[10], &mut storage).unwrap();

        // Verify remaining keys
        assert!(tree.get(&[0], &mut storage).is_none());
        assert!(tree.get(&[2], &mut storage).is_none());
        assert!(tree.get(&[4], &mut storage).is_none());
        assert!(tree.get(&[6], &mut storage).is_none());
        assert!(tree.get(&[8], &mut storage).is_none());
        assert!(tree.get(&[10], &mut storage).is_none());

        for i in [1, 3, 5, 7, 9, 11, 12, 13, 14] {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "key {} should exist",
                i
            );
        }

        let root_page = tree.root_page_location.load_page(&mut storage);
        root_page.print(&mut storage);

        let mut missing = Vec::new();

        for i in 15u8..100 {
            if tree.get(&[i], &mut storage).is_none() {
                missing.push(i);
            }
        }

        assert!(
            missing.len() == 0,
            "keys should be present but weren't {:?}",
            missing
        );
    }

    // // ── Stress tests ──────────────────────────────────────────────────────────────

    #[test]
    fn test_remove_half_of_keys() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        let count = 30;
        for i in 0u8..count {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        tree.root_page_location
            .load_page(&mut storage)
            .print(storage);

        // Remove half
        for i in 0u8..count / 2 {
            tree.remove(&[i], storage);
            println!("removed {}", i);
        }

        tree.root_page_location
            .load_page(storage)
            .print(&mut storage);

        // Check remaining
        for i in 0u8..count {
            let result = tree.get(&[i], storage);
            if i < count / 2 {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                assert!(result.is_some(), "key {} should remain", i);
            }
        }

        check_is_root_sorted(&mut tree, storage);
    }
}
