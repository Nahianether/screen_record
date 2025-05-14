use anyhow::{bail, Result};
use futures_util::TryStreamExt;
use reqwest::Client;
use std::{io::ErrorKind, path::PathBuf};
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;

pub async fn download_recorder_exe(url: &str, dest: &PathBuf) -> Result<()> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        bail!("Failed to download file: {}", response.status());
    }

    let byte_stream = response
        .bytes_stream()
        .map_err(|e| std::io::Error::new(ErrorKind::Other, e));
    let mut reader = StreamReader::new(byte_stream);

    let mut writer = BufWriter::new(File::create(dest).await?);

    tokio::io::copy(&mut reader, &mut writer).await?;

    println!("✅ Download complete: {}", dest.display());
    Ok(())
}
