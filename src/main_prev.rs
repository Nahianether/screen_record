// // main.rs
// use chrono::Utc;
// use scrap::{Capturer, Display};
// use std::fs::File;
// use std::io::Write;
// use std::path::PathBuf;
// use std::process::Command;
// use std::thread;
// use std::time::Instant;
// use std::{fs, time::Duration};
// // use tokio::time::sleep;

// mod uploader {
//     tonic::include_proto!("uploader");
// }

// use futures_core::Stream;
// use std::path::PathBuf;
// use std::pin::Pin;
// use std::task::{Context, Poll};
// use tokio::io::AsyncReadExt;
// use tokio_stream::StreamExt;
// use tonic::Request;
// use uploader::uploader_client::UploaderClient;
// use uploader::UploadRequest;

// #[tokio::main]
// // async fn main() -> Result<(), Box<dyn std::error::Error>> {
// //     env_logger::init();

// //     loop {
// //         let now = Utc::now();
// //         let ts = now.format("%Y%m%dT%H%M%S").to_string();
// //         let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
// //         let tmp_dir = exe_dir.join("temp");
// //         fs::create_dir_all(&tmp_dir)?;
// //         let raw_path = tmp_dir.join(format!("screencap_{}.raw", ts));
// //         let mp4_path = tmp_dir.join(format!("screencap_{}.mp4", ts));

// //         println!("Recording screen for 180 seconds...");
// //         let (width, height, frame_count, actual_secs) =
// //             record_screen(&raw_path, Duration::from_secs(180))?;
// //         println!(
// //             "Recording saved to: {} with {} frames (duration: {:.2} seconds)",
// //             raw_path.display(),
// //             frame_count,
// //             actual_secs
// //         );

// //         // convert_raw_to_mp4(&raw_path, &mp4_path, width, height, frame_count)?;
// //         let raw_path_clone = raw_path.clone();
// //         let mp4_path_clone = mp4_path.clone();
// //         thread::spawn(move || {
// //             if let Err(e) = convert_raw_to_mp4(
// //                 &raw_path_clone,
// //                 &mp4_path_clone,
// //                 width,
// //                 height,
// //                 frame_count,
// //                 actual_secs,
// //             ) {
// //                 eprintln!("Error during conversion: {}", e);
// //             }
// //         });
// //         println!("Converted to MP4: {}", mp4_path.display());

// //         // sleep(Duration::from_secs(10)).await;
// //     }
// // }

// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     env_logger::init();

//     loop {
//         let now = Utc::now();
//         let ts = now.format("%Y%m%dT%H%M%S").to_string();
//         let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
//         let tmp_dir = exe_dir.join("temp");
//         fs::create_dir_all(&tmp_dir)?;
//         let raw_path = tmp_dir.join(format!("screencap_{}.raw", ts));
//         let mp4_path = tmp_dir.join(format!("screencap_{}.mp4", ts));

//         println!("Recording screen for 180 seconds...");
//         let (width, height, frame_count, actual_secs) =
//             record_screen(&raw_path, Duration::from_secs(180))?;
//         println!(
//             "Recording saved to: {} with {} frames (duration: {:.2} seconds)",
//             raw_path.display(),
//             frame_count,
//             actual_secs
//         );

//         // ✅ Move conversion to background thread
//         let raw_path_clone = raw_path.clone();
//         let mp4_path_clone = mp4_path.clone();
//         thread::spawn(move || {
//             println!("Starting conversion in background...");
//             if let Err(e) = convert_raw_to_mp4(
//                 &raw_path_clone,
//                 &mp4_path_clone,
//                 width,
//                 height,
//                 frame_count,
//                 actual_secs,
//             ) {
//                 eprintln!("Error during conversion: {}", e);
//             } else {
//                 println!("Converted to MP4: {}", mp4_path_clone.display());
//             }
//         });

//         // ✅ Immediately continue capturing next one
//     }
// }

// fn record_screen(
//     path: &PathBuf,
//     duration: Duration,
// ) -> Result<(usize, usize, usize, f64), Box<dyn std::error::Error>> {
//     let one = Display::primary()?;
//     let mut capturer = Capturer::new(one)?;
//     let (w, h) = (capturer.width(), capturer.height());

//     let mut output = File::create(path)?;
//     let start = Instant::now();
//     let mut frame_count = 0;

