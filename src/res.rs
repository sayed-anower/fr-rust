use actix_files::NamedFile;
use actix_web::{Responder, ResponseError, HttpRequest, HttpResponse};
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
pub fn send_file(req: HttpRequest, path: &str) -> impl Responder {
    match NamedFile::open(PathBuf::from(path)) {
        Ok(file) => file.into_response(&req),
        Err(_) => HttpResponse::NotFound().body("File not found"),
    }
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
