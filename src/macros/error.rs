/// Unwraps a Result. If it's an Err, automatically converts it into an 
/// Actix-Web Internal Server Error (500) with a custom string message.
#[macro_export]
macro_rules! http_try_500 {
    ($expression:expr, $msg:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                log::error!("Internal System Failure: {} | Debug: {:?}", $msg, err);
                return Err(actix_web::error::ErrorInternalServerError($msg));
            }
        }
    };
}

/// Unwraps a Result. If it's an Err, automatically returns an HTTP 400 Bad Request
/// response immediately, using fr-rust's built-in `http_bad` helper.
#[macro_export]
macro_rules! http_error {
    ($expression:expr, $msg:expr) => {
        match $expression {
            Ok(val) => val,
            Err(_err) => {
                return http_bad($msg);
            }
        }
    };
}

/// Unwraps an Option. If it is None, returns an HTTP 404 Not Found response immediately.
#[macro_export]
macro_rules! http_unwrap_or_404 {
    ($expression:expr, $msg:expr) => {
        match $expression {
            Some(val) => val,
            None => {
                return Err(actix_web::error::ErrorNotFound($msg));
            }
        }
    };
}

/// Evaluates a boolean condition. If false, short-circuits the execution path
/// and returns an HTTP 401 Unauthorized error with a custom message.
#[macro_export]
macro_rules! http_assert_auth {
    ($condition:expr, $msg:expr) => {
        if !$condition {
            return Err(actix_web::error::ErrorUnauthorized($msg));
        }
    };
}
