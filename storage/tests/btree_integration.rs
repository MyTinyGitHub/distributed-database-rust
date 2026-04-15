include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    // use super::*;
    // use std::io::Cursor;

    // // ── Insert → Remove → Re-insert ──────────────────────────────────────────────

    use std::{io::Cursor, ops::Add};

    use storage::btree::btree::MAX_KEYS_PER_PAGE;

    use crate::{check_is_root_sorted, create_loc, is_loc_equal, make_tree, PAGE_SIZE};

    #[test]
    fn test_reinsert_after_remove() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc10 = create_loc(10);
        tree.add(&[1], loc10, storage).expect("insert failed");
        let loc20 = create_loc(10);
        tree.add(&[2], loc20, storage).expect("insert failed");
        let loc30 = create_loc(30);
        tree.add(&[3], loc30, storage).expect("insert failed");

        tree.remove(&[2], &mut storage);
        assert!(
            tree.get(&[2], &mut storage).is_none(),
            "key 2 should be gone"
        );

        // re-insert with a different value
        let loc99 = create_loc(99);
        tree.add(&[2], loc99, storage).expect("reinsert failed");

        assert!(
            is_loc_equal(tree.get(&[2], &mut storage).unwrap(), loc99),
            "re-inserted key should have new value"
        );

        assert!(is_loc_equal(tree.get(&[1], &mut storage).unwrap(), loc10));
        assert!(is_loc_equal(tree.get(&[3], &mut storage).unwrap(), loc30));
    }

    #[test]
    fn test_reinsert_after_remove_with_splits() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        // fill enough to trigger splits
        for i in 0u8..20 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // remove half
        for i in 0u8..10 {
            tree.remove(&[i], &mut storage);
        }

        // re-insert with offset values
        for i in 0u8..10 {
            let loc = create_loc(i.add(50) as usize);
            tree.add(&[i], loc, storage).expect("reinsert failed");
        }

        // verify all 20 keys are present with correct values
        for i in 0u8..10 {
            let loc = create_loc(i.add(50) as usize);

            assert!(
                is_loc_equal(tree.get(&[i], &mut storage).unwrap(), loc),
                "re-inserted key {} should have value {}",
                i,
                i + 50
            );
        }
        for i in 10u8..20 {
            let loc = create_loc(i as usize);
            assert!(
                is_loc_equal(tree.get(&[i], &mut storage).unwrap(), loc),
                "original key {} should still have original value",
                i
            );
        }
    }

    // // ── Tree grows after shrinking ────────────────────────────────────────────────

    #[test]
    fn test_insert_after_mass_remove() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        for i in 0u8..30 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // remove almost everything
        for i in 0u8..25 {
            tree.remove(&[i], &mut storage);
        }

        // now grow again past original size
        for i in 30u8..60 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage)
                .expect("insert after shrink failed");
        }

        // verify the 5 originals survived
        for i in 25u8..30 {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "surviving key {} should still exist",
                i
            );
        }

        // verify new inserts are all present
        for i in 30u8..60 {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "new key {} should exist after regrowth",
                i
            );
        }

        // verify removed keys are gone
        for i in 0u8..25 {
            assert!(
                tree.get(&[i], &mut storage).is_none(),
                "removed key {} should not exist",
                i
            );
        }
    }

    // // ── Value correctness after removes ──────────────────────────────────────────

    #[test]
    fn test_values_correct_after_rebalance() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        // use key * 3 as value so we can verify independently
        for i in 0u8..20 {
            let loc = create_loc((i * 3) as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // remove every third key
        for i in (0u8..20).step_by(3) {
            tree.remove(&[i], &mut storage);
        }

        // verify remaining keys have correct values
        for i in 0u8..20 {
            if i % 3 == 0 {
                assert!(
                    tree.get(&[i], &mut storage).is_none(),
                    "key {} should be gone",
                    i
                );
            } else {
                let loc = create_loc((i * 3) as usize);
                assert!(
                    is_loc_equal(tree.get(&[i], &mut storage).unwrap(), loc),
                    "key {} should have value {}, rebalance corrupted values",
                    i,
                    i * 3
                );
            }
        }
    }

    // // ── Remove separator keys specifically ───────────────────────────────────────

    #[test]
    fn test_remove_separator_key() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        // insert exactly enough to split — the promoted key is the separator
        for i in 0u8..=MAX_KEYS_PER_PAGE as u8 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // the root has exactly 1 separator key — find and remove it
        let root = tree.root_page_location.load_page(&mut storage);
        assert_eq!(root.size(), 1, "root should have 1 separator");

        let separator_key = root.peek_first().clone();

        tree.remove(&separator_key, &mut storage);

        // all other keys should still be findable
        for i in 0u8..=MAX_KEYS_PER_PAGE as u8 {
            if i != separator_key[0] {
                assert!(
                    tree.get(&[i], &mut storage).is_some(),
                    "key {} should survive separator removal",
                    i
                );
            }
        }
        // verify order is still maintained
    }

    // ── Random order operations ───────────────────────────────────────────────────

    #[test]
    fn test_shuffled_remove_order() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        for i in 0u8..20 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // remove in non-sequential order
        let remove_order: Vec<u8> = vec![15, 3, 18, 0, 7, 11, 4, 19, 1, 9];
        for &k in &remove_order {
            tree.remove(&[k], &mut storage);
        }

        // verify removed keys are gone
        for &k in &remove_order {
            assert!(
                tree.get(&[k], storage).is_none(),
                "shuffled-removed key {} should be gone",
                k
            );
        }

        check_is_root_sorted(&mut tree, storage);

        // verify exact remaining set
        let removed_set: std::collections::HashSet<u8> = remove_order.iter().cloned().collect();
        for i in 0u8..20 {
            let result = tree.get(&[i], &mut storage);
            if removed_set.contains(&i) {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                assert!(result.is_some(), "key {} should remain", i);
            }
        }
    }

    // // ── Interleaved inserts and removes ──────────────────────────────────────────

    #[test]
    fn test_interleaved_insert_remove() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        // interleave inserts and removes in batches
        for i in 0u8..15 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        for i in 0u8..5 {
            tree.remove(&[i], &mut storage);
        }

        for i in 15u8..30 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        for i in 5u8..10 {
            tree.remove(&[i], &mut storage);
        }

        for i in 30u8..45 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // verify final state: 0-9 removed, 10-44 present
        for i in 0u8..10 {
            assert!(
                tree.get(&[i], &mut storage).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 10u8..45 {
            assert!(
                tree.get(&[i], &mut storage).is_some(),
                "key {} should exist",
                i
            );
        }

        // verify sorted order
        check_is_root_sorted(&mut tree, storage);
    }

    // // ── Page alignment invariant ─────────────────────────────────────────────────

    #[test]
    fn test_page_alignment_after_removes() {
        use std::io::{Seek, SeekFrom};

        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        for i in 0u8..50 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        for i in 0u8..25 {
            tree.remove(&[i], &mut storage);
        }

        let len = storage.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(
            len % PAGE_SIZE as u64,
            0,
            "file not page-aligned after removes, len: {}",
            len
        );
    }

    // // ── Full cycle stress ─────────────────────────────────────────────────────────

    #[test]
    fn test_full_cycle_insert_remove_verify() {
        let mut storage = &mut Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(storage);

        let total = (MAX_KEYS_PER_PAGE * 8) as u8;

        // insert all
        for i in 0u8..total {
            let loc = create_loc(i.wrapping_mul(2) as usize);
            tree.add(&[i], loc, storage).expect("insert failed");
        }

        // remove first third
        for i in 0u8..total / 3 {
            tree.remove(&[i], storage).unwrap();
            tree.root_page_location.load_page(storage).print(storage);
        }

        tree.root_page_location.load_page(storage).print(storage);

        // remove last third
        for i in (total * 2 / 3)..total {
            tree.remove(&[i], storage).unwrap();
            tree.root_page_location.load_page(storage).print(storage);
        }

        // only middle third should remain

        tree.root_page_location.load_page(storage).print(storage);
        for i in 0u8..total {
            let result = tree.get(&[i], &mut storage);
            if i < total / 3 || i >= total * 2 / 3 {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                let l_loc = result.unwrap();
                let r_loc = create_loc(i.wrapping_mul(2) as usize);
                assert!(
                    is_loc_equal(l_loc, r_loc),
                    "key {} has wrong value after full cycle",
                    i
                );
            }
        }

        // final order check
        check_is_root_sorted(&mut tree, storage);
    }
}
