use actix_files::NamedFile;
use actix_web::{HttpResponse, Responder, ResponseError, Result};
use serde::Serialize;
use std::path::PathBuf;

// Sends a plain string body
pub fn send_str(content: String) -> impl Responder {
    HttpResponse::Ok().body(content)
}

// Sends a JSON response
pub fn send_json<T: Serialize>(data: &T) -> impl Responder {
    HttpResponse::Ok().json(data)
}

// Sends a file from a path
pub fn send_file(path: &str) -> impl Responder {
    match NamedFile::open(PathBuf::from(path)) {
        Ok(file) => file.into_response(),
        Err(_) => HttpResponse::NotFound().body("File not found"),
    }
}

/// 200 OK
pub fn http_ok(msg: &str) -> impl Responder {
    HttpResponse::Ok().body(msg.into())
}

/// 400 Bad Request
pub fn http_bad(msg: &str) -> impl Responder {
    HttpResponse::BadRequest().body(msg.into())
}

// 200 OK response with json
pub fn http_ok_json<T: Serialize>(data: T) -> impl Responder {
    HttpResponse::Ok().json(data)
}

// 400 Bad Request response with json
pub fn http_bad_json<T: Serialize>(msg: T) -> impl Responder {
    HttpResponse::BadRequest().json(msg)
}
