use chrono::Utc;
use grpc_video_server::file_upload_to_grpc;
use std::fs;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::modules::components::record_screen::record_screen_fl::record_screen;
use crate::modules::components::video_conversion::video_conversion_fl::convert_raw_to_mp4;

pub async fn process_screen_recording() -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let ts = now.format("%Y%m%dT%H%M%S").to_string();
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let tmp_dir = exe_dir.join("temp");
    fs::create_dir_all(&tmp_dir)?;
    let raw_path = tmp_dir.join(format!("screencap_{}.raw", ts));
    let mp4_path = tmp_dir.join(format!("screencap_{}.mp4", ts));

    println!("Recording screen for 180 seconds...");
    let (width, height, frame_count, actual_secs) =
        record_screen(&raw_path, Duration::from_secs(180))?;
    println!(
        "Recording saved to: {} with {} frames (duration: {:.2} seconds)",
        raw_path.display(),
        frame_count,
        actual_secs
    );

    let raw_path_clone = raw_path.clone();
    let mp4_path_clone = mp4_path.clone();

    thread::spawn(move || {
        println!("Starting conversion in background...");
        if let Err(e) = convert_raw_to_mp4(
            &raw_path_clone,
            &mp4_path_clone,
            width,
            height,
            frame_count,
            actual_secs,
        ) {
            eprintln!("Error during conversion: {}", e);
        } else {
            println!("FFmpeg conversion successful: {}", mp4_path_clone.display());

            let upload_path = mp4_path_clone.clone();

            Runtime::new().unwrap().block_on(async move {
                if let Err(e) =
                    file_upload_to_grpc(&upload_path.display().to_string(), "23.98.93.20", "50057")
                        .await
                {
                    eprintln!("Upload failed: {}", e);
                } else {
                    println!("Upload successful!");
                }
            });
        }
    });

    Ok(())
}
