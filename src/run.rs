use chrono::Utc;
use grpc_video_server::file_upload_to_grpc;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::Runtime;

use crate::modules::api::download::download_recorder_exe;
use crate::modules::api::upload_video_id_fl::video_id_send_to_api_fn;

lazy_static::lazy_static! {
    static ref MP4_BUFFER: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}

static RECORDER_CHILD: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

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
        .unwrap_or("screen_record.exe");

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

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;

    match record_screen_py(&mp4_path, &file_name) {
        Ok(_) => {
            let mp4_path_clone = mp4_path.clone();
            let user_id = user_id.to_string();
            let api_url = api_url.to_string();
            let grpc_server_ip = grpc_server_ip.to_string();
            let grpc_server_port = grpc_server_port.to_string();

            thread::Builder::new()
                .name("convert_and_upload".into())
                .spawn(move || {
                    println!("🌀 Starting conversion in background...");

                    Runtime::new().unwrap().block_on(async {
                        const MAX_RETRIES: usize = 3;
                        let mut attempt = 0;

                        loop {
                            attempt += 1;
                            let start = Instant::now();
                            println!("🚀 Attempt {} to upload...", attempt);
                            match file_upload_to_grpc(
                                &mp4_path_clone.display().to_string(),
                                &grpc_server_ip,
                                &grpc_server_port,
                            )
                            .await
                            {
                                Ok(_) => {
                                    println!("✅ Upload successful in {:.2?}", start.elapsed());
                                    if let Err(e) = video_id_send_to_api_fn(
                                        &client,
                                        &mp4_path_clone,
                                        &user_id,
                                        &api_url,
                                    )
                                    .await
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
                        if let Err(e) = fs::remove_file(&mp4_path_clone) {
                            eprintln!(
                                "⚠️ Failed to delete final video: {} — {}",
                                mp4_path_clone.display(),
                                e
                            );
                        }
                    });
                })?;
        }
        Err(e) => {
            eprintln!("Error during recording: {}", e);
            return Err(e);
        }
    }

    Ok(())
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

    // let status = Command::new(&recorder_exe)
    //     .arg("--output")
    //     .arg(path.to_str().ok_or("Invalid path")?)
    //     .status()?;
    println!("{}", path.to_str().ok_or("Invalid path")?);
    let mut child = Command::new(&recorder_exe)
        .args([
            "--output",
            // "C:\\Recordings\\rust_clip.mp4",
            path.to_str().ok_or("Invalid path")?,
            "--duration",
            "120",
            "--fps",
            "24",
            "--resolution",
            "1280x720",
        ])
        .spawn()?;

    if child.wait()?.success() {
        println!("✅ Recording saved to {}", path.display());
        *RECORDER_CHILD.lock().unwrap() = Some(child);
        Ok(())
    } else {
        Err(format!(
            "❌ Recording failed with exit code: {:?}",
            child.wait()?.code()
        )
        .into())
    }
}

pub fn stop_recorder() -> io::Result<()> {
    if let Some(mut child) = RECORDER_CHILD.lock().unwrap().take() {
        child.kill()?;
        let status = child.wait()?;
        println!("Recorder exited with: {}", status);
    } else {
        println!("Recorder was not running");
    }
    Ok(())
}
