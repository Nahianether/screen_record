use chrono::Utc;
use grpc_video_server::file_upload_to_grpc;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::Runtime;

use crate::modules::api::download::download_recorder_exe;
use crate::modules::api::upload_video_id_fl::video_id_send_to_api_fn;

pub const VIDEO_RECORDER_EXE: &str = "screen_record.exe";
lazy_static::lazy_static! {
    static ref MP4_BUFFER: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}

// static RECORDER_CHILD: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

use tokio::process::Command as AsyncCommand;
use tokio::time::{sleep, Duration as TokioDuration};

pub async fn process_screen_recording(
    user_id: &str,
    api_url: &str,
    recorder_exe_url: &str,
    grpc_server_ip: &str,
    grpc_server_port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let ts = now.format("%Y%m%dT%H%M%S").to_string();
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let tmp_dir = exe_dir.join("temp");
    fs::create_dir_all(&tmp_dir)?;
    let file_name = recorder_exe_url
        .split('/')
        .last()
        .unwrap_or(VIDEO_RECORDER_EXE);

    let recorder_exe = exe_dir.join("bin").join(&file_name);
    if !recorder_exe.exists() {
        fs::create_dir_all(&recorder_exe.parent().unwrap())?;
        match download_recorder_exe(recorder_exe_url, &recorder_exe).await {
            Ok(_) => {
                println!("✅ Recorder executable downloaded successfully.");
            }
            Err(e) => {
                eprintln!("❌ Failed to download recorder executable: {}", e);
                return Err(e.into());
            }
        }
    }

    let mp4_path = tmp_dir.join(format!("{}{}.mp4", user_id, ts));
    let output_path = if mp4_path.extension().and_then(|s| s.to_str()) != Some("mp4") {
        mp4_path.with_extension("mp4")
    } else {
        mp4_path.clone()
    };

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;

    println!("🎬 Starting recording to: {}", output_path.display());

    // Start recording process asynchronously
    let mut child = AsyncCommand::new(&recorder_exe)
        .current_dir(&exe_dir)
        .args([
            "--output",
            output_path.to_str().ok_or("Invalid path")?,
            "--duration",
            "120",
            "--fps",
            "24",
            "--resolution",
            "1280x720",
        ])
        .spawn()?;

    println!("✅ Recording process started!");

    // Monitor the recording process and file creation
    let output_path_clone = output_path.clone();
    let monitor_task = tokio::spawn(async move {
        let mut last_size = 0u64;
        let mut file_created = false;

        loop {
            if output_path_clone.exists() && !file_created {
                println!("📁 Recording file created!");
                file_created = true;
            }

            if file_created {
                if let Ok(metadata) = std::fs::metadata(&output_path_clone) {
                    let size = metadata.len();
                    if size != last_size && size > 0 {
                        println!("📈 File size: {} bytes", size);
                        last_size = size;
                    }
                }
            }

            sleep(TokioDuration::from_secs(5)).await;
        }
    });

    // Wait for recording to complete
    let recording_result = child.wait().await;
    monitor_task.abort(); // Stop monitoring

    match recording_result {
        Ok(status) if status.success() => {
            println!("🎬 Recording completed successfully!");

            // Verify file was created
            if output_path.exists() {
                let file_size = std::fs::metadata(&output_path)?.len();
                println!("✅ Recording saved (size: {} bytes)", file_size);

                if file_size == 0 {
                    return Err("Recording file is empty".into());
                }

                // Start upload process
                println!("📤 Starting upload process...");

                const MAX_RETRIES: usize = 3;
                let mut attempt = 0;

                loop {
                    attempt += 1;
                    let start = Instant::now();
                    println!("🚀 Attempt {} to upload...", attempt);

                    match file_upload_to_grpc(
                        &output_path.display().to_string(),
                        grpc_server_ip,
                        grpc_server_port,
                    )
                    .await
                    {
                        Ok(_) => {
                            println!("✅ Upload successful in {:.2?}", start.elapsed());

                            // Send video ID to API
                            if let Err(e) =
                                video_id_send_to_api_fn(&client, &output_path, user_id, api_url)
                                    .await
                            {
                                println!("⚠️ Failed to send video Id to API: {}", e);
                            } else {
                                println!("✅ Video ID sent to API successfully.");
                            }
                            break;
                        }
                        Err(e) if attempt < MAX_RETRIES => {
                            eprintln!("⚠️ Upload failed (attempt {}): {}. Retrying...", attempt, e);
                            sleep(TokioDuration::from_secs(5)).await;
                        }
                        Err(e) => {
                            eprintln!("❌ Final upload attempt failed: {}", e);
                            return Err(e.into());
                        }
                    }
                }

                // Clean up the file
                if let Err(e) = fs::remove_file(&output_path) {
                    eprintln!(
                        "⚠️ Failed to delete video file: {} — {}",
                        output_path.display(),
                        e
                    );
                }

                println!("🎉 Process completed successfully!");
                Ok(())
            } else {
                Err("Recording file was not created".into())
            }
        }
        Ok(status) => Err(format!("Recording failed with exit code: {:?}", status.code()).into()),
        Err(e) => Err(format!("Failed to wait for recording process: {}", e).into()),
    }
}

