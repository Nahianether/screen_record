use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn convert_raw_to_mp4(
    raw_path: &PathBuf,
    mp4_path: &PathBuf,
    width: usize,
    height: usize,
    frames: usize,
    duration_secs: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame_rate = (frames as f64 / duration_secs).round() as usize;
    if frame_rate == 0 {
        frame_rate = 1;
    }

    let status = Command::new("C:\\ffmpeg\\bin\\ffmpeg.exe")
        .args([
            "-f",
            "rawvideo",
            "-pixel_format",
            "bgra",
            "-video_size",
            &format!("{}x{}", width, height),
            "-framerate",
            &frame_rate.to_string(),
            "-i",
            raw_path.to_str().unwrap(),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            mp4_path.to_str().unwrap(),
        ])
        .status()?;

    if status.success() {
        println!("Conversion succeeded!");

        if raw_path.exists() {
            fs::remove_file(raw_path)?;
            println!("Deleted raw file: {}", raw_path.display());
        }
    }

    if !status.success() {
        return Err("FFmpeg failed to convert raw to mp4".into());
    }
    Ok(())
}
