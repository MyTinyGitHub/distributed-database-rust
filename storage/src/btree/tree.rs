use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

use log::info;

use crate::{
    btree::{
        internal_page::Internal,
        leaf_page::Leaf,
        location::{Location, PageStore, RefPageLocation},
        page::{Page, PushResult, RemoveResult},
    },
    storage_error::StorageError,
};

pub const MAX_KEYS_PER_PAGE: usize = 9;
pub const MIN_KEYS_PER_PAGE: usize = MAX_KEYS_PER_PAGE / 2;

#[derive(Debug)]
pub struct PagingBtree<W: PageStore> {
    pub storage: W,
    pub root_page_location: RefPageLocation,
}

impl PagingBtree<File> {
    pub fn open(file_path: &PathBuf) -> Self {
        let root_location = RefPageLocation { start_offset: 0 };

        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();

        let exists = std::fs::exists(file_path).unwrap();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)
            .unwrap();

        if !exists {
            let root_page = Page::Leaf(Leaf {
                keys: Vec::new(),
                values: Vec::new(),
            });

            root_location.write_page(&root_page, &mut file);
        }

        Self {
            storage: file,
            root_page_location: root_location,
        }
    }
}

impl<W: PageStore> PagingBtree<W> {
    pub fn with_storage(storage: W) -> Self {
        Self {
            storage,
            root_page_location: RefPageLocation { start_offset: 0 },
        }
    }

    pub fn get(&mut self, key: &[u8]) -> Option<Location> {
        let page = self.root_page_location.load_page(&mut self.storage);
        info!(" searching for key {:?} in {:?}", key, page);
        page.get(key, &mut self.storage)
    }

    pub fn remove(&mut self, key: &[u8]) -> Result<(), StorageError> {
        let storage = &mut self.storage;
        let mut root_page = self.root_page_location.load_page(storage);

        let result = root_page.remove(key, storage);

        match result {
            RemoveResult::NotFound => println!("root notfound"),
            RemoveResult::Underflow => match &root_page {
                Page::Leaf(_) => println!("underflow in leaf"),
                Page::Internal(internal) => {
                    if internal.pages.len() == 1 {
                        println!("replacing root");
                        root_page = internal.pages.first().unwrap().load_page(storage);
                    }
                }
            },
            RemoveResult::Removed => {
                println!("root removed");

                match &root_page {
                    Page::Leaf(_) => println!("underflow in leaf"),
                    Page::Internal(internal) => {
                        println!("replacing root");
                        if internal.pages.len() == 1 {
                            root_page = internal.pages.first().unwrap().load_page(storage);
                        }
                    }
                }
            }
        }

        self.root_page_location.write_page(&root_page, storage);

        Ok(())
    }

    pub fn insert(&mut self, key: &[u8], value: Location) -> Result<(), StorageError> {
        let storage = &mut self.storage;
        let mut root_page = self.root_page_location.load_page(storage);

        let result = root_page.add(key, value, storage);

        match result {
            PushResult::Overflow(overflow) => {
                info!("inserted {:?} with overflow", key);
                let right_page_loc = Location::Page(RefPageLocation::alloc(storage)?);
                right_page_loc.write_page(&overflow.page, storage);

                let left_page_loc = Location::Page(RefPageLocation::alloc(storage)?);
                left_page_loc.write_page(&root_page, storage);

                let new_root_page = Page::Internal(Internal {
                    separators: vec![overflow.key],
                    pages: vec![left_page_loc, right_page_loc],
                });

                self.root_page_location.write_page(&new_root_page, storage);

                Ok(())
            }
            PushResult::Inserted => {
                info!("inserted {:?}", key);
                self.root_page_location.write_page(&root_page, storage);
                Ok(())
            }
        }
    }
}