pub fn record_screen_py(path: &PathBuf, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let recorder_exe = exe_dir.join("bin").join(&file_name);

    if !recorder_exe.exists() {
        return Err(format!(
            "Screen recorder executable not found at: {}",
            recorder_exe.display()
        )
        .into());
    }

    // Convert output path to MP4 if it's not already
    let output_path = if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
        path.with_extension("mp4")
    } else {
        path.clone()
    };

    println!("Starting recording to: {}", output_path.display());

    let output = Command::new(&recorder_exe)
        .current_dir(&exe_dir) // Set working directory
        .args([
            "--output",
            output_path.to_str().ok_or("Invalid path")?,
            "--duration",
            "120",
            "--fps",
            "24",
            "--resolution",
            "1280x720",
        ])
        .output()?; // This waits for completion and captures output

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Always print output for debugging
    if !stdout.is_empty() {
        println!("Recorder output: {}", stdout);
    }
    if !stderr.is_empty() {
        println!("Recorder errors: {}", stderr);
    }

    if output.status.success() {
        // Verify the file was actually created
        if output_path.exists() {
            let file_size = std::fs::metadata(&output_path)?.len();
            println!(
                "✅ Recording saved to {} (size: {} bytes)",
                output_path.display(),
                file_size
            );
            if file_size == 0 {
                return Err("Recording file is empty".into());
            }
        } else {
            return Err("Recording file was not created".into());
        }
        Ok(())
    } else {
        Err(format!(
            "❌ Recording failed with exit code: {:?}\nStderr: {}\nStdout: {}",
            output.status.code(),
            stderr,
            stdout
        )
        .into())
    }
}

pub fn stop_recorder() -> io::Result<()> {
    match is_process_running(VIDEO_RECORDER_EXE) {
        Ok(v) => match v {
            true => kill_process_by_name(VIDEO_RECORDER_EXE),
            false => {
                println!("⚠️ Process {} is not running.", VIDEO_RECORDER_EXE);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Process {} is not running.", VIDEO_RECORDER_EXE),
                ));
            }
        },
        Err(e) => Err(e),
    }
}

fn is_process_running(exe_name: &str) -> io::Result<bool> {
    let output = Command::new("tasklist")
        .args(&["/FI", &format!("IMAGENAME eq {}", exe_name)])
        .stdout(Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_lowercase().contains(&exe_name.to_lowercase()))
}

fn kill_process_by_name(exe_name: &str) -> io::Result<()> {
    let status = Command::new("taskkill")
        .args(&["/F", "/IM", exe_name])
        .status()?;

    if status.success() {
        println!("✅ Successfully killed {}", exe_name);
    } else {
        eprintln!("⚠️ taskkill exited with {:?}", status.code());
    }
    Ok(())
}
