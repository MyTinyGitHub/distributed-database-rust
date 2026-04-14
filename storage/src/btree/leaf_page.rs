use serde::{Deserialize, Serialize};

use crate::btree::{
    btree::{MAX_KEYS_PER_PAGE, MIN_KEYS_PER_PAGE},
    location::Location,
    page::{OverFlowElement, Page, PushResult, RemoveResult},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Leaf {
    pub keys: Vec<Box<[u8]>>,
    pub values: Vec<Location>,
}

impl Leaf {
    pub fn size(&self) -> usize {
        self.keys.len()
    }

    pub fn pop_first(&mut self) -> (Box<[u8]>, Location) {
        let separator = self.keys.remove(0);
        let page_loc = self.values.remove(0);

        (separator, page_loc)
    }

    pub fn pop_last(&mut self) -> (Box<[u8]>, Location) {
        let separator = self.keys.remove(self.keys.len() - 1);
        let page_loc = self.values.remove(self.values.len() - 1);

        (separator, page_loc)
    }

    pub fn remove(&mut self, key: &[u8]) -> RemoveResult {
        let index = self.keys.partition_point(|p_key| p_key.as_ref() < key);
        println!("removing {:?} at index {}", key, index);
        if index < self.keys.len() && self.keys[index].as_ref() == key {
            self.keys.remove(index);
            self.values.remove(index);

            if self.keys.len() > MIN_KEYS_PER_PAGE {
                println!("leaf-removed");
                RemoveResult::Removed
            } else {
                println!("leaf-remove underflow {:?}", self.keys.len());
                RemoveResult::Underflow
            }
        } else {
            println!("leaf-remove notfound");
            RemoveResult::NotFound
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<Location> {
        for i in 0..self.keys.len() {
            if self.keys[i].as_ref() == key {
                return Some(self.values[i].clone());
            }
        }

        None
    }

    pub fn add(&mut self, key: &[u8], value: Location) -> PushResult {
        let index = self.index_of(key);

        self.keys.insert(index, key.into());
        self.values.insert(index, value);

        if self.keys.len() >= MAX_KEYS_PER_PAGE {
            let (page, key) = self.split();
            return PushResult::Overflow(OverFlowElement { key, page });
        }

        PushResult::Inserted
    }

    pub fn split(&mut self) -> (Page, Box<[u8]>) {
        let r_keys = self.keys.split_off(MAX_KEYS_PER_PAGE / 2);
        let r_val_loc = self.values.split_off(MAX_KEYS_PER_PAGE / 2);

        let m_key = r_keys[0].clone();

        println!("split element {:?}", m_key);

        println!(
            "sizes after split keys {} values {}",
            self.keys.len(),
            self.values.len()
        );

        println!(
            "sizes after split keys {} values {}",
            r_keys.len(),
            r_val_loc.len()
        );

        return (
            Page::Leaf(Leaf {
                keys: r_keys,
                values: r_val_loc,
            }),
            m_key,
        );
    }

    fn index_of(&self, key: &[u8]) -> usize {
        self.keys.partition_point(|sep| sep.as_ref() <= key)
    }
}
