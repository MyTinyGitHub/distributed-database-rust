pub struct EngineRecord {
    pub version: u8,
    pub data: EngineHeader,
}

pub struct EngineHeader {
    // pub expired: u64,
    // pub created: u64,
    pub start_offset: u64,
    pub size: usize,
}
