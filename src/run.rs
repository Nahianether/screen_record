use chrono::Utc;
use grpc_video_server::file_upload_to_grpc;
use reqwest::Client;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use crate::modules::api::upload_video_id_fl::video_id_send_to_api_fn;

pub const VIDEO_RECORDER_EXE: &str = "screen_record.exe";

lazy_static::lazy_static! {
    static ref VIDEO_BUFFER: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}
// screen_record.exe
pub async fn process_screen_recording(
    user_id: &str,
    api_url: &str,
    _recorder_exe_url: &str,
    grpc_server_ip: &str,
    grpc_server_port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let ts = now.format("%Y%m%dT%H%M%S").to_string();
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let tmp_dir = exe_dir.join("temp");

    // Ensure temp directory exists and is writable
    println!("📂 Ensuring temp directory exists: {}", tmp_dir.display());
    fs::create_dir_all(&tmp_dir).map_err(|e| {
        format!("Failed to create temp directory at '{}': {}", tmp_dir.display(), e)
    })?;

    // Verify directory is accessible
    match fs::read_dir(&tmp_dir) {
        Ok(_) => println!("✅ Temp directory is accessible"),
        Err(e) => {
            return Err(format!(
                "Temp directory exists but is not accessible: {} - Error: {}",
                tmp_dir.display(),
                e
            ).into());
        }
    }

    // Try multiple locations for the recorder executable
    // IMPORTANT: Exclude target/debug/screen_record.exe as that's THIS program!
    let possible_locations = vec![
        exe_dir.join("bin").join("screen_record.exe"),           // target/debug/bin/screen_record.exe
        exe_dir.parent().unwrap().parent().unwrap().join("screen_record.exe"), // project root (69MB file)
        std::env::current_dir()?.join("screen_record.exe"),      // current working directory
        exe_dir.join("..").join("..").join("bin").join("screen_record.exe"), // project_root/bin/screen_record.exe
    ];

    let mut recorder_exe: Option<PathBuf> = None;
    let current_exe = std::env::current_exe()?;

    println!("🔍 Searching for screen_record.exe...");
    for location in &possible_locations {
        println!("   Checking: {}", location.display());

        // Skip if this is the current executable (prevent recursion!)
        if location.canonicalize().ok() == current_exe.canonicalize().ok() {
            println!("   ⚠️ Skipping - this is the current program itself");
            continue;
        }

        if location.exists() {
            // Verify file size - the actual recorder should be larger (69MB)
            if let Ok(metadata) = fs::metadata(&location) {
                let size = metadata.len();
                if size > 50_000_000 {  // Should be around 69MB
                    println!("✅ Found recorder executable at: {} ({} MB)", location.display(), size / 1_000_000);
                    recorder_exe = Some(location.clone());
                    break;
                } else {
                    println!("   ⚠️ File too small ({} MB) - likely not the recorder", size / 1_000_000);
                }
            }
        }
    }

    let recorder_exe = recorder_exe.ok_or_else(|| {
        format!(
            "❌ Recorder executable 'screen_record.exe' not found in any of these locations:\n{}\n\
             Note: Looking for the large (~69MB) screen recorder executable, not the compiled Rust program.",
            possible_locations.iter()
                .map(|p| format!("   - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;

    // Verify the executable is accessible
    match fs::metadata(&recorder_exe) {
        Ok(_) => {
            println!("✅ Recorder executable is accessible and ready");
        }
        Err(e) => {
            return Err(format!("Cannot access recorder executable: {} - Error: {}", recorder_exe.display(), e).into());
        }
    }

    // Create initial path with .webm extension for web compatibility
    let initial_path = tmp_dir.join(format!("{}{}.webm", user_id, ts));

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;

    println!("🎬 Starting recording to: {}", initial_path.display());
    println!("🔧 Recorder executable: {}", recorder_exe.display());
    println!("📁 Working directory: {}", exe_dir.display());

    // Verify paths before execution
    if !tmp_dir.exists() {
        println!("⚠️ Temp directory doesn't exist, creating: {}", tmp_dir.display());
        fs::create_dir_all(&tmp_dir)?;
    }

    // Execute the recorder with improved error handling and real-time output
    println!("🚀 Executing recorder command...");
    println!("⏱️  Recording will take 120 seconds (2 minutes)...");

    let mut child = Command::new(&recorder_exe)
        .current_dir(&exe_dir)
        .args([
            "--output",
            initial_path.to_str().ok_or("Invalid path")?,
            "--duration",
            "120",
            "--fps",
            "24",
            "--resolution",
            "1280x720",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to execute recorder at '{}': {} (Error code: {:?})\n\
                 This might be due to:\n\
                 1. The executable is corrupted or not a valid Windows executable\n\
                 2. The executable is blocked by Windows (right-click > Properties > Unblock)\n\
                 3. Missing required DLL files or dependencies\n\
                 4. Antivirus software blocking execution\n\
                 5. The file path contains invalid characters\n\
                 Working directory: {}\n\
                 Output path: {}",
                recorder_exe.display(),
                e,
                e.raw_os_error(),
                exe_dir.display(),
                initial_path.display()
            )
        })?;

    // Read stdout in real-time (without adding extra emojis)
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    }

    // Wait for the process to complete
    let output = child.wait_with_output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    println!(
        "🏁 Process completed with exit code: {:?}",
        output.status.code()
    );

    // Show errors if any
    if !stderr.is_empty() {
        println!("=== RECORDER ERRORS ===");
        println!("{}", stderr);
        println!("=== END ERRORS ===");
    }

    // Smart file detection - the Python script might create different formats
    println!("📁 Detecting created video file...");

    let possible_extensions = vec!["webm", "mp4", "avi", "mkv"];
    let base_name = initial_path.file_stem().unwrap().to_str().unwrap();
    let parent_dir = initial_path.parent().unwrap();

    let mut actual_file_path: Option<PathBuf> = None;

    // First, check for files with the exact base name
    for ext in &possible_extensions {
        let test_path = parent_dir.join(format!("{}.{}", base_name, ext));
        println!("📁 Checking: {}", test_path.display());

        if test_path.exists() {
            let file_size = fs::metadata(&test_path)?.len();
            if file_size > 0 {
                println!(
                    "✅ Found video file: {} ({} bytes)",
                    test_path.display(),
                    file_size
                );
                actual_file_path = Some(test_path);
                break;
            } else {
                println!("⚠️ Found empty file: {}", test_path.display());
            }
        }
    }

    // If no exact match, scan for recent video files in the directory
    if actual_file_path.is_none() {
        println!("🔍 Scanning for recent video files in temp directory...");

        if let Ok(entries) = fs::read_dir(&tmp_dir) {
            let mut recent_videos = Vec::new();
            let now = std::time::SystemTime::now();

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if let Some(ext) = path.extension() {
                        if possible_extensions.contains(&ext.to_str().unwrap_or("")) {
                            if let Ok(metadata) = fs::metadata(&path) {
                                let size = metadata.len();
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(duration) = now.duration_since(modified) {
                                        // Consider files created in the last 2 minutes
                                        if duration.as_secs() < 120 && size > 1000 {
                                            println!(
                                                "📹 Found recent video: {} ({} bytes)",
                                                path.display(),
                                                size
                                            );
                                            recent_videos.push((path, size, modified));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Use the most recently created video file
            if !recent_videos.is_empty() {
                recent_videos.sort_by_key(|(_, _, modified)| *modified);
                if let Some((path, size, _)) = recent_videos.last() {
                    println!(
                        "✅ Using most recent video: {} ({} bytes)",
                        path.display(),
                        size
                    );
                    actual_file_path = Some(path.clone());
                }
            }
        }
    }

    // Process the found file
    match actual_file_path {
        Some(final_path) => {
            let file_size = fs::metadata(&final_path)?.len();

            if file_size == 0 {
                return Err("Recording file is empty".into());
            }

            println!("🎬 Recording completed successfully!");
            println!(
                "✅ Final video file: {} ({} bytes)",
                final_path.display(),
                file_size
            );

            // Determine file format for logging
            let file_format = final_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_uppercase();
            println!("📄 Format: {}", file_format);

            // Start upload process
            println!("📤 Starting upload process...");

            const MAX_RETRIES: usize = 3;
            let mut attempt = 0;

            loop {
                attempt += 1;
                let start = Instant::now();
                println!("🚀 Upload attempt {} of {}...", attempt, MAX_RETRIES);

                match file_upload_to_grpc(
                    &final_path.display().to_string(),
                    grpc_server_ip,
                    grpc_server_port,
                )
                .await
                {
                    Ok(_) => {
                        println!("✅ Upload successful in {:.2?}", start.elapsed());

                        // Send video ID to API
                        match video_id_send_to_api_fn(&client, &final_path, user_id, api_url).await
                        {
                            Ok(_) => {
                                println!("✅ Video ID sent to API successfully.");
                            }
                            Err(e) => {
                                println!("⚠️ Failed to send video ID to API: {}", e);
                                // Don't fail the entire process for API issues
                            }
                        }
                        break;
                    }
                    Err(e) if attempt < MAX_RETRIES => {
                        eprintln!(
                            "⚠️ Upload failed (attempt {}): {}. Retrying in 5s...",
                            attempt, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        eprintln!("❌ All upload attempts failed: {}", e);
                        return Err(e.into());
                    }
                }
            }

            // Clean up the file
            match fs::remove_file(&final_path) {
                Ok(_) => {
                    println!("🗑️ Temporary file cleaned up");
                }
                Err(e) => {
                    eprintln!(
                        "⚠️ Failed to delete temporary file: {} — {}",
                        final_path.display(),
                        e
                    );
                    // Don't fail the process for cleanup issues
                }
            }

            println!("🎉 Screen recording process completed successfully!");
            Ok(())
        }
        None => {
            eprintln!("❌ No recording file was created");

            // List all files for debugging
            println!("📂 Files in temp directory:");
            if let Ok(entries) = fs::read_dir(&tmp_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Ok(metadata) = fs::metadata(&path) {
                            println!("  - {} ({} bytes)", path.display(), metadata.len());
                        }
                    }
                }
            }

            Err(format!(
                "Recording failed: No video file was created\nExit code: {:?}",
                output.status.code()
            )
            .into())
        }
    }
}

// Utility function to stop any running recorder processes
pub fn stop_recorder() -> io::Result<()> {
    match is_process_running(VIDEO_RECORDER_EXE) {
        Ok(true) => {
            println!("🛑 Stopping running recorder process...");
            kill_process_by_name(VIDEO_RECORDER_EXE)
        }
        Ok(false) => {
            println!("ℹ️ No recorder process is currently running.");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Error checking for running processes: {}", e);
            Err(e)
        }
    }
}

fn is_process_running(exe_name: &str) -> io::Result<bool> {
    let output = Command::new("tasklist")
        .args(&["/FI", &format!("IMAGENAME eq {}", exe_name)])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_lowercase().contains(&exe_name.to_lowercase()))
}

fn kill_process_by_name(exe_name: &str) -> io::Result<()> {
    let status = Command::new("taskkill")
        .args(&["/F", "/IM", exe_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if status.success() {
        println!("✅ Successfully stopped {}", exe_name);
    } else {
        eprintln!(
            "⚠️ Failed to stop {} (exit code: {:?})",
            exe_name,
            status.code()
        );
    }
    Ok(())
}

// Test function for development
#[cfg(debug_assertions)]
pub async fn test_recording() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running test recording...");

    process_screen_recording(
        "test_user",
        "http://localhost:8080/api",
        "http://example.com/screen_record.exe",
        "localhost",
        "50051",
    )
    .await
}
