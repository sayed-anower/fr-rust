use actix_files::NamedFile;
use actix_web::{HttpResponse, Result};
use serde::Serialize;

// Sends a plain string body
pub fn send_str(content: &str) -> HttpResponse {
    HttpResponse::Ok().body(content.to_string())
}

// Sends a JSON response
pub fn send_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// Sends a file
pub async fn send_file(path: &'static str) -> Result<NamedFile> {
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