//     while start.elapsed() < duration {
//         match capturer.frame() {
//             Ok(frame) => {
//                 output.write_all(&frame)?;
//                 frame_count += 1;
//             }
//             Err(error) => {
//                 if error.kind() != std::io::ErrorKind::WouldBlock {
//                     return Err(Box::new(error));
//                 }
//             }
//         }
//         thread::sleep(Duration::from_millis(33));
//     }

//     let actual_secs = start.elapsed().as_secs_f64();
//     println!(
//         "Captured {}x{} for {:.2} seconds with {} frames",
//         w, h, actual_secs, frame_count
//     );
//     Ok((w, h, frame_count, actual_secs))
// }

// // fn convert_raw_to_mp4(
// //     raw_path: &PathBuf,
// //     mp4_path: &PathBuf,
// //     width: usize,
// //     height: usize,
// //     frames: usize,
// // ) -> Result<(), Box<dyn std::error::Error>> {
// //     let frame_rate = (frames as f64 / 10.0).ceil() as usize;
// //     let status = Command::new("C:\\ffmpeg\\bin\\ffmpeg.exe")
// //         .args([
// //             "-f",
// //             "rawvideo",
// //             "-pixel_format",
// //             "bgra",
// //             "-video_size",
// //             &format!("{}x{}", width, height),
// //             "-framerate",
// //             &frame_rate.to_string(),
// //             "-i",
// //             raw_path.to_str().unwrap(),
// //             "-c:v",
// //             "libx264",
// //             "-pix_fmt",
// //             "yuv420p",
// //             mp4_path.to_str().unwrap(),
// //         ])
// //         .status()?;

// //     if !status.success() {
// //         return Err("FFmpeg failed to convert raw to mp4".into());
// //     }
// //     Ok(())
// // }

// fn convert_raw_to_mp4(
//     raw_path: &PathBuf,
//     mp4_path: &PathBuf,
//     width: usize,
//     height: usize,
//     frames: usize,
//     duration_secs: f64,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut frame_rate = (frames as f64 / duration_secs).round() as usize;
//     if frame_rate == 0 {
//         frame_rate = 1; // ⚡ Force at least 1 FPS
//     }
//     let status = Command::new("C:\\ffmpeg\\bin\\ffmpeg.exe")
//         .args([
//             "-f",
//             "rawvideo",
//             "-pixel_format",
//             "bgra",
//             "-video_size",
//             &format!("{}x{}", width, height),
//             "-framerate",
//             &frame_rate.to_string(),
//             "-i",
//             raw_path.to_str().unwrap(),
//             "-c:v",
//             "libx264",
//             "-pix_fmt",
//             "yuv420p",
//             mp4_path.to_str().unwrap(),
//         ])
//         .status()?;

//     if status.success() {
//         println!("FFmpeg conversion successful");

//         tokio::spawn(async move {
//             if let Err(e) = upload_file_via_grpc(mp4_path_clone).await {
//                 eprintln!("Upload failed: {}", e);
//             }
//         });

//         if raw_path.exists() {
//             fs::remove_file(raw_path)?;
//             println!("Deleted raw file: {}", raw_path.display());
//         }
//     }

//     if !status.success() {
//         return Err("FFmpeg failed to convert raw to mp4".into());
//     }
//     Ok(())
// }


// pub async fn upload_file_via_grpc(mp4_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
//     println!("Connecting to gRPC server...");

//     let mut client = UploaderClient::connect("http://localhost:50051").await?;
//     let file_path = mp4_path.clone();
//     let mut file = File::open(&file_path)?;

//     println!("Preparing file for upload: {}", file_path.display());

//     let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer per chunk

//     // Make an async generator stream
//     let output_stream = async_stream::stream! {
//         loop {
//             let n = match file.read(&mut buf).await {
//                 Ok(0) => break, // End of file
//                 Ok(n) => n,
//                 Err(e) => {
//                     eprintln!("Error reading file: {}", e);
//                     break;
//                 }
//             };
//             let chunk = UploadRequest {
//                 chunk: buf[..n].to_vec(),
//             };
//             yield chunk;
//         }
//     };

//     let response = client.upload(Request::new(output_stream)).await?;
//     println!("Upload finished: {:?}", response.into_inner().message);

//     Ok(())
// }
