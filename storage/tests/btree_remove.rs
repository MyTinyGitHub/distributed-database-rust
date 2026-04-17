include!("btree_helper.rs");

#[cfg(test)]
mod tests {

    use super::*;

    // // ── Basic remove ───────────────────────────────────────────────────────────────

    use crate::make_tree;

    #[test]
    fn test_remove_existing_key() {
        let mut tree = make_tree();

        let loc10 = create_loc(10);
        tree.insert(&[1], loc10).expect("insert failed");
        let loc20 = create_loc(20);
        tree.insert(&[2], loc20).expect("insert failed");
        let loc30 = create_loc(30);
        tree.insert(&[3], loc30).expect("insert failed");

        // Remove the middle key
        tree.remove(&[2]).unwrap();

        // Key should no longer be findable
        assert!(tree.get(&[2]).is_none(), "removed key should not be found");

        // Other keys should still be present
        let r_loc1 = tree.get(&[1]).unwrap();
        let r_loc3 = tree.get(&[3]).unwrap();

        assert!(is_loc_equal(loc10, r_loc1));
        assert!(is_loc_equal(loc30, r_loc3));
    }

    #[test]
    fn test_remove_non_existent_key() {
        let mut tree = make_tree();

        let loc10 = create_loc(10);
        tree.insert(&[1], loc10).expect("insert failed");

        // Try to remove a key that doesn't exist
        tree.remove(&[99]).unwrap();

        // Original key should still be there
        let r_loc1 = tree.get(&[1]).unwrap();
        assert!(is_loc_equal(loc10, r_loc1));
    }

    #[test]
    fn test_remove_from_single_node_leaf() {
        let mut tree = make_tree();

        let loc50 = create_loc(50);
        tree.insert(&[5], loc50).expect("insert failed");

        tree.remove(&[5]).unwrap();

        assert!(tree.get(&[5]).is_none(), "key should be removed");
    }

    // // ── Remove with tree restructuring ───────────────────────────────────────────

    #[test]
    fn test_remove_triggers_underflow() {
        let mut tree = make_tree();

        // Fill a page exactly to MAX_KEYS_PER_PAGE, then remove some
        // to trigger underflow (below MAX_KEYS_PER_PAGE/2 = 4)
        for i in 0u8..6 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove enough keys to cause underflow
        // After: [0, 1, 4, 5] - 4 keys is min threshold
        tree.remove(&[1]).unwrap();
        tree.remove(&[2]).unwrap();
        tree.remove(&[3]).unwrap();

        // Remaining keys should still be findable
        assert!(tree.get(&[0]).is_some());
        assert!(tree.get(&[4]).is_some());
        assert!(tree.get(&[5]).is_some());
    }

    #[test]
    fn test_remove_all_keys() {
        let mut tree = make_tree();

        for i in 0u8..5 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove all keys
        for i in 0u8..5 {
            tree.remove(&[i]).unwrap();
        }

        // Tree should be empty
        for i in 0u8..5 {
            assert!(tree.get(&[i]).is_none(), "key {} should be removed", i);
        }
    }

    #[test]
    fn test_remove_all_keys_backwards() {
        let mut tree = make_tree();

        for i in 0u8..100 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove all keys
        for i in 0u8..100 {
            tree.remove(&[100 - i - 1]).unwrap();
        }

        // Tree should be empty
        for i in 0u8..5 {
            assert!(tree.get(&[i]).is_none(), "key {} should be removed", i);
        }
    }

    #[test]
    fn test_remove_then_get_returns_none() {
        let mut tree = make_tree();

        let loc99 = create_loc(99);
        tree.insert(&[42], loc99).expect("insert failed");

        assert!(tree.get(&[42]).is_some());

        tree.remove(&[42]).unwrap();

        assert!(tree.get(&[42]).is_none());
    }

    // // ── Multiple removes ──────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_removes_maintain_order() {
        let mut tree = make_tree();

