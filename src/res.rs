use actix_files::NamedFile;
use actix_web::{Responder, HttpResponse, Result};
use serde::Serialize;

// Sends a plain string body
pub fn send_str(content: String) -> impl Responder {
    HttpResponse::Ok().body(content)
}

// Sends a JSON response
pub fn send_json<T: Serialize>(data: &T) -> impl Responder {
    HttpResponse::Ok().json(data)
}

// Sends a file
pub async fn send_file(path: &'static str) -> Result<NamedFile> {
    Ok(NamedFile::open_async(path).await?)
}

/// 200 OK
pub fn http_ok(msg: String) -> impl Responder {
    HttpResponse::Ok().body(msg)
}

/// 400 Bad Request
pub fn http_bad(msg: String) -> impl Responder {
    HttpResponse::BadRequest().body(msg)
}

// 200 OK response with json
pub fn http_ok_json<T: Serialize>(data: T) -> impl Responder {
    HttpResponse::Ok().json(data)
}

// 400 Bad Request response with json
pub fn http_bad_json<T: Serialize>(msg: T) -> impl Responder {
    HttpResponse::BadRequest().json(msg)
}
