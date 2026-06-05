#[macro_export]
macro_rules! http_error {
    ($expression:expr, $msg:expr) => {
        match $expression {
            Ok(val) => val,
            Err(_err) => {
                // Returns an HTTP 400 Bad Request with the custom message immediately
                return HttpResponse::BadRequest().body($msg.to_string());
            }
        }
    };
}
