use anyhow::Result;
use screen_record::run::process_screen_recording;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    loop {
        if let Err(e) = process_screen_recording(
            "fb01503c-0302-4033-9b0b-ab737ae1875f",
            "https://app.trackforce.io/api/TrackerDesktop/AddWebCamEvent",
            // "https://screen-record.ibos.io/file/screen_record.exe",
            "https://drive.google.com/file/d/1Nd9DwSXr3oxrfa7f77j3OBHDF4MkA7YU/view?usp=sharing",
            "23.98.93.20",
            "50057",
        )
        .await
        {
            eprintln!("Error: {}", e);
        }
    }
    // if let Err(e) = process_screen_recording(
    //     "fb01503c-0302-4033-9b0b-ab737ae1875f",
    //     "https://app.trackforce.io/api/TrackerDesktop/AddWebCamEvent",
    //     "23.98.93.20",
    //     "50057",
    // )
    // .await
    // {
    //     eprintln!("Error: {}", e);
    // }

    // Ok(())
}
