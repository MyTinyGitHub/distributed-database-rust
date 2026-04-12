include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    // use super::*;
    // use std::io::Cursor;

    // // ── Insert → Remove → Re-insert ──────────────────────────────────────────────

    // #[test]
    // fn test_reinsert_after_remove() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     tree.add_node(&mut storage, &[1], &[10])
    //         .expect("insert failed");
    //     tree.add_node(&mut storage, &[2], &[20])
    //         .expect("insert failed");
    //     tree.add_node(&mut storage, &[3], &[30])
    //         .expect("insert failed");

    //     tree.remove(&[2], &mut storage);
    //     assert!(
    //         tree.get(&[2], &mut storage).is_none(),
    //         "key 2 should be gone"
    //     );

    //     // re-insert with a different value
    //     tree.add_node(&mut storage, &[2], &[99])
    //         .expect("reinsert failed");

    //     assert_eq!(
    //         tree.get(&[2], &mut storage).unwrap().as_ref(),
    //         &[99],
    //         "re-inserted key should have new value"
    //     );
    //     assert_eq!(tree.get(&[1], &mut storage).unwrap().as_ref(), &[10]);
    //     assert_eq!(tree.get(&[3], &mut storage).unwrap().as_ref(), &[30]);
    // }

    // #[test]
    // fn test_reinsert_after_remove_with_splits() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     // fill enough to trigger splits
    //     for i in 0u8..20 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     // remove half
    //     for i in 0u8..10 {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     // re-insert with offset values
    //     for i in 0u8..10 {
    //         tree.add_node(&mut storage, &[i], &[i + 50])
    //             .expect("reinsert failed");
    //     }

    //     // verify all 20 keys are present with correct values
    //     for i in 0u8..10 {
    //         assert_eq!(
    //             tree.get(&[i], &mut storage).unwrap().as_ref(),
    //             &[i + 50],
    //             "re-inserted key {} should have value {}",
    //             i,
    //             i + 50
    //         );
    //     }
    //     for i in 10u8..20 {
    //         assert_eq!(
    //             tree.get(&[i], &mut storage).unwrap().as_ref(),
    //             &[i],
    //             "original key {} should still have original value",
    //             i
    //         );
    //     }
    // }

    // // ── Tree grows after shrinking ────────────────────────────────────────────────

    // #[test]
    // fn test_insert_after_mass_remove() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..30 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     // remove almost everything
    //     for i in 0u8..25 {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     // now grow again past original size
    //     for i in 30u8..60 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert after shrink failed");
    //     }

    //     // verify the 5 originals survived
    //     for i in 25u8..30 {
    //         assert!(
    //             tree.get(&[i], &mut storage).is_some(),
    //             "surviving key {} should still exist",
    //             i
    //         );
    //     }

    //     // verify new inserts are all present
    //     for i in 30u8..60 {
    //         assert!(
    //             tree.get(&[i], &mut storage).is_some(),
    //             "new key {} should exist after regrowth",
    //             i
    //         );
    //     }

    //     // verify removed keys are gone
    //     for i in 0u8..25 {
    //         assert!(
    //             tree.get(&[i], &mut storage).is_none(),
    //             "removed key {} should not exist",
    //             i
    //         );
    //     }
    // }

    // // ── Value correctness after removes ──────────────────────────────────────────

    // #[test]
    // fn test_values_correct_after_rebalance() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     // use key * 3 as value so we can verify independently
    //     for i in 0u8..20 {
    //         tree.add_node(&mut storage, &[i], &[i * 3])
    //             .expect("insert failed");
    //     }

    //     // remove every third key
    //     for i in (0u8..20).step_by(3) {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     // verify remaining keys have correct values
    //     for i in 0u8..20 {
    //         if i % 3 == 0 {
    //             assert!(
    //                 tree.get(&[i], &mut storage).is_none(),
    //                 "key {} should be gone",
    //                 i
    //             );
    //         } else {
    //             assert_eq!(
    //                 tree.get(&[i], &mut storage).unwrap().as_ref(),
    //                 &[i * 3],
    //                 "key {} should have value {}, rebalance corrupted values",
    //                 i,
    //                 i * 3
    //             );
    //         }
    //     }
    // }

    // // ── Remove separator keys specifically ───────────────────────────────────────

    // #[test]
    // fn test_remove_separator_key() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     // insert exactly enough to split — the promoted key is the separator
    //     for i in 0u8..=MAX_KEYS_PER_PAGE as u8 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     // the root has exactly 1 separator key — find and remove it
    //     let root = tree.root_page_location.load_page(&mut storage);
    //     assert_eq!(root.nodes.len(), 1, "root should have 1 separator");
    //     let separator_key = root.nodes[0].key.clone();

    //     tree.remove(&separator_key, &mut storage);

    //     // all other keys should still be findable
    //     for i in 0u8..=MAX_KEYS_PER_PAGE as u8 {
    //         if i != separator_key[0] {
    //             assert!(
    //                 tree.get(&[i], &mut storage).is_some(),
    //                 "key {} should survive separator removal",
    //                 i
    //             );
    //         }
    //     }

    //     // verify order is still maintained
    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut collected = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut collected);
    //     let mut sorted = collected.clone();
    //     sorted.sort();
    //     assert_eq!(
    //         collected, sorted,
    //         "tree out of order after separator removal"
    //     );
    // }

    // // ── Random order operations ───────────────────────────────────────────────────

    // #[test]
    // fn test_shuffled_remove_order() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..20 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     // remove in non-sequential order
    //     let remove_order: Vec<u8> = vec![15, 3, 18, 0, 7, 11, 4, 19, 1, 9];
    //     for &k in &remove_order {
    //         tree.remove(&[k], &mut storage);
    //     }

    //     // verify removed keys are gone
    //     for &k in &remove_order {
    //         assert!(
    //             tree.get(&[k], &mut storage).is_none(),
    //             "shuffled-removed key {} should be gone",
    //             k
    //         );
    //     }

    //     // verify remaining keys are still present and sorted
    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut collected = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut collected);

    //     let mut sorted = collected.clone();
    //     sorted.sort();
    //     assert_eq!(
    //         collected, sorted,
    //         "tree out of order after shuffled removes"
    //     );

    //     // verify exact remaining set
    //     let removed_set: std::collections::HashSet<u8> = remove_order.iter().cloned().collect();
    //     for i in 0u8..20 {
    //         let result = tree.get(&[i], &mut storage);
    //         if removed_set.contains(&i) {
    //             assert!(result.is_none(), "key {} should be removed", i);
    //         } else {
    //             assert!(result.is_some(), "key {} should remain", i);
    //         }
    //     }
    // }

    // // ── Interleaved inserts and removes ──────────────────────────────────────────

    // #[test]
    // fn test_interleaved_insert_remove() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     // interleave inserts and removes in batches
    //     for i in 0u8..15 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }
    //     for i in 0u8..5 {
    //         tree.remove(&[i], &mut storage);
    //     }
    //     for i in 15u8..30 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }
    //     for i in 5u8..10 {
    //         tree.remove(&[i], &mut storage);
    //     }
    //     for i in 30u8..45 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }

    //     // verify final state: 0-9 removed, 10-44 present
    //     for i in 0u8..10 {
    //         assert!(
    //             tree.get(&[i], &mut storage).is_none(),
    //             "key {} should be removed",
    //             i
    //         );
    //     }
    //     for i in 10u8..45 {
    //         assert!(
    //             tree.get(&[i], &mut storage).is_some(),
    //             "key {} should exist",
    //             i
    //         );
    //     }

    //     // verify sorted order
    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut collected = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut collected);
    //     let mut sorted = collected.clone();
    //     sorted.sort();
    //     assert_eq!(collected, sorted, "tree out of order after interleaved ops");
    // }

    // // ── Page alignment invariant ─────────────────────────────────────────────────

    // #[test]
    // fn test_page_alignment_after_removes() {
    //     use std::io::{Seek, SeekFrom};

    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     for i in 0u8..50 {
    //         tree.add_node(&mut storage, &[i], &[i])
    //             .expect("insert failed");
    //     }
    //     for i in 0u8..25 {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     let len = storage.seek(SeekFrom::End(0)).unwrap();
    //     assert_eq!(
    //         len % PAGE_SIZE as u64,
    //         0,
    //         "file not page-aligned after removes, len: {}",
    //         len
    //     );
    // }

    // // ── Full cycle stress ─────────────────────────────────────────────────────────

    // #[test]
    // fn test_full_cycle_insert_remove_verify() {
    //     let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
    //     let mut tree = make_tree();

    //     let total = (MAX_KEYS_PER_PAGE * 8) as u8;

    //     // insert all
    //     for i in 0u8..total {
    //         tree.add_node(&mut storage, &[i], &[i.wrapping_mul(2)])
    //             .expect("insert failed");
    //     }

    //     // remove first third
    //     for i in 0u8..total / 3 {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     // remove last third
    //     for i in (total * 2 / 3)..total {
    //         tree.remove(&[i], &mut storage);
    //     }

    //     // only middle third should remain
    //     for i in 0u8..total {
    //         let result = tree.get(&[i], &mut storage);
    //         if i < total / 3 || i >= total * 2 / 3 {
    //             assert!(result.is_none(), "key {} should be removed", i);
    //         } else {
    //             assert_eq!(
    //                 result.unwrap().as_ref(),
    //                 &[i.wrapping_mul(2)],
    //                 "key {} has wrong value after full cycle",
    //                 i
    //             );
    //         }
    //     }

    //     // final order check
    //     let root = tree.root_page_location.load_page(&mut storage);
    //     let mut collected = Vec::new();
    //     collect_keys_in_order(&mut storage, &root, &mut collected);
    //     let mut sorted = collected.clone();
    //     sorted.sort();
    //     assert_eq!(collected, sorted, "tree out of order after full cycle");
    // }
}
