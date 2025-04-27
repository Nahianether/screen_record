fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        // .out_dir("src/") // Generates uploader.rs in src/
        .compile(&["proto/uploader.proto"], &["proto"])?;
    Ok(())
}