        // Insert many keys
        for i in 0u8..20 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove even numbers
        for i in (0u8..20).step_by(2) {
            tree.remove(&[i]).unwrap();
        }

        // Verify odd keys still exist and are sorted
        check_is_root_sorted(&mut tree);

        // Check only odd keys remain
        for i in 0u8..20 {
            let result = tree.get(&[i]);
            if i % 2 == 0 {
                assert!(result.is_none(), "even key {} should be removed", i);
            } else {
                assert!(result.is_some(), "odd key {} should remain", i);
            }
        }
    }

    #[test]
    fn test_remove_first_key() {
        let mut tree = make_tree();

        let loc10 = create_loc(10);
        tree.insert(&[1], loc10).expect("insert failed");

        let loc20 = create_loc(20);
        tree.insert(&[2], loc20).expect("insert failed");

        let loc30 = create_loc(30);
        tree.insert(&[3], loc30).expect("insert failed");

        tree.remove(&[1]).unwrap();

        assert!(tree.get(&[1]).is_none());

        let r_loc2 = tree.get(&[2]).unwrap();
        let r_loc3 = tree.get(&[3]).unwrap();

        assert!(is_loc_equal(r_loc2, loc20));
        assert!(is_loc_equal(r_loc3, loc30));
    }

    #[test]
    fn test_remove_last_key() {
        let mut tree = make_tree();

        let loc10 = create_loc(10);
        tree.insert(&[1], loc10).expect("insert failed");

        let loc20 = create_loc(20);
        tree.insert(&[2], loc20).expect("insert failed");

        let loc30 = create_loc(30);
        tree.insert(&[3], loc30).expect("insert failed");

        tree.remove(&[3]).unwrap();

        let r_loc1 = tree.get(&[1]).unwrap();
        let r_loc2 = tree.get(&[2]).unwrap();
        assert!(is_loc_equal(r_loc1, loc10));
        assert!(is_loc_equal(r_loc2, loc20));
        assert!(tree.get(&[3]).is_none());
    }

    // // ── Remove after splits ──────────────────────────────────────────────────────

    #[test]
    fn test_remove_after_split() {
        let mut tree = make_tree();

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..15 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove some keys
        tree.remove(&[2]).unwrap();
        tree.remove(&[13]).unwrap();

        // Verify remaining keys
        assert!(tree.get(&[2]).is_none());
        assert!(tree.get(&[13]).is_none());

        for i in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14] {
            assert!(tree.get(&[i]).is_some(), "key {} should exist", i);
        }
    }

    #[test]
    fn test_remove_after_splits() {
        let mut tree = make_tree();

        // Insert enough to trigger splits (11+ keys)
        for i in 0u8..100 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove some keys
        tree.remove(&[0]).unwrap();
        tree.remove(&[2]).unwrap();
        tree.remove(&[4]).unwrap();
        tree.remove(&[6]).unwrap();
        tree.remove(&[8]).unwrap();
        tree.remove(&[10]).unwrap();

        // Verify remaining keys
        assert!(tree.get(&[0]).is_none());
        assert!(tree.get(&[2]).is_none());
        assert!(tree.get(&[4]).is_none());
        assert!(tree.get(&[6]).is_none());
        assert!(tree.get(&[8]).is_none());
        assert!(tree.get(&[10]).is_none());

        for i in [1, 3, 5, 7, 9, 11, 12, 13, 14] {
            assert!(tree.get(&[i]).is_some(), "key {} should exist", i);
        }

        let mut missing = Vec::new();

        for i in 15u8..100 {
            if tree.get(&[i]).is_none() {
                missing.push(i);
            }
        }

        assert!(
            missing.is_empty(),
            "keys should be present but weren't {:?}",
            missing
        );
    }

    // // ── Stress tests ──────────────────────────────────────────────────────────────

    #[test]
    fn test_remove_half_of_keys() {
        let mut tree = make_tree();

        let count = 30;
        for i in 0u8..count {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        // Remove half
        for i in 0u8..count / 2 {
            tree.remove(&[i]).unwrap();
            println!("removed {}", i);
        }

        // Check remaining
        for i in 0u8..count {
            let result = tree.get(&[i]);
            if i < count / 2 {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                assert!(result.is_some(), "key {} should remain", i);
            }
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Remove middle, then end, then add, then remove again ─────────────────────

    #[test]
    fn test_remove_middle_then_end_then_add_then_remove_again() {
        let mut tree = make_tree();

        // Phase 1: Add initial elements (10x: 100 instead of 10)
        for i in 0u16..100 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Verify all initial keys exist
        for i in 0u16..100 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }

        // Phase 2: Remove from middle (10x: keys 40-59 instead of 4-6)
        for i in 40u16..60 {
            tree.remove(&[i as u8]).unwrap();
        }

        // Middle keys should be gone
        for i in 40u16..60 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "middle key {} should be removed",
                i
            );
        }

        // Other keys should still exist
        for i in 0u16..100 {
            if !(40..60).contains(&i) {
                assert!(
                    tree.get(&[i as u8]).is_some(),
                    "key {} should still exist",
                    i
                );
            }
        }

        // Phase 3: Remove from end (10x: keys 80-99 instead of 8-9)
        for i in 80u16..100 {
            tree.remove(&[i as u8]).unwrap();
        }

        // End keys should be gone
        for i in 80u16..100 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "end key {} should be removed",
                i
            );
        }

        // Phase 4: Add new elements (10x: 200-249 instead of 100-104)
        for i in 200u16..250 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // New keys should exist
        for i in 200u16..250 {
            assert!(tree.get(&[i as u8]).is_some(), "new key {} should exist", i);
        }

        // Phase 5: Remove again (10x: mixed)
        // from beginning: 0-9
        for i in 0u16..10 {
            tree.remove(&[i as u8]).unwrap();
        }
        // from newly added: 200-209
        for i in 200u16..210 {
            tree.remove(&[i as u8]).unwrap();
        }
        // from middle of original: 20-29
        for i in 20u16..30 {
            tree.remove(&[i as u8]).unwrap();
        }

        // These should be gone
        for i in 0u16..10 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 200u16..210 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 20u16..30 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }

        // These should remain
        for i in 10u16..20 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should remain", i);
        }
        for i in 30u16..40 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should remain", i);
        }
        for i in 60u16..80 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should remain", i);
        }
        for i in 210u16..250 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should remain", i);
        }

        check_is_root_sorted(&mut tree);
    }

    #[test]
    fn test_remove_end_then_middle_then_add_then_remove_again() {
        let mut tree = make_tree();

        // Phase 1: Add initial elements (10x: 120 instead of 12)
        for i in 0u16..120 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 2: Remove from end (10x: keys 100-119 instead of 10-11)
        for i in 100u16..120 {
            tree.remove(&[i as u8]).unwrap();
        }

        for i in 100u16..120 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "end key {} should be removed",
                i
            );
        }

        // Phase 3: Remove from middle (10x: keys 40-59 instead of 4-6)
        for i in 40u16..60 {
            tree.remove(&[i as u8]).unwrap();
        }

        for i in 40u16..60 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "middle key {} should be removed",
                i
            );
        }

        // Phase 4: Add new elements after removals (10x: 200-249 instead of 50-54)
        for i in 200u16..250 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 5: Remove from new elements (10x: every other from 200-249)
        for i in (200u16..250).step_by(2) {
            tree.remove(&[i as u8]).unwrap();
        }

        for i in (200u16..250).step_by(2) {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "new key {} should be removed",
                i
            );
        }

        // Verify expected remaining keys
        // Original keys that should remain: 0-39, 60-99
        for i in 0u16..40 {
            assert!(
                tree.get(&[i as u8]).is_some(),
                "key {} should be present",
                i
            );
        }
        for i in 60u16..100 {
            assert!(
                tree.get(&[i as u8]).is_some(),
                "key {} should be present",
                i
            );
        }
        // New keys that should remain: odd ones from 200-249
        for i in (201u16..250).step_by(2) {
            assert!(
                tree.get(&[i as u8]).is_some(),
                "new key {} should be present",
                i
            );
        }

        // Verify removed keys
        for i in 40u16..60 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 100u16..120 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in (200u16..250).step_by(2) {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "new key {} should be removed",
                i
            );
        }

        check_is_root_sorted(&mut tree);
    }

    #[test]
    fn test_remove_all_except_one_then_add_then_remove_remaining() {
        let mut tree = make_tree();

        // Phase 1: Add elements (10x: 80 instead of 8)
        for i in 0u16..80 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 2: Remove all except one (key 30-39 instead of key 3)
        for i in 0u16..80 {
            if !(30..40).contains(&i) {
                tree.remove(&[i as u8]).unwrap();
            }
        }

        // Keys 30-39 should remain
        for i in 30u16..40 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should remain", i);
        }
        for i in 0u16..80 {
            if !(30..40).contains(&i) {
                assert!(
                    tree.get(&[i as u8]).is_none(),
                    "key {} should be removed",
                    i
                );
            }
        }

        // Phase 3: Add new elements (10x: 200-249 instead of 20-24)
        for i in 200u16..250 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 4: Remove the last remaining original keys
        for i in 30u16..40 {
            tree.remove(&[i as u8]).unwrap();
        }
        for i in 30u16..40 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }

        // Only new keys should remain
        for i in 200u16..250 {
            assert!(tree.get(&[i as u8]).is_some(), "new key {} should exist", i);
        }

        check_is_root_sorted(&mut tree);
    }

    #[test]
    fn test_remove_middle_elements_interleaved_with_adds() {
        let mut tree = make_tree();

        // Phase 1: Add initial set (10x: 60 instead of 6)
        for i in 0u16..60 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Remove from middle (10x: keys 20-39 instead of 2-3)
        for i in 20u16..40 {
            tree.remove(&[i as u8]).unwrap();
        }

        // Add more (10x: 100-139 instead of 10-13)
        for i in 100u16..140 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Remove from middle again (10x: keys 110-119 instead of 11)
        for i in 110u16..120 {
            tree.remove(&[i as u8]).unwrap();
        }

        // Add more (10x: 200-219 instead of 20-21)
        for i in 200u16..220 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Verify final state - removed keys
        for i in 20u16..40 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 110u16..120 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }

        // Verify final state - remaining keys
        for i in 0u16..20 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 40u16..60 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 100u16..110 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 120u16..140 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 200u16..220 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }

        check_is_root_sorted(&mut tree);
    }

    #[test]
    fn test_remove_end_then_add_until_split_then_remove_again() {
        let mut tree = make_tree();

        // Phase 1: Add initial elements (10x: 50 instead of 5)
        for i in 0u16..50 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 2: Remove from end (10x: keys 30-49 instead of 3-4)
        for i in 30u16..50 {
            tree.remove(&[i as u8]).unwrap();
        }

        // Phase 3: Add many new elements to trigger splits (10x: 150-249 instead of 50-79)
        for i in 150u16..250 {
            let loc = create_loc(i as usize);
            tree.insert(&[i as u8], loc).expect("insert failed");
        }

        // Phase 4: Remove from various positions in the split tree (10x)
        // from beginning: 0-9
        for i in 0u16..10 {
            tree.remove(&[i as u8]).unwrap();
        }
        // from middle of new range: 190-199
        for i in 190u16..200 {
            tree.remove(&[i as u8]).unwrap();
        }
        // from end of new range: 240-249
        for i in 240u16..250 {
            tree.remove(&[i as u8]).unwrap();
        }

        // Verify removals
        for i in 0u16..10 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 190u16..200 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }
        for i in 240u16..250 {
            assert!(
                tree.get(&[i as u8]).is_none(),
                "key {} should be removed",
                i
            );
        }

        // Verify some still exist
        for i in 10u16..30 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 150u16..190 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }
        for i in 200u16..240 {
            assert!(tree.get(&[i as u8]).is_some(), "key {} should exist", i);
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Empty tree ────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_tree_get_returns_none() {
        let mut tree = make_tree();

        assert!(tree.get(&[42]).is_none());
    }

    // // ── Value correctness ─────────────────────────────────────────────────────────

    #[test]
    fn test_value_correctness_after_removes() {
        // Most stress tests only check key presence. This test verifies that the
        // Location stored for each key is actually the one that was inserted.
        let mut tree = make_tree();

        for i in 0u8..60 {
            tree.insert(&[i], create_loc(i as usize))
                .expect("insert failed");
        }

        // Remove every third key.
        for i in (0u8..60).step_by(3) {
            tree.remove(&[i]).unwrap();
        }

        for i in 0u8..60 {
            let result = tree.get(&[i]);
            if i % 3 == 0 {
                assert!(result.is_none(), "key {} should be removed", i);
            } else {
                let loc = result.unwrap_or_else(|| panic!("key {} should exist", i));
                assert!(
                    is_loc_equal(loc, create_loc(i as usize)),
                    "key {} returned wrong location",
                    i
                );
            }
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Re-insert after remove ────────────────────────────────────────────────────

    #[test]
    fn test_reinsert_after_remove() {
        let mut tree = make_tree();

        for i in 0u8..30 {
            tree.insert(&[i], create_loc(i as usize))
                .expect("insert failed");
        }

        // Remove a spread of keys, then re-add them with different location values.
        let targets = [5u8, 10, 15, 20, 25];
        for &i in &targets {
            tree.remove(&[i]).unwrap();
            assert!(
                tree.get(&[i]).is_none(),
                "key {} should be absent after remove",
                i
            );
        }

        for &i in &targets {
            tree.insert(&[i], create_loc(i as usize + 100))
                .expect("re-insert failed");
        }

        for &i in &targets {
            let loc = tree
                .get(&[i])
                .unwrap_or_else(|| panic!("re-inserted key {} should exist", i));
            assert!(
                is_loc_equal(loc, create_loc(i as usize + 100)),
                "key {} should return the new location after re-insert",
                i
            );
        }

        // Keys that were never removed still carry their original values.
        for i in 0u8..30 {
            if !targets.contains(&i) {
                let loc = tree
                    .get(&[i])
                    .unwrap_or_else(|| panic!("untouched key {} should exist", i));
                assert!(
                    is_loc_equal(loc, create_loc(i as usize)),
                    "untouched key {} should still have its original location",
                    i
                );
            }
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Separator key removal ─────────────────────────────────────────────────────

    #[test]
    fn test_remove_separator_key() {
        // With MAX_KEYS_PER_PAGE=9 and a leaf split at index MAX/2=4, inserting keys
        // 0..15 produces an internal root with separators [4, 8]:
        //   leaf [0,1,2,3]  |4|  leaf [4,5,6,7]  |8|  leaf [8,9,10,11,12,13,14]
        // Removing keys 4 and 8 — which are stored both in their leaf and as internal
        // separators — verifies that stale separators do not corrupt routing.
        let mut tree = make_tree();

        for i in 0u8..15 {
            tree.insert(&[i], create_loc(i as usize))
                .expect("insert failed");
        }

        tree.remove(&[4]).unwrap();
        tree.remove(&[8]).unwrap();

        assert!(
            tree.get(&[4]).is_none(),
            "separator key 4 should be removed"
        );
        assert!(
            tree.get(&[8]).is_none(),
            "separator key 8 should be removed"
        );

        for i in 0u8..15 {
            if i == 4 || i == 8 {
                continue;
            }
            let loc = tree.get(&[i]).unwrap_or_else(|| {
                panic!("key {} should still be findable after separator removal", i)
            });
            assert!(
                is_loc_equal(loc, create_loc(i as usize)),
                "key {} returned wrong location",
                i
            );
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Root height collapse ──────────────────────────────────────────────────────

    #[test]
    fn test_root_height_collapse() {
        // Build a 3-level tree then remove enough keys to merge internal nodes all
        // the way up so the root collapses to a shallower height.
        let mut tree = make_tree();

        for i in 0u8..100 {
            tree.insert(&[i], create_loc(i as usize))
                .expect("insert failed");
        }

        for i in 10u8..100 {
            tree.remove(&[i]).unwrap();
        }

        for i in 0u8..10 {
            let loc = tree
                .get(&[i])
                .unwrap_or_else(|| panic!("key {} should exist after collapse", i));
            assert!(
                is_loc_equal(loc, create_loc(i as usize)),
                "key {} has wrong location after root collapse",
                i
            );
        }

        check_is_root_sorted(&mut tree);
    }

    // // ── Random removal order ──────────────────────────────────────────────────────

    #[test]
    fn test_remove_random_order() {
        // Deterministic XorShift shuffle exercises borrow/merge paths that purely
        // sequential removal orders miss.
        let mut tree = make_tree();

        let count: u8 = 100;
        for i in 0..count {
            tree.insert(&[i], create_loc(i as usize))
                .expect("insert failed");
        }

        let mut keys: Vec<u8> = (0..count).collect();
        let mut state: u32 = 0xdeadbeef;
        for i in (1..keys.len()).rev() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let j = (state as usize) % (i + 1);
            keys.swap(i, j);
        }

        for &key in &keys {
            tree.remove(&[key]).unwrap();
        }

        for i in 0..count {
            assert!(tree.get(&[i]).is_none(), "key {} should be removed", i);
        }
    }

    // // ── Multi-byte keys ───────────────────────────────────────────────────────────

    #[test]
    fn test_multi_byte_keys() {
        // All previous tests use 1-byte keys. This test uses 2-byte keys to verify
        // that lexicographic ordering is handled correctly end-to-end.
        // Lex order: [0,255] < [1,0] — first byte dominates, so this matches
        // big-endian u16 ordering and is a natural extension of single-byte tests.
        let mut tree = make_tree();

        // 30 two-byte keys: high byte from 0..3, low byte from 0..10.
        for high in 0u8..3 {
            for low in 0u8..10 {
                let key = [high, low];
                let idx = (high as usize) * 10 + (low as usize);
                tree.insert(key.as_slice(), create_loc(idx))
                    .expect("insert failed");
            }
        }

        // All keys should be retrievable with correct locations.
        for high in 0u8..3 {
            for low in 0u8..10 {
                let key = [high, low];
                let idx = (high as usize) * 10 + (low as usize);
                let loc = tree
                    .get(key.as_slice())
                    .unwrap_or_else(|| panic!("key {:?} should exist", key));
                assert!(
                    is_loc_equal(loc, create_loc(idx)),
                    "key {:?} returned wrong location",
                    key
                );
            }
        }

        // Remove one key per high-byte group and verify ordering is not disturbed.
        tree.remove(&[0, 5]).unwrap();
        tree.remove(&[1, 0]).unwrap();
        tree.remove(&[2, 9]).unwrap();

        assert!(tree.get(&[0, 5]).is_none());
        assert!(tree.get(&[1, 0]).is_none());
        assert!(tree.get(&[2, 9]).is_none());

        // Neighbours of the removed keys should be unaffected.
        assert!(tree.get(&[0, 4]).is_some());
        assert!(tree.get(&[0, 6]).is_some());
        assert!(tree.get(&[1, 1]).is_some());
        assert!(tree.get(&[2, 8]).is_some());

        check_is_root_sorted(&mut tree);
    }
}
