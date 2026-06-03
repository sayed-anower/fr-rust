use actix_files::NamedFile;
use actix_web::{HttpResponse, Result, Error};
use serde::Serialize;
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

// Sends a plain string body
pub fn send_str(content: &str) -> HttpResponse {
    HttpResponse::Ok().body(content.to_string())
}

// Sends a JSON response
pub fn send_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// Sends a file
pub async fn send_file(path: &str) -> Result<NamedFile> {
    Ok(NamedFile::open_async(path).await?)
}

/// 200 OK
pub fn http_ok(msg: &str) -> HttpResponse {
    HttpResponse::Ok().body(msg.to_string())
}

/// 400 Bad Request
pub fn http_bad(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().body(msg.to_string())
}

// 200 OK response with json
pub fn http_ok_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// 400 Bad Request response with json
pub fn http_bad_json<T: Serialize>(msg: T) -> HttpResponse {
    HttpResponse::BadRequest().json(msg)
}
// Handle Upload File
pub async fn upload_file<P: AsRef<Path>>(mut payload: Multipart, target_dir: P) -> Result<Vec<String>, Error> {
    let mut uploaded_files = Vec::new();
    let base_path = target_dir.as_ref();

    // Ensure the target directory exists asynchronously before writing to it
    if !base_path.exists() {
        tokio::fs::create_dir_all(base_path).await?;
    }

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let filename = field
            .content_disposition()
            .as_ref()
            .and_then(|cd| cd.get_filename())
            .map_or_else(|| "unknown".to_string(), |f| f.to_string());

        
        // Safely join the target directory with the filename (handles OS slashes automatically)
        let filepath = base_path.join(&filename);
        
        let file = File::create(&filepath).await?;
        let mut writer = BufWriter::new(file);

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            writer.write_all(&data).await?;
        }

        writer.flush().await?;
        uploaded_files.push(filename);
    }

    Ok(uploaded_files)
}
