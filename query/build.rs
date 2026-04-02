use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("database.bin"),
        )
        .compile_protos(&["../common/proto/wal.proto"], &["../common/proto/"])?;

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("database.bin"),
        )
        .compile_protos(&["../common/proto/storage.proto"], &["../common/proto/"])?;

    Ok(())
}
