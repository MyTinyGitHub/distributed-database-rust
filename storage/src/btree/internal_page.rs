use serde::{Deserialize, Serialize};

use crate::btree::{
    btree::{MAX_KEYS_PER_PAGE, MIN_KEYS_PER_PAGE},
    location::{Location, PageStore, RefPageLocation},
    page::{OverFlowElement, Page, PushResult, RemoveResult},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Internal {
    pub separators: Vec<Box<[u8]>>,
    pub pages: Vec<Location>,
}

impl Internal {
    pub fn size(&self) -> usize {
        self.separators.len()
    }
    pub fn pop_first(&mut self) -> (Box<[u8]>, Location) {
        let separator = self.separators.remove(0);
        let page_loc = self.pages.remove(0);

        (separator, page_loc)
    }

    pub fn pop_last(&mut self) -> (Box<[u8]>, Location) {
        let separator = self.separators.remove(self.separators.len() - 1);
        let page_loc = self.pages.remove(self.pages.len() - 1);

        (separator, page_loc)
    }

    pub fn remove<W: PageStore>(&mut self, key: &[u8], storage: &mut W) -> RemoveResult {
        let index = self.index_of(key);
        let page_loc = self.pages[index];
        let mut page = page_loc.load_page(storage);

        let result = page.remove(key, storage);
        page_loc.write_page(&page, storage);

        match result {
            RemoveResult::NotFound => RemoveResult::NotFound,
            RemoveResult::Removed => RemoveResult::Removed,
            RemoveResult::Underflow => {
                self.handle_underflow(key, storage);

                if self.separators.len() > MIN_KEYS_PER_PAGE {
                    println!("internal removed {:?}", self.separators);
                    RemoveResult::Removed
                } else {
                    println!("internal underflow {:?}", self.separators);
                    RemoveResult::Underflow
                }
            }
        }
    }

    fn underflow_both_right_merge<W: PageStore>(
        &mut self,
        page: &mut Page,
        index: usize,
        r_page: &mut Page,
        storage: &mut W,
    ) {
        println!("both merge r_page");

        let sep = match r_page {
            Page::Leaf(_) => r_page.peek_first().into(),
            Page::Internal(_) => r_page
                .peek_first_location()
                .load_page(storage)
                .peek_first()
                .into(),
        };

        let _ = self.separators.remove(index);
        let _ = self.pages.remove(index + 1);

        page.print(storage);
        page.merge_right(sep, r_page);
    }

    fn underflow_both_right<W: PageStore>(
        &mut self,
        page: &mut Page,
        index: usize,
        r_page: &mut Page,
        r_page_loc: &Location,
        storage: &mut W,
    ) {
        println!("both r_page");
        // borrow from right
        let (r_key, r_ref_page) = r_page.pop_first();
        r_page_loc.write_page(r_page, storage);

        let old_parent_sep = self.separators[index].clone();
        self.separators[index] = match r_page {
            Page::Leaf(_) => r_page.peek_first().into(),
            Page::Internal(_) => r_key.clone(),
        };

        match page {
            Page::Internal(_) => page.push_last(old_parent_sep, r_ref_page),
            Page::Leaf(_) => page.push_last(r_key, r_ref_page),
        };
    }

    fn underflow_both_left<W: PageStore>(
        &mut self,
        page: &mut Page,
        index: usize,
        l_page: &mut Page,
        l_page_loc: &Location,
        storage: &mut W,
    ) {
        println!("both l_page");

        let (l_key, l_ref_page) = l_page.pop_last();

        l_page_loc.write_page(l_page, storage);

        let old_parent_sep = self.separators[index - 1].clone();
        self.separators[index - 1] = l_key.clone();

        match page {
            Page::Internal(_) => page.push_first(old_parent_sep, l_ref_page),
            Page::Leaf(_) => page.push_first(l_key, l_ref_page),
        };
    }

    fn underflow_both<W: PageStore>(
        &mut self,
        page: &mut Page,
        index: usize,
        l_page: &mut Page,
        l_page_loc: &Location,
        r_page: &mut Page,
        r_page_loc: &Location,
        storage: &mut W,
    ) {
        if r_page.size() > MIN_KEYS_PER_PAGE {
            self.underflow_both_right(page, index, r_page, r_page_loc, storage);
        } else if l_page.size() > MIN_KEYS_PER_PAGE {
            self.underflow_both_left(page, index, l_page, l_page_loc, storage);
        } else {
            self.underflow_both_right_merge(page, index, r_page, storage);
        }
    }

    fn underflow_single_left<W: PageStore>(
        &mut self,
        page: &mut Page,
        l_page: &mut Page,
        l_page_loc: &Location,
        storage: &mut W,
    ) {
        if l_page.size() > MIN_KEYS_PER_PAGE {
            println!("single l_page");
            let (l_key, l_ref_page) = l_page.pop_last();
            l_page_loc.write_page(l_page, storage);

            match page {
                Page::Leaf(leaf) => {
                    leaf.keys.insert(0, l_key.clone());
                    leaf.values.insert(0, l_ref_page);

                    println!("taking leaf separator {:?}", l_key);

                    let sep_size = self.size();
                    self.separators[sep_size - 1] = l_key;
                }
                Page::Internal(int) => {
                    let sep_size = self.size();

                    let s = self.separators[sep_size - 1].clone();

                    int.pages.insert(0, l_ref_page);
                    int.separators.insert(0, s.clone());

                    self.separators[sep_size - 1] = l_key;
                }
            };
        } else {
            println!("single merge l_page");

            let sep = match &page {
                Page::Leaf(_) => self.separators.pop().unwrap(),
                Page::Internal(_) => self.separators.pop().unwrap(),
            };

            let _ = self.pages.pop().unwrap();

            l_page.merge_right(sep, page);
        }
    }

    fn underflow_single_right<W: PageStore>(
        &mut self,
        page: &mut Page,
        r_page: &mut Page,
        r_page_loc: &Location,
        storage: &mut W,
    ) {
        if r_page.size() > MIN_KEYS_PER_PAGE {
            println!("single r_page");

            let (r_key, r_ref_page) = r_page.pop_first();

            r_page_loc.write_page(r_page, storage);

            match page {
                Page::Leaf(leaf) => {
                    let s = r_page.peek_first().into();

                    leaf.keys.push(r_key);
                    leaf.values.push(r_ref_page);

                    println!("taking leaf separator {:?}", s);

                    self.separators[0] = s;
                }
                Page::Internal(int) => {
                    let s = self.separators[0].clone();

                    int.pages.push(r_ref_page);
                    int.separators.push(s.clone());

                    self.separators[0] = r_key;
                }
            };
        } else {
            println!("single merge r_page");

            let sep = match r_page {
                Page::Leaf(_) => r_page.peek_first().into(),
                Page::Internal(_) => r_page
                    .peek_first_location()
                    .load_page(storage)
                    .peek_first()
                    .into(),
            };

            let _ = self.separators.remove(0);
            let _ = self.pages.remove(1);

            page.merge_right(sep, r_page);
        }
    }

    pub fn handle_underflow<W: PageStore>(&mut self, key: &[u8], storage: &mut W) {
        let index = self.index_of(key);
        let page_loc = self.pages[index];
        let mut page = page_loc.load_page(storage);

        println!("underflow index {}", index);

        let r_page_loc = self.pages.get(index + 1).copied();
        let l_page_loc = if index > 0 {
            self.pages.get(index - 1).copied()
        } else {
            None
        };

        let r_page = match r_page_loc {
            None => None,
            Some(loc) => Some(loc.load_page(storage)),
        };

        let l_page = match l_page_loc {
            None => None,
            Some(loc) => Some(loc.load_page(storage)),
        };

        match (l_page, r_page) {
            (None, None) => unreachable!(),
            (None, Some(mut r_page)) => {
                if let Some(mut r_page_loc) = r_page_loc {
                    self.underflow_single_right(&mut page, &mut r_page, &mut r_page_loc, storage);
                    r_page_loc.write_page(&r_page, storage);
                }
            }
            (Some(mut l_page), None) => {
                if let Some(mut l_page_loc) = l_page_loc {
                    self.underflow_single_left(&mut page, &mut l_page, &mut l_page_loc, storage);
                    l_page_loc.write_page(&l_page, storage);
                }
            }
            (Some(mut l_page), Some(mut r_page)) => {
                self.underflow_both(
                    &mut page,
                    index,
                    &mut l_page,
                    &mut l_page_loc.unwrap(),
                    &mut r_page,
                    &mut r_page_loc.unwrap(),
                    storage,
                );
                l_page_loc.unwrap().write_page(&l_page, storage);
                r_page_loc.unwrap().write_page(&r_page, storage);
            }
        };

        page_loc.write_page(&page, storage);
    }

    pub fn get<W: PageStore>(&self, key: &[u8], storage: &mut W) -> Option<Location> {
        let index = self.index_of(key);
        self.pages[index].load_page(storage).get(key, storage)
    }

    pub fn add_page(&mut self, key: &[u8], value: RefPageLocation) -> PushResult {
        let index = self.index_of(key);

        self.separators.insert(index, key.into());
        self.pages.insert(index, Location::Page(value));

        PushResult::Inserted
    }

    pub fn add<W: PageStore>(
        &mut self,
        key: &[u8],
        value: Location,
        storage: &mut W,
    ) -> PushResult {
        let index = self.index_of(key);

        println!(
            "Sep: {:?} Page size: {} index: {}",
            self.separators,
            self.pages.len(),
            index
        );

        let page_loc = self.pages[index];
        let mut page = page_loc.load_page(storage);
        let result = page.add(key, value, storage);

        page_loc.write_page(&page, storage);

        match result {
            PushResult::Inserted => PushResult::Inserted,
            PushResult::Overflow(overflow) => {
                self.separators.insert(index, overflow.key);

                let p_location = RefPageLocation::alloc(storage).unwrap();
                let p_location = Location::Page(p_location);

                p_location.write_page(&overflow.page, storage);

                self.pages.insert(index + 1, p_location);

                if self.separators.len() >= MAX_KEYS_PER_PAGE {
                    let (page, key) = self.split();
                    PushResult::Overflow(OverFlowElement { key, page })
                } else {
                    PushResult::Inserted
                }
            }
        }
    }

    pub fn split(&mut self) -> (Page, Box<[u8]>) {
        let mut r_separators = self.separators.split_off(self.separators.len() / 2);
        // Use the post-split separator count (+1) as the page split index so both
        // halves satisfy the pages.len() == separators.len() + 1 invariant for
        // any input size (odd or even).
        let r_pages = self.pages.split_off(self.separators.len() + 1);

        // let key = r_separators.remove(0);
        let key = r_separators.remove(0);

        return (
            Page::Internal(Internal {
                separators: r_separators,
                pages: r_pages,
            }),
            key,
        );
    }

    fn index_of(&self, key: &[u8]) -> usize {
        let index = self.separators.partition_point(|sep| sep.as_ref() <= key);
        println!("key: {:?}, index: {}", key, index);
        index
    }
}
