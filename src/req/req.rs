// Macro for mapping basic routes safely
#[macro_export]
macro_rules! route {
    ($method:ident, $path:literal, $handler:expr) => {
        ::actix_web::web::$method().to($handler)
    };
    ($method:ident, $path:literal, $handler:expr, $($extra:tt)*) => {
        ::actix_web::web::$method().to($handler).$($extra)*
    };
}

// Generate the standard HTTP verb helper macros
#[macro_export] macro_rules! get {     ($path:literal, $handler:expr $(, $($extra:tt)*)?) => { $crate::route!(get, $path, $handler $(, $($extra)*)?) }; }
#[macro_export] macro_rules! post {    ($path:literal, $handler:expr $(, $($extra:tt)*)?) => { $crate::route!(post, $path, $handler $(, $($extra)*)?) }; }
#[macro_export] macro_rules! put {     ($path:literal, $handler:expr $(, $($extra:tt)*)?) => { $crate::route!(put, $path, $handler $(, $($extra)*)?) }; }
#[macro_export] macro_rules! delete {  ($path:literal, $handler:expr $(, $($extra:tt)*)?) => { $crate::route!(delete, $path, $handler $(, $($extra)*)?) }; }
#[macro_export] macro_rules! patch {   ($path:literal, $handler:expr $(, $($extra:tt)*)?) => { $crate::route!(patch, $path, $handler $(, $($extra)*)?) }; }

// Unified Routing Configurator Macro
#[macro_export]
macro_rules! routes {
    ($cfg:ident => { $($item:expr),* $(,)? }) => {
        {
            $(
                $cfg = $cfg.service($item);
            )*
        }
    };
}

// Scope Builder Macro (Resolves without needing raw cfg)
#[macro_export]
macro_rules! scope {
    ($path:literal $(, .$extra:ident($($args:tt)*))* => { $($routes:expr),* $(,)? }) => {
        ::actix_web::web::scope($path)
            $(.$extra($($args)*))*
            $(.service($routes))*
    };
}

// Resource Builder Macro
#[macro_export]
macro_rules! resource {
    ($path:literal $(, .$extra:ident($($args:tt)*))* => { $($routes:expr),* $(,)? }) => {
        ::actix_web::web::resource($path)
            $(.$extra($($args)*))*
            $(.route($routes))*
    };
}
