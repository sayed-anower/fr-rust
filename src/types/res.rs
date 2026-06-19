use actix_web::{HttpResponse, ResponseError, body::BoxBody};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResError {
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized access to realm: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Conflict occurred: {0}")]
    Conflict(String),
    
    #[error("Unsupported media type: {0}")]
    UnsupportedMedia(String),
    
    #[error("Too many requests. Retry after {0} seconds")]
    TooManyRequests(u64),
    
    #[error("Service temporarily unavailable. Retry after {0} seconds")]
    ServiceUnavailable(u64),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Actix Web Error: {0}")]
    Actix(#[from] actix_web::Error),
}

pub type Http = Result<actix_web::HttpResponse, ResError>;