// First, define a macro for creating routes with service
#[macro_export]
macro_rules! route {
    // Basic route without extras
    ($method:ident, $path:literal, $handler:expr) => {
        ::actix_web::web::$method().to($handler)
    };
    // Route with extras (guard, middleware, etc.)
    ($method:ident, $path:literal, $handler:expr, $($extra:tt)*) => {
        ::actix_web::web::$method().to($handler).$($extra)*
    };
}

// Now define individual method macros that use route!
#[macro_export]
macro_rules! get {
    ($path:literal, $handler:expr) => {
        $crate::route!(get, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(get, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! post {
    ($path:literal, $handler:expr) => {
        $crate::route!(post, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(post, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! put {
    ($path:literal, $handler:expr) => {
        $crate::route!(put, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(put, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! delete {
    ($path:literal, $handler:expr) => {
        $crate::route!(delete, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(delete, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! patch {
    ($path:literal, $handler:expr) => {
        $crate::route!(patch, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(patch, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! head {
    ($path:literal, $handler:expr) => {
        $crate::route!(head, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(head, $path, $handler, $($extra)*)
    };
}

#[macro_export]
macro_rules! options {
    ($path:literal, $handler:expr) => {
        $crate::route!(options, $path, $handler)
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::route!(options, $path, $handler, $($extra)*)
    };
}

// Finally, the cfg macro that uses service()
#[macro_export]
macro_rules! cfg {
    // Single route
    ($route:expr) => {
        $crate::cfg.service($route)
    };
    // Multiple routes
    ($($route:expr),* $(,)?) => {
        $crate::cfg
            $(.service($route))*
    };
}

#[macro_export]
macro_rules! scope {
    ($path:literal, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::scope($path)
                .$($content)*
        )
    };
    ($path:literal, $($extra:tt)*, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::scope($path)
                .$($extra)*
                .$($content)*
        )
    };
}

#[macro_export]
macro_rules! resource {
    ($path:literal, $($content:tt)*) => {
        $crate::cfg.service(
            ::actix_web::web::resource($path)
                .$($content)*
        )
    };
    ($path:literal, $($extra:tt)*, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::resource($path)
                .$($extra)*
                .$($content)*
        )
    };
}