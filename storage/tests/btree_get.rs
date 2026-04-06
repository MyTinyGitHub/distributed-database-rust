include!("btree_helper.rs");

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_get_returns_correct_value() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // insert keys with distinct values to verify get() returns the right one
        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");
        tree.add_node(&mut storage, &[2], &[20])
            .expect("insert failed");
        tree.add_node(&mut storage, &[3], &[30])
            .expect("insert failed");

        assert_eq!(tree.get(&[1], &mut storage).unwrap().as_ref(), &[10]);
        assert_eq!(tree.get(&[2], &mut storage).unwrap().as_ref(), &[20]);
        assert_eq!(tree.get(&[3], &mut storage).unwrap().as_ref(), &[30]);
    }

    #[test]
    fn test_get_missing_key_returns_none() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[1], &[10])
            .expect("insert failed");
        tree.add_node(&mut storage, &[2], &[20])
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

        // write an empty page at offset 0 so load_page doesn't read garbage
        let empty_page = Page {
            nodes: vec![],
            pages: None,
        };
        let loc = PageLocation { start_offset: 0 };
        loc.write_page(&empty_page, &mut storage);

        let tree = make_tree();
        assert!(
            tree.get(&[1], &mut storage).is_none(),
            "expected None on empty tree"
        );
    }

    #[test]
    fn test_get_correct_value_after_split() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        // use distinct key != value so we know we're reading the right value
        for i in 0u8..11 {
            tree.add_node(&mut storage, &[i], &[i + 100])
                .expect("insert failed");
        }

        for i in 0u8..11 {
            let result = tree.get(&[i], &mut storage);
            assert!(result.is_some(), "key {} not found", i);
            assert_eq!(
                result.unwrap().as_ref(),
                &[i + 100],
                "wrong value for key {}",
                i
            );
        }
    }

    #[test]
    fn test_get_correct_value_after_multiple_splits() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        let count = (MAX_KEYS_PER_PAGE * 5) as u8;
        for i in 0u8..count {
            tree.add_node(&mut storage, &[i], &[i.wrapping_add(50)])
                .expect("insert failed");
        }

        for i in 0u8..count {
            let result = tree.get(&[i], &mut storage);
            assert!(result.is_some(), "key {} not found", i);
            assert_eq!(
                result.unwrap().as_ref(),
                &[i.wrapping_add(50)],
                "wrong value for key {}",
                i
            );
        }
    }

    #[test]
    fn test_get_multi_byte_key_and_value() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        let key = [0xDE, 0xAD, 0xBE, 0xEF];
        let value = [0xCA, 0xFE, 0xBA, 0xBE];

        tree.add_node(&mut storage, &key, &value)
            .expect("insert failed");

        let result = tree.get(&key, &mut storage);
        assert!(result.is_some(), "multi-byte key not found");
        assert_eq!(
            result.unwrap().as_ref(),
            value.as_slice(),
            "value mismatch for multi-byte key"
        );
    }

    #[test]
    fn test_get_boundary_keys() {
        let mut storage = Cursor::new(vec![0u8; PAGE_SIZE]);
        let mut tree = make_tree();

        tree.add_node(&mut storage, &[u8::MIN], &[1])
            .expect("insert failed");
        tree.add_node(&mut storage, &[u8::MAX], &[2])
            .expect("insert failed");

        assert_eq!(tree.get(&[u8::MIN], &mut storage).unwrap().as_ref(), &[1]);
        assert_eq!(tree.get(&[u8::MAX], &mut storage).unwrap().as_ref(), &[2]);
        assert!(
            tree.get(&[128], &mut storage).is_none(),
            "expected None for key not inserted"
        );
    }
}
