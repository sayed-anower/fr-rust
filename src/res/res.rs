use actix_files::NamedFile;
use actix_web::{Error, HttpResponse, Result};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use serde::Serialize;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

// Sends a plain string body
#[inline]
pub fn send_str(content: &str) -> HttpResponse {
    HttpResponse::Ok().body(content.to_string())
}

// Sends a JSON response
#[inline]
pub fn send_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// Sends a file
#[inline]
pub async fn send_file(path: &str) -> Result<NamedFile> {
    Ok(NamedFile::open_async(path).await?)
}

/// 200 OK
#[inline]
pub fn http_ok(msg: &str) -> HttpResponse {
    HttpResponse::Ok().body(msg.to_string())
}

/// 400 Bad Request
#[inline]
pub fn http_bad(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().body(msg.to_string())
}

// 200 OK response with json
#[inline]
pub fn http_ok_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// 400 Bad Request response with json
#[inline]
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

    while let Some(mut field) = payload.try_next().await? {
        let filename = field
            .content_disposition()
            .expect("Sending File Failed!")
            .get_filename()
            .map_or_else(|| "unknown".to_string(), |f| f.to_string());

        // Safely join the target directory with the filename
        let filepath = base_path.join(&filename);
        
        let file = File::create(&filepath).await?;
        // BufWriter significantly improves performance for streaming chunk writes
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