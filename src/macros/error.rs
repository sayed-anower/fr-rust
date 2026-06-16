#[macro_export]
macro_rules! err {
    ($expression:expr, $err:expr) => {
        match $expression {
            Ok(val) => val,
            Err(_err) => {
                return $err;
            }
        }
    };
}