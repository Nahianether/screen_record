use scrap::{Capturer, Display};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub fn record_screen(
    path: &PathBuf,
    duration: Duration,
) -> Result<(usize, usize, usize, f64), Box<dyn std::error::Error>> {
    let one = Display::primary()?;
    let mut capturer = Capturer::new(one)?;
    let (w, h) = (capturer.width(), capturer.height());

    let mut output = File::create(path)?;
    let start = Instant::now();
    let mut frame_count = 0;

    while start.elapsed() < duration {
        match capturer.frame() {
            Ok(frame) => {
                output.write_all(&frame)?;
                frame_count += 1;
            }
            Err(error) => {
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(Box::new(error));
                }
            }
        }
        thread::sleep(Duration::from_millis(33));
    }

    let actual_secs = start.elapsed().as_secs_f64();
    println!(
        "Captured {}x{} for {:.2} seconds with {} frames",
        w, h, actual_secs, frame_count
    );
    Ok((w, h, frame_count, actual_secs))
}
