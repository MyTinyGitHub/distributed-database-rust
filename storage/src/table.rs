use std::collections::{BTreeMap, HashMap};

use crate::heap_file::HeapFile;
use crate::record::EngineRecord;

pub struct Table {
    file: HeapFile,
    indexes: HashMap<Vec<u8>, BTreeMap<Vec<u8>, EngineRecord>>,
}
