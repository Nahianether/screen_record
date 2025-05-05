use chrono::Utc;
use grpc_video_server::file_upload_to_grpc;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::Runtime;

use crate::modules::api::upload_video_id_fl::video_id_send_to_api_fn;
use crate::modules::components::record_screen::record_screen_fl::record_screen;
use crate::modules::components::video_conversion::components::join_mp4_files_fl::join_mp4_files;
use crate::modules::components::video_conversion::video_conversion_fl::convert_raw_to_mp4;

lazy_static::lazy_static! {
    static ref MP4_BUFFER: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}

pub async fn process_screen_recording() -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let ts = now.format("%Y%m%dT%H%M%S").to_string();
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let tmp_dir = exe_dir.join("temp");
    fs::create_dir_all(&tmp_dir)?;
    let raw_path = tmp_dir.join(format!("screencap_{}.raw", ts));
    let mp4_path = tmp_dir.join(format!("screencap_{}.mp4", ts));

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Failed to create reqwest client");

    println!("Recording screen for 60 seconds...");
    let (width, height, frame_count, actual_secs) =
        record_screen(&raw_path, Duration::from_secs(60))?;
    println!(
        "Recording saved to: {} with {} frames (duration: {:.2} seconds)",
        raw_path.display(),
        frame_count,
        actual_secs
    );

    let raw_path_clone = raw_path.clone();
    let mp4_path_clone = mp4_path.clone();
    let tmp_dir_clone = tmp_dir.clone();

    thread::Builder::new()
        .name("convert_and_upload".into())
        .spawn(move || {
            println!("🌀 Starting conversion in background...");
            if let Err(e) = convert_raw_to_mp4(
                &raw_path_clone,
                &mp4_path_clone,
                width,
                height,
                frame_count,
                actual_secs,
            ) {
                eprintln!("❌ Error during conversion: {}", e);
                return;
            }

            println!(
                "✅ FFmpeg conversion successful: {}",
                mp4_path_clone.display()
            );

            let mut buffer = MP4_BUFFER.lock().unwrap();
            buffer.push(mp4_path_clone.clone());

            if buffer.len() >= 4 {
                let to_join: Vec<_> = buffer.drain(..4).collect();

                let user_id = if cfg!(debug_assertions) {
                    "fb01503c-0302-4033-9b0b-ab737ae1875f"
                } else {
                    "fb01503c-0302-4033-9b0b-ab737ae1875f"
                };
                let joined_output = tmp_dir_clone.join(format!(
                    "{}_{}.mp4",
                    user_id,
                    Utc::now().format("%Y%m%dT%H%M%S")
                ));

                match join_mp4_files(&to_join, &joined_output) {
                    Ok(_) => {
                        println!("🎞️ Videos joined successfully: {}", joined_output.display());
                        // Upload with retry logic
                        Runtime::new().unwrap().block_on(async {
                            const MAX_RETRIES: usize = 3;
                            let mut attempt = 0;

                            loop {
                                attempt += 1;
                                let start = Instant::now();
                                println!("🚀 Attempt {} to upload...", attempt);
                                match file_upload_to_grpc(
                                    &joined_output.display().to_string(),
                                    "23.98.93.20",
                                    "50057",
                                )
                                .await
                                {
                                    Ok(_) => {
                                        println!("✅ Upload successful in {:.2?}", start.elapsed());
                                        if let Err(e) =
                                            video_id_send_to_api_fn(&client, &joined_output).await
                                        {
                                            println!("⚠️ Failed to send video Id to API: {}", e);
                                        } else {
                                            println!("✅ Video ID sent to API successfully.");
                                        }
                                        break;
                                    }
                                    Err(e) if attempt < MAX_RETRIES => {
                                        eprintln!(
                                            "⚠️ Upload failed (attempt {}): {}. Retrying...",
                                            attempt, e
                                        );
                                        tokio::time::sleep(Duration::from_secs(5)).await;
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Final upload attempt failed: {}", e);
                                        break;
                                    }
                                }
                            }
                        });

                        if let Err(e) = fs::remove_file(&joined_output) {
                            eprintln!(
                                "⚠️ Failed to delete final video: {} — {}",
                                joined_output.display(),
                                e
                            );
                        }

                        for f in to_join {
                            let _ = fs::remove_file(&f).map_err(|e| {
                                eprintln!(
                                    "⚠️ Failed to delete temp video: {} — {}",
                                    f.display(),
                                    e
                                );
                            });
                        }
                    }
                    Err(e) => eprintln!("❌ Join failed: {}", e),
                }
            }
        })?;

    Ok(())
}
