use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub fn join_mp4_files(
    mp4_paths: &[PathBuf],
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let list_path = exe_dir.join("file_list.txt");

    // Step 1: Create file_list.txt for FFmpeg
    let mut list_file = File::create(&list_path)?;
    for path in mp4_paths {
        writeln!(list_file, "file '{}'", path.display())?;
    }

    // Step 2: Run FFmpeg concat
    let ffmpeg_exe = if cfg!(debug_assertions) {
        PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe")
    } else {
        exe_dir.join("ffmpeg").join("ffmpeg.exe")
    };

    let status = Command::new(ffmpeg_exe)
        .args([
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            list_path.to_str().unwrap(),
            "-c",
            "copy",
            output_path.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        return Err("Failed to join mp4 files using FFmpeg".into());
    }

    // Optional cleanup
    fs::remove_file(list_path)?;

    println!("✅ Joined video created at: {}", output_path.display());
    Ok(())
}
