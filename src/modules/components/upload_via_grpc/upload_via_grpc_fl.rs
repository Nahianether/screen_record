mod upload {
    tonic::include_proto!("upload"); // package name
}

use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tonic::Request;
use upload::upload_service_client::UploadServiceClient;
use upload::{UploadRequest, upload_request};

pub async fn upload_file_via_grpc(mp4_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to gRPC server...");

    let mut client = UploadServiceClient::connect("http://23.98.93.20:50057").await?;
    let mut file = tokio::fs::File::open(&mp4_path).await?;

    println!("Preparing file for upload: {}", mp4_path.display());

    let mut buf = vec![0u8; 1024 * 1024];

    let output_stream = async_stream::stream! {
        loop {
            let n = match file.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    break;
                }
            };

            let chunk = UploadRequest {
                r#type: Some(upload_request::Type::Chunk(buf[..n].to_vec())),
            };
            yield chunk;
        }
    };

    let response = client.upload_file(Request::new(output_stream)).await?;
    println!("Upload finished: {:?}", response.into_inner().message);

    Ok(())
}
