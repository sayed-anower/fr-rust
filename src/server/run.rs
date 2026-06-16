#[macro_export]
macro_rules! run_server {
    (
        state: $state:expr,
        config: $config:expr,
        addr: $addr:expr
        $(, app: { $($app_extra:tt)* })?          // Extra stuff after App::new()
        $(, server: { $($server_extra:tt)* })?    // Extra stuff after .bind()
        $(,)?                                      // Optional trailing comma
    ) => {{
        use actix_web::{web, App, HttpServer};
        use actix_web::middleware::{Compress, NormalizePath, TrailingSlash};
        use std::net::SocketAddr;

        let bind_addr: SocketAddr = $addr.parse()
            .expect("Invalid socket address format");

        log::info!("Starting server on {}", bind_addr);

        HttpServer::new(move || {
            let mut app = App::new()
                .app_data(web::Data::new($state.clone()))
                // === Security & Performance Middleware (always included) ===
                .wrap(NormalizePath::new(TrailingSlash::Trim))
                .wrap(Compress::default());
            // User can add more middleware / services / app_data here
            $(
                app = app $($app_extra)*;
            )?
            app.configure($config)
        })
        .bind(bind_addr)?
        $(
            .$($server_extra)*
        )?
        .run()
        .await
    }};
}