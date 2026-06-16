use actix_files::NamedFile;
use actix_web::{Error, HttpResponse, Result, HttpRequest};
use actix_multipart::Multipart;
use actix_web::http::{header, Method};
use futures_util::{StreamExt, TryStreamExt};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt, BufWriter}; // Brought AsyncSeekExt into scope
use bytes::Bytes;
use brotli::CompressorWriter;
use tokio_util::io::ReaderStream;
// Use public prepended API instead of private frame modules
use lz4_flex::{compress_prepended, decompress_size_prepended};

// ============== HIGH-PERFORMANCE RESPONSES ==============

/// 200 OK with zero-copy static string (fastest)
#[inline]
pub fn http_ok_static(msg: &'static str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(msg)
}

/// 200 OK with streaming body (for large responses)
#[inline]
pub fn http_ok_stream(stream: impl StreamExt<Item = Result<Bytes, Error>> + 'static) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .streaming(stream)
}

/// 204 No Content (for PUT/POST without response)
#[inline]
pub fn http_no_content() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

/// 201 Created with location header
#[inline]
pub fn http_created(location: &str) -> HttpResponse {
    HttpResponse::Created()
        .insert_header((header::LOCATION, location))
        .finish()
}

/// 202 Accepted for async processing
#[inline]
pub fn http_accepted() -> HttpResponse {
    HttpResponse::Accepted().finish()
}

/// 206 Partial Content (for range requests)
#[inline]
pub fn http_partial_content(data: Bytes, range: &str, total_len: u64) -> HttpResponse {
    HttpResponse::PartialContent()
        .insert_header((header::CONTENT_RANGE, format!("bytes {}/{}", range, total_len)))
        .content_type("application/octet-stream")
        .body(data)
}

/// 400 Bad Request with static message (fast)
#[inline]
pub fn http_bad_static(msg: &'static str) -> HttpResponse {
    HttpResponse::BadRequest()
        .content_type("text/plain")
        .body(msg)
}

/// 401 Unauthorized
#[inline]
pub fn http_unauthorized(realm: &str) -> HttpResponse {
    HttpResponse::Unauthorized()
        .insert_header((header::WWW_AUTHENTICATE, format!("Bearer realm=\"{}\"", realm)))
        .finish()
}

/// 403 Forbidden
#[inline]
pub fn http_forbidden(msg: &str) -> HttpResponse {
    HttpResponse::Forbidden().body(msg.to_string())
}

/// 404 Not Found
#[inline]
pub fn http_not_found(msg: &str) -> HttpResponse {
    HttpResponse::NotFound().body(msg.to_string())
}

/// 405 Method Not Allowed
#[inline]
pub fn http_method_not_allowed(allowed_methods: &[Method]) -> HttpResponse {
    let methods = allowed_methods.iter()
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    
    HttpResponse::MethodNotAllowed()
        .insert_header((header::ALLOW, methods))
        .finish()
}

/// 409 Conflict
#[inline]
pub fn http_conflict(msg: &str) -> HttpResponse {
    HttpResponse::Conflict().body(msg.to_string())
}

/// 415 Unsupported Media Type
#[inline]
pub fn http_unsupported_media(msg: &str) -> HttpResponse {
    HttpResponse::UnsupportedMediaType().body(msg.to_string())
}

/// 429 Too Many Requests
#[inline]
pub fn http_too_many_requests(retry_after_secs: u64) -> HttpResponse {
    HttpResponse::TooManyRequests()
        .insert_header((header::RETRY_AFTER, retry_after_secs))
        .finish()
}

/// 500 Internal Server Error
#[inline]
pub fn http_server_error(msg: &str) -> HttpResponse {
    HttpResponse::InternalServerError().body(msg.to_string())
}

/// 503 Service Unavailable
#[inline]
pub fn http_service_unavailable(retry_after_secs: u64) -> HttpResponse {
    HttpResponse::ServiceUnavailable()
        .insert_header((header::RETRY_AFTER, retry_after_secs))
        .finish()
}

// ============== ZERO-COPY FILE SENDING ==============

/// Send file with memory-mapped I/O (fastest for static files)
pub async fn send_file_fast(path: &str, req: &HttpRequest) -> Result<NamedFile> {
    let file = NamedFile::open_async(path).await?;
    
    if let Some(accept_encoding) = req.headers().get(header::ACCEPT_ENCODING) {
        if accept_encoding.to_str().unwrap_or("").contains("br") {
            let br_path = format!("{}.br", path);
            if Path::new(&br_path).exists() {
                // Fixed ContentEncoding variation to Brotli
                return Ok(NamedFile::open_async(br_path).await?
                    .set_content_encoding(header::ContentEncoding::Brotli));
            }
        }
    }
    
    Ok(file.use_etag(true).use_last_modified(true))
}

/// Stream large file with chunked transfer (for progressive loading)
/// Stream large file with chunked transfer (for progressive loading)
pub async fn stream_file_chunked(path: &str, chunk_size: usize) -> Result<HttpResponse, Error> {
    let file = File::open(path).await?;
    
    // ReaderStream automatically manages a highly optimized BytesMut buffer.
    // We use `with_capacity` to respect your chunk_size parameter.
    let stream = ReaderStream::with_capacity(file, chunk_size)
        // actix-web expects an actix_web::Error, so we map the std::io::Error
        .map_err(|e| actix_web::error::ErrorInternalServerError(e));
    
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .streaming(stream))
}

