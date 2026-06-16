#[macro_export]
macro_rules! get {
    // Basic
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::get().to($handler))
    };
    // With extra chaining (guard, middleware, etc.)
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::get().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! post {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::post().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::post().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! put {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::put().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::put().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! delete {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::delete().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::delete().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! patch {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::patch().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::patch().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! head {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::head().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::head().to(\( handler). \)($extra)*)
    };
}

#[macro_export]
macro_rules! options {
    ($path:literal, $handler:expr) => {
        $crate::cfg.route($path, ::actix_web::web::options().to($handler))
    };
    ($path:literal, $handler:expr, $($extra:tt)*) => {
        $crate::cfg.route($path, ::actix_web::web::options().to(\( handler). \)($extra)*)
    };
}


#[macro_export]
macro_rules! scope {
    ($path:literal, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::scope($path)
                $($content)*
        )
    };
    ($path:literal, $($extra:tt)*, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::scope($path)
                $($extra)*
                $($content)*
        )
    };
}

#[macro_export]
macro_rules! resource {
    ($path:literal, $($content:tt)*) => {
        $crate::cfg.service(
            ::actix_web::web::resource($path)
                $($content)*
        )
    };
    ($path:literal, $($extra:tt)*, { $($content:tt)* }) => {
        $crate::cfg.service(
            ::actix_web::web::resource($path)
                $($extra)*
                $($content)*
        )
    };
}