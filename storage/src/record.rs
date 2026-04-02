pub struct EngineRecord {
    version: u8,
    data: EngineHeader,
}

pub struct EngineHeader {
    expired: u64,
    created: u64,
    data: Vec<u8>,
}
