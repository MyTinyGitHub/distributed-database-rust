#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    version: usize,
    wal_manifest: WalManifest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalManifest {
    active_idx: usize,
}
