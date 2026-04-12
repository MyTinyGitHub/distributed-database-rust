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
                let r_page_loc = self.pages.get(index + 1);
                let l_page_loc = if index > 0 {
                    self.pages.get(index - 1)
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
                    (Some(mut l_page), None) => {
                        if l_page.size() > MIN_KEYS_PER_PAGE {
                            let (l_key, l_ref_page) = l_page.pop_last();
                            l_page_loc
                                .expect("page was loaded with this location, cannot be None")
                                .write_page(&l_page, storage);

                            let sep_size = self.size();
                            self.separators[sep_size - 1] = l_key.clone();

                            page.push_first(l_key, l_ref_page);
                            page_loc.write_page(&page, storage);
                        } else {
                            let sep = self.separators.pop().unwrap();

                            l_page.merge_right(sep, &mut page);
                            l_page_loc.unwrap().write_page(&l_page, storage);
                        }
                    }
                    (None, Some(mut r_page)) => {
                        if r_page.size() > MIN_KEYS_PER_PAGE {
                            let (r_key, r_ref_page) = r_page.pop_first();

                            r_page_loc
                                .expect("page was loaded with this location, cannot be None")
                                .write_page(&r_page, storage);

                            // self.separators[0] = r_key.clone();
                            // self.separators[1] = r_page.peek_first().into();
                            self.separators[0] = r_page.peek_first().into();

                            page.push_last(r_key, r_ref_page);
                        } else {
                            let sep = self.separators.remove(0);
                            let _ = self.pages.remove(1);

                            page.merge_right(sep, &mut r_page);
                        }
                    }
                    (Some(mut l_page), Some(mut r_page)) => {
                        if r_page.size() > MIN_KEYS_PER_PAGE {
                            // borrow from right
                            let (r_key, r_ref_page) = r_page.pop_first();
                            r_page_loc
                                .expect("page was loaded with this location, cannot be None")
                                .write_page(&r_page, storage);

                            println!("deleting key: {:?}", key);
                            println!("separators: {:?}", self.separators);
                            println!("pages: {:?}", self.pages.len());

                            // self.separators[index] = r_key.clone();
                            // self.separators[index + 1] = r_page.peek_first().into();
                            self.separators[index] = r_page.peek_first().into();

                            page.push_last(r_key, r_ref_page);
                        } else if l_page.size() > MIN_KEYS_PER_PAGE {
                            // borrow from left
                            let (l_key, l_ref_page) = l_page.pop_last();
                            l_page_loc
                                .expect("page was loaded with this location, cannot be None")
                                .write_page(&l_page, storage);
                            self.separators[index - 1] = l_key.clone();
                            page.push_first(l_key, l_ref_page);
                        } else {
                            // merge with right
                            let sep = self.separators.remove(index);
                            let _ = self.pages.remove(index + 1);
                            page.merge_right(sep, &mut r_page);
                        }
                    }
                };

                page_loc.write_page(&page, storage);

                if self.separators.len() > MIN_KEYS_PER_PAGE {
                    RemoveResult::Removed
                } else {
                    RemoveResult::Underflow
                }
            }
        }
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
            "Sep size: {:?} Page size: {} index: {}",
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
        let r_pages = self.pages.split_off(self.pages.len() / 2);

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
        self.separators.partition_point(|sep| sep.as_ref() <= key)
    }
}
