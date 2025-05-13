use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/upload.proto")?;
    // Copy screen_recorder.exe to the output directory
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("../../.."); // Points to target/debug or target/release
    let bin_dest = dest_path.join("bin");

    // Create the bin directory in the output path
    fs::create_dir_all(&bin_dest)?;

    // Copy screen_recorder.exe
    let src_exe = Path::new("bin/screen_recorder.exe");
    let dest_exe = bin_dest.join("screen_recorder.exe");

    if src_exe.exists() {
        fs::copy(&src_exe, &dest_exe)?;
        println!("Copied {} to {}", src_exe.display(), dest_exe.display());
    } else {
        println!("Warning: {} not found", src_exe.display());
    }
    Ok(())
}
