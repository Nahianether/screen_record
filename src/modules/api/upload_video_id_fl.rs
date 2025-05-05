use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

pub async fn video_id_send_to_api_fn(client: &Client, video_id: &PathBuf) -> Result<()> {
    let api_url = "https://app.trackforce.io/api/TrackerDesktop/AddWebCamEvent";

    println!("Sending video Id to the API...");
    let file_name = if let Some(name) = video_id.file_name().and_then(|name| name.to_str()) {
        println!("Video ID: {}", name);
        name.to_string()
    } else {
        println!(
            "Failed to extract video ID from path: {}",
            video_id.display()
        );
        return Err(anyhow::anyhow!("Failed to extract video ID from path"));
    };

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()?);

    let payload = json!({
        "employeeId": "fb01503c-0302-4033-9b0b-ab737ae1875f",
        "accountId": 0,
        "fileId": file_name.to_string(),
        "createdAt": Utc::now().to_rfc3339(),
    });

    println!("Payload: {}", payload.to_string());
    println!("Headers: {:?}", headers);

    let response = client
        .post(api_url)
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("✅ Video ID sent successfully: {}", video_id.display());
        Ok(())
    } else {
        eprintln!("⚠️ Failed to send video ID: {}", response.status());
        Err(anyhow::anyhow!(
            "Failed to send video ID: {}",
            response.status()
        ))
    }
}