/// Send file with range support (for resume downloads)
pub async fn send_file_range(path: &str, range: Option<&str>) -> Result<HttpResponse, Error> {
    let file = tokio::fs::File::open(path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    
    if let Some(range_str) = range {
        if let Some((start, end)) = parse_range(range_str, file_size) {
            let len = end - start + 1;
            // Note: If memory exhaustion is a concern, range requests should stream rather than read_exact into RAM.
            let mut buf = vec![0u8; len as usize];
            
            let mut reader = tokio::io::BufReader::new(file);
            reader.seek(std::io::SeekFrom::Start(start)).await?;
            reader.read_exact(&mut buf).await?;
            
            return Ok(HttpResponse::PartialContent()
                .insert_header((header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size)))
                .insert_header((header::CONTENT_LENGTH, len))
                .content_type("application/octet-stream")
                .body(buf));
        }
    }
    
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_LENGTH, file_size))
        .content_type("application/octet-stream")
        .body(())
    )
}

// ============== COMPRESSED RESPONSES ==============

/// Brotli compressed response (best compression ratio)
pub fn http_brotli(data: &[u8], quality: u32) -> HttpResponse {
    // Fixed: Take ownership of the vector in CompressorWriter and extract it afterwards
    // Added capacity hinting to prevent vector resizing
    let mut writer = CompressorWriter::new(Vec::with_capacity(data.len() / 2), 4096, quality as u32, 22);
    use std::io::Write;
    writer.write_all(data).unwrap();
    writer.flush().unwrap();
    
    let compressed = writer.into_inner();
    
    HttpResponse::Ok()
        .insert_header((header::CONTENT_ENCODING, "br"))
        .content_type("application/octet-stream")
        .body(compressed)
}

/// LZ4 compressed response (fastest decompression)
pub fn http_lz4(data: &[u8]) -> HttpResponse {
    // Fixed: Uses public prepended API
    let compressed = compress_prepended(data);
    
    HttpResponse::Ok()
        .insert_header((header::CONTENT_ENCODING, "lz4"))
        .content_type("application/octet-stream")
        .body(compressed)
}

// ============== REQUEST PARSING ==============

/// Parse multipart form with streaming (high memory efficiency)
pub async fn parse_multipart_stream<F>(mut payload: Multipart, mut handler: F) -> Result<Vec<Bytes>, Error>
where
    F: FnMut(String, Bytes) -> Result<(), Error>,
{
    let mut results = Vec::new();
    
    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field.name().map_or("unknown", |n| n).to_string();
        let mut data = Vec::new();
        
        while let Some(chunk) = field.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
            handler(name.clone(), chunk)?;
        }
        
        results.push(Bytes::from(data));
    }
    
    Ok(results)
}

/// Parse JSON with zero-copy (fastest)
pub fn parse_json_fast<T: serde::de::DeserializeOwned>(data: &Bytes) -> Result<T, Error> {
    serde_json::from_slice(data).map_err(|e| actix_web::error::ErrorBadRequest(e))
}

// ============== UTILITIES ==============

#[inline]
fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    let range_str = range_str.trim_start_matches("bytes=");
    let parts: Vec<&str> = range_str.split('-').collect();
    
    if parts.len() != 2 {
        return None;
    }
    
    let start = parts[0].parse::<u64>().ok()?;
    let end = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1].parse::<u64>().ok()?
    };
    
    if start > end || end >= file_size {
        None
    } else {
        Some((start, end))
    }
}

// ============== EXTENDED UPLOAD FUNCTIONS ==============

/// Upload with progress tracking
pub async fn upload_with_progress<P: AsRef<Path>, F>(
    mut payload: Multipart, 
    target_dir: P,
    mut progress_cb: F
) -> Result<Vec<String>, Error>
where
    F: FnMut(&str, u64, u64),
{
    let mut uploaded_files = Vec::new();
    let base_path = target_dir.as_ref();
    
    if !base_path.exists() {
        tokio::fs::create_dir_all(base_path).await?;
    }
    
    while let Some(mut field) = payload.try_next().await? {
        let filename = field
            .content_disposition()
            .expect("Sending File Failed!")
            .get_filename()
            .map_or_else(|| "unknown".to_string(), |f| f.to_string());
        
        let filepath = base_path.join(&filename);
        let mut file = File::create(&filepath).await?;
        let mut writer = BufWriter::new(&mut file);
        let mut total_bytes = 0u64;
        
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            writer.write_all(&data).await?;
            total_bytes += data.len() as u64;
            progress_cb(&filename, total_bytes, 0);
        }
        
        writer.flush().await?;
        uploaded_files.push(filename);
    }
    
    Ok(uploaded_files)
}

/// Upload with immediate streaming to disk (lowest memory usage)
pub async fn upload_streaming<P: AsRef<Path>>(payload: Multipart, target_dir: P) -> Result<Vec<String>, Error> {
    let mut uploaded_files = Vec::new();
    let base_path = target_dir.as_ref();
    
    if !base_path.exists() {
        tokio::fs::create_dir_all(base_path).await?;
    }
    
    let mut stream = payload;
    while let Some(mut field) = stream.try_next().await? {
        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .unwrap_or("unknown")
            .to_string();
        
        let filepath = base_path.join(&filename);
        let mut file = File::create(filepath).await?;
        
        while let Some(chunk) = field.next().await {
            file.write_all(&chunk?).await?;
        }
        
        uploaded_files.push(filename);
    }
    
    Ok(uploaded_files)
}
