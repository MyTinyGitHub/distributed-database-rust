include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use storage::btree::location::{Location, RefValueLocation};

    use super::*;

    #[test]
    fn test_get_returns_correct_value() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let first_loc = create_loc(1);
        let second_loc = create_loc(2);
        let third_loc = create_loc(3);

        // insert keys with distinct values to verify get() returns the right one
        println!("adding first element");
        tree.add(&[1], first_loc, &mut storage)
            .expect("insert failed");

        println!("adding second element");
        tree.add(&[2], second_loc, &mut storage)
            .expect("insert failed");

        println!("adding third element");
        tree.add(&[3], third_loc, &mut storage)
            .expect("insert failed");

        assert!(is_loc_equal(
            tree.get(&[1], &mut storage).unwrap(),
            first_loc,
        ));

        assert!(is_loc_equal(
            tree.get(&[2], &mut storage).unwrap(),
            second_loc,
        ));

        assert!(is_loc_equal(
            tree.get(&[3], &mut storage).unwrap(),
            third_loc,
        ));
    }

    #[test]
    fn test_get_missing_key_returns_none() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let first_loc = create_loc(10);
        let second_loc = create_loc(20);

        tree.add(&[1], first_loc, &mut storage)
            .expect("insert failed");
        tree.add(&[2], second_loc, &mut storage)
            .expect("insert failed");

        assert!(
            tree.get(&[99], &mut storage).is_none(),
            "expected None for missing key 99"
        );
        assert!(
            tree.get(&[0], &mut storage).is_none(),
            "expected None for missing key 0"
        );
    }

    #[test]
    fn test_get_on_empty_tree() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let tree = make_tree(&mut storage);

        assert!(
            tree.get(&[1], &mut storage).is_none(),
            "expected None on empty tree"
        );
    }

    #[test]
    fn test_get_correct_value_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        // use distinct key != value so we know we're reading the right value
        //
        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        for i in 0u8..11 {
            let loc = create_loc(i as usize);
            let result = tree.get(&[i], &mut storage);
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
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let count = (MAX_KEYS_PER_PAGE * 5) as u8;
        for i in 0u8..count {
            println!("Adding {}", i);
            let loc = create_loc(i.wrapping_add(50) as usize);
            tree.add(&[i], loc, &mut storage).expect("insert failed");
        }

        tree.root_page_location
            .load_page(&mut storage)
            .print(&mut storage);

        for i in 0u8..count {
            let result = tree.get(&[i], &mut storage);
            let r_loc = create_loc(i.wrapping_add(50) as usize);
            assert!(result.is_some(), "key {} not found", i);
            let l_loc = result.unwrap();
            assert!(is_loc_equal(l_loc, r_loc), "wrong value for key {}", i);
        }
    }

    #[test]
    fn test_get_multi_byte_key_and_value() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let key = [0xDE, 0xAD, 0xBE, 0xEF];
        let loc = create_loc(33);

        tree.add(&key, loc, &mut storage).expect("insert failed");

        let result = tree.get(&key, &mut storage);
        let r_loc = result.unwrap();
        assert!(result.is_some(), "multi-byte key not found");
        assert!(
            is_loc_equal(loc, r_loc),
            "value mismatch for multi-byte key"
        );
    }

    #[test]
    fn test_get_boundary_keys() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree(&mut storage);

        let loc = create_loc(1);
        tree.add(&[u8::MIN], loc, &mut storage)
            .expect("insert failed");
        tree.add(&[u8::MAX], loc, &mut storage)
            .expect("insert failed");

        let r_loc = tree.get(&[u8::MAX], &mut storage).unwrap();
        let r_loc_two = tree.get(&[u8::MIN], &mut storage).unwrap();
        assert!(is_loc_equal(loc, r_loc));
        assert!(is_loc_equal(loc, r_loc_two));
        assert!(
            tree.get(&[128], &mut storage).is_none(),
            "expected None for key not inserted"
        );
    }
}
