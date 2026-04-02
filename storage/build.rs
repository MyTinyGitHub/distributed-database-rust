use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("engine_descriptor.bin"),
        )
        .compile_protos(&["../common/proto/engine.proto"], &["../common/proto"])?;

    Ok(())
}
