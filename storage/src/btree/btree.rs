use std::path::PathBuf;

use crate::{
    btree::{
        internal_page::Internal,
        location::{Location, PageStore, RefPageLocation},
        page::{Page, PushResult, RemoveResult},
    },
    storage_error::StorageError,
};

pub const MAX_KEYS_PER_PAGE: usize = 9;
pub const MIN_KEYS_PER_PAGE: usize = MAX_KEYS_PER_PAGE / 2 - 1;

#[derive(Debug)]
pub struct PagingBtree {
    pub file_path: PathBuf,
    pub root_page_location: RefPageLocation,
}

impl PagingBtree {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path: file_path.clone(),
            root_page_location: RefPageLocation { start_offset: 0 },
        }
    }

    pub fn get<R: PageStore>(&self, key: &[u8], storage: &mut R) -> Option<Location> {
        let page = self.root_page_location.load_page(storage);
        page.get(key, storage)
    }

    pub fn remove<W: PageStore>(&self, key: &[u8], storage: &mut W) -> Result<(), StorageError> {
        let mut root_page = self.root_page_location.load_page(storage);

        let result = root_page.remove(key, storage);
        //HANDLE UNDERFLOW IN PARENT

        self.root_page_location.write_page(&root_page, storage);

        Ok(())
    }

    pub fn add<W: PageStore>(
        &mut self,
        key: &[u8],
        value: Location,
        storage: &mut W,
    ) -> Result<(), StorageError> {
        println!("Adding");
        let mut root_page = self.root_page_location.load_page(storage);

        let result = root_page.add(key, value, storage);

        match result {
            PushResult::Overflow(overflow) => {
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
                self.root_page_location.write_page(&root_page, storage);
                Ok(())
            }
        }
    }
}
