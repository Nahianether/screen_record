use screen_record::run::process_screen_recording;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    loop {
        if let Err(e) = process_screen_recording().await {
            eprintln!("Error: {}", e);
        }
    }
}
