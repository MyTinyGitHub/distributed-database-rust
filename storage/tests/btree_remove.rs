include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    // ── Basic remove ───────────────────────────────────────────────────────────────

    #[test]
    fn test_remove_existing_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");
        tree.add_node(&mut storage, &[2], &[20])
            .expect("insert failed");
        tree.add_node(&mut storage, &[3], &[30])
            .expect("insert failed");

        // Remove the middle key
        tree.remove(&[2], &mut storage);

        // Key should no longer be findable
        assert!(
            tree.get(&[2], &mut storage).is_none(),
            "removed key should not be found"
        );

        // Other keys should still be present
        assert_eq!(tree.get(&[1], &mut storage).unwrap().as_ref(), &[10]);
        assert_eq!(tree.get(&[3], &mut storage).unwrap().as_ref(), &[30]);
    }

    #[test]
    fn test_remove_non_existent_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");

        // Try to remove a key that doesn't exist
        tree.remove(&[99], &mut storage);

        // Original key should still be there
        assert_eq!(tree.get(&[1], &mut storage).unwrap().as_ref(), &[10]);
    }

    #[test]
    fn test_remove_from_single_node_leaf() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[5], &[50])
            .expect("insert failed");

        tree.remove(&[5], &mut storage);

        assert!(
            tree.get(&[5], &mut storage).is_none(),
            "key should be removed"
        );
    }

    // ── Remove with tree restructuring ───────────────────────────────────────────

    #[test]
    fn test_remove_triggers_underflow() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // Fill a page exactly to MAX_KEYS_PER_PAGE, then remove some
        // to trigger underflow (below MAX_KEYS_PER_PAGE/2 = 4)
        for i in 0u8..6 {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
        }

        // Remove enough keys to cause underflow
        // After: [0, 1, 4, 5] - 4 keys is min threshold
        tree.remove(&[2], &mut storage);
        tree.remove(&[3], &mut storage);

        // Remaining keys should still be findable
        assert!(tree.get(&[0], &mut storage).is_some());
        assert!(tree.get(&[1], &mut storage).is_some());
        assert!(tree.get(&[4], &mut storage).is_some());
        assert!(tree.get(&[5], &mut storage).is_some());
    }

    #[test]
    fn test_remove_all_keys() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        for i in 0u8..5 {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
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
    fn test_remove_then_get_returns_none() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[42], &[99])
            .expect("insert failed");

        assert!(tree.get(&[42], &mut storage).is_some());

        tree.remove(&[42], &mut storage);

        assert!(tree.get(&[42], &mut storage).is_none());
    }

    // ── Multiple removes ──────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_removes_maintain_order() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // Insert many keys
        for i in 0u8..20 {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
        }

        // Remove even numbers
        for i in (0u8..20).step_by(2) {
            tree.remove(&[i], &mut storage);
        }

        // Verify odd keys still exist and are sorted
        let root = tree.root_page_location.load_page(&mut storage);
        let mut collected = Vec::new();
        collect_keys_in_order(&mut storage, &root, &mut collected);

        let mut sorted = collected.clone();
        sorted.sort();
        assert_eq!(collected, sorted, "remaining keys should be sorted");

        println!("{:?}", sorted);

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
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");
        tree.add_node(&mut storage, &[2], &[20])
            .expect("insert failed");
        tree.add_node(&mut storage, &[3], &[30])
            .expect("insert failed");

        tree.remove(&[1], &mut storage);

        assert!(tree.get(&[1], &mut storage).is_none());
        assert_eq!(tree.get(&[2], &mut storage).unwrap().as_ref(), &[20]);
        assert_eq!(tree.get(&[3], &mut storage).unwrap().as_ref(), &[30]);
    }

    #[test]
    fn test_remove_last_key() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");
        tree.add_node(&mut storage, &[2], &[20])
            .expect("insert failed");
        tree.add_node(&mut storage, &[3], &[30])
            .expect("insert failed");

        tree.remove(&[3], &mut storage);

        assert_eq!(tree.get(&[1], &mut storage).unwrap().as_ref(), &[10]);
        assert_eq!(tree.get(&[2], &mut storage).unwrap().as_ref(), &[20]);
        assert!(tree.get(&[3], &mut storage).is_none());
    }

    // ── Remove after splits ──────────────────────────────────────────────────────

    #[test]
    fn test_remove_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..15 {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
        }

        // Remove some keys
        tree.remove(&[2], &mut storage);
        tree.remove(&[13], &mut storage);

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

    #[test]
    fn test_remove_after_splits() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..100 {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
        }

        // Remove some keys
        tree.remove(&[0], &mut storage);
        tree.remove(&[2], &mut storage);
        tree.remove(&[4], &mut storage);
        tree.remove(&[6], &mut storage);
        tree.remove(&[8], &mut storage);
        tree.remove(&[10], &mut storage);

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

        for i in 15u8..100 {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "key {} should exist",
                i
            );
        }
    }

    // ── Stress tests ──────────────────────────────────────────────────────────────

    #[test]
    fn test_remove_half_of_keys() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        let count = 30;
        for i in 0u8..count {
            tree.add_node(&mut storage, &[i], &[i])
                .expect("insert failed");
        }

        // Remove half
        for i in 0u8..count / 2 {
            tree.remove(&[i], &mut storage);
        }

        // Check remaining
        for i in 0u8..count {
            let result = tree.get(&[i], &mut storage);
            if i < count / 2 {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                assert!(result.is_some(), "key {} should remain", i);
            }
        }

        // Verify order
        let root = tree.root_page_location.load_page(&mut storage);
        let mut collected = Vec::new();
        collect_keys_in_order(&mut storage, &root, &mut collected);
        let mut sorted = collected.clone();
        sorted.sort();
        assert_eq!(collected, sorted);
    }
}
