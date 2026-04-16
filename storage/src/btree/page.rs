use serde::{Deserialize, Serialize};

use crate::btree::{
    internal_page::Internal,
    leaf_page::Leaf,
    location::{Location, PageStore},
};

#[derive(Clone, Serialize, Deserialize)]
pub enum Page {
    Internal(Internal),
    Leaf(Leaf),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PushResult {
    Inserted,
    Overflow(OverFlowElement),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OverFlowElement {
    pub key: Box<[u8]>,
    pub page: Page,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum RemoveResult {
    Removed,
    NotFound,
    Underflow,
}

impl Page {
    pub fn size(&self) -> usize {
        match self {
            Page::Internal(n) => n.separators.len(),
            Page::Leaf(n) => n.keys.len(),
        }
    }

    pub fn print<R: PageStore>(&self, storage: &mut R) {
        match self {
            Page::Internal(n) => {
                println!("{:?}", n.separators);
                println!();

                for i in 0..n.pages.len() {
                    print!("child: {i}");
                    n.pages[i].load_page(storage).print(storage);
                }
            }
            Page::Leaf(n) => println!("{:?}", n.keys),
        };
    }

    pub fn peek_first_location(&self) -> Location {
        match self {
            Page::Internal(n) => n.pages[0],
            Page::Leaf(n) => n.values[0],
        }
    }

    pub fn peek_first(&self) -> &[u8] {
        match self {
            Page::Internal(n) => n.separators[0].as_ref(),
            Page::Leaf(n) => n.keys[0].as_ref(),
        }
    }

    pub fn peek_last(&self) -> &[u8] {
        match self {
            Page::Internal(n) => n.separators.last().unwrap().as_ref(),
            Page::Leaf(n) => n.keys.last().unwrap().as_ref(),
        }
    }

    pub fn push_first(&mut self, key: Box<[u8]>, val: Location) {
        match self {
            Page::Internal(n) => {
                n.separators.insert(0, key);
                n.pages.insert(0, val);
            }
            Page::Leaf(n) => {
                n.keys.insert(0, key);
                n.values.insert(0, val);
            }
        }
    }

    pub fn push_last(&mut self, key: Box<[u8]>, val: Location) {
        match self {
            Page::Internal(n) => {
                n.separators.push(key);
                n.pages.push(val);
            }
            Page::Leaf(n) => {
                n.keys.push(key);
                n.values.push(val);
            }
        }
    }

    pub fn merge_right(&mut self, sep: Box<[u8]>, right: &mut Page) {
        match (self, right) {
            (Page::Internal(l), Page::Internal(r)) => {
                println!("merging left: {:?}, right {:?}", l.separators, r.separators);
                l.separators.push(sep);
                l.separators.append(&mut r.separators);
                l.pages.append(&mut r.pages);
            }
            (Page::Leaf(l), Page::Leaf(r)) => {
                println!("merging leafs left: {:?}, right {:?}", l.keys, r.keys);
                l.keys.append(&mut r.keys);
                l.values.append(&mut r.values);
            }
            _ => unreachable!("Cannot merge Leaf Page with Internal Page"),
        }
    }

    pub fn add<W: PageStore>(
        &mut self,
        key: &[u8],
        value: Location,
        storage: &mut W,
    ) -> PushResult {
        match value {
            Location::Page(page_loc) => match self {
                Page::Internal(node) => node.add_page(key, page_loc),
                _ => unimplemented!(),
            },
            Location::Value(_) => match self {
                Page::Internal(node) => node.add(key, value, storage),
                Page::Leaf(node) => node.add(key, value),
            },
        }
    }

    pub fn split(&mut self) -> (Page, Box<[u8]>) {
        println!("Splitting");
        match self {
            Page::Internal(internal) => internal.split(),
            Page::Leaf(leaf) => leaf.split(),
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<Location> {
        match self {
            Page::Internal(internal) => internal.get(key, storage),
            Page::Leaf(leaf) => leaf.get(key),
        }
    }

    pub fn remove<W: PageStore>(&mut self, key: &[u8], storage: &mut W) -> RemoveResult {
        match self {
            Page::Internal(internal) => internal.remove(key, storage),
            Page::Leaf(leaf) => leaf.remove(key),
        }
    }

    pub fn pop_first(&mut self) -> (Box<[u8]>, Location) {
        match self {
            Page::Internal(internal) => internal.pop_first(),
            Page::Leaf(leaf) => leaf.pop_first(),
        }
    }

    pub fn pop_last(&mut self) -> (Box<[u8]>, Location) {
        match self {
            Page::Internal(internal) => internal.pop_last(),
            Page::Leaf(leaf) => leaf.pop_last(),
        }
    }
}
