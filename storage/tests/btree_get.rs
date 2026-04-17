include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_get_returns_correct_value() {
        let mut tree = make_tree();

        let first_loc = create_loc(1);
        let second_loc = create_loc(2);
        let third_loc = create_loc(3);

        // insert keys with distinct values to verify get() returns the right one
        println!("inserting first element");
        tree.insert(&[1], first_loc).expect("insert failed");

        println!("inserting second element");
        tree.insert(&[2], second_loc).expect("insert failed");

        println!("inserting third element");
        tree.insert(&[3], third_loc).expect("insert failed");

        assert!(is_loc_equal(tree.get(&[1]).unwrap(), first_loc,));

        assert!(is_loc_equal(tree.get(&[2]).unwrap(), second_loc,));

        assert!(is_loc_equal(tree.get(&[3]).unwrap(), third_loc,));
    }

    #[test]
    fn test_get_missing_key_returns_none() {
        let mut tree = make_tree();

        let first_loc = create_loc(10);
        let second_loc = create_loc(20);

        tree.insert(&[1], first_loc).expect("insert failed");
        tree.insert(&[2], second_loc).expect("insert failed");

        assert!(
            tree.get(&[99]).is_none(),
            "expected None for missing key 99"
        );
        assert!(tree.get(&[0]).is_none(), "expected None for missing key 0");
    }

    #[test]
    fn test_get_on_empty_tree() {
        let mut tree = make_tree();

        assert!(tree.get(&[1]).is_none(), "expected None on empty tree");
    }

    #[test]
    fn test_get_correct_value_after_split() {
        let mut tree = make_tree();

        // use distinct key != value so we know we're reading the right value
        //
        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            let result = tree.get(&[i]);
            assert!(result.is_some(), "key {} not found", i);
            assert!(
                is_loc_equal(result.unwrap(), loc),
                "wrong value for key {}",
                i
            );
        }
    }

    #[test]
    fn test_get_correct_value_after_multiple_splits() {
        let mut tree = make_tree();

        let count = (MAX_KEYS_PER_PAGE * 5) as u8;
        for i in 0u8..count {
            println!("inserting {}", i);
            let loc = create_loc(i.wrapping_add(50) as usize);
            tree.insert(&[i], loc).expect("insert failed");
        }

        for i in 0u8..count {
            let result = tree.get(&[i]);
            let r_loc = create_loc(i.wrapping_add(50) as usize);
            assert!(result.is_some(), "key {} not found", i);
            let l_loc = result.unwrap();
            assert!(is_loc_equal(l_loc, r_loc), "wrong value for key {}", i);
        }
    }

    #[test]
    fn test_get_multi_byte_key_and_value() {
        let mut tree = make_tree();

        let key = [0xDE, 0xAD, 0xBE, 0xEF];
        let loc = create_loc(33);

        tree.insert(&key, loc).expect("insert failed");

        let result = tree.get(&key);
        let r_loc = result.unwrap();
        assert!(result.is_some(), "multi-byte key not found");
        assert!(
            is_loc_equal(loc, r_loc),
            "value mismatch for multi-byte key"
        );
    }

    #[test]
    fn test_get_boundary_keys() {
        let mut tree = make_tree();

        let loc = create_loc(1);
        tree.insert(&[u8::MIN], loc).expect("insert failed");
        tree.insert(&[u8::MAX], loc).expect("insert failed");

        let r_loc = tree.get(&[u8::MAX]).unwrap();
        let r_loc_two = tree.get(&[u8::MIN]).unwrap();
        assert!(is_loc_equal(loc, r_loc));
        assert!(is_loc_equal(loc, r_loc_two));
        assert!(
            tree.get(&[128]).is_none(),
            "expected None for key not inserted"
        );
    }
}
