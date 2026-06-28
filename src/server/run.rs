#[macro_export]
macro_rules! run {
    (
        // addr is mandatory, the rest are wrapped in $(...)? to make them optional
        addr: $addr:expr
        $(, state: $state:expr )?
        $(, config: $config:expr )?
        $(, configure_app: |$app:ident| $app_body:expr )?
        $(, configure_server: |$server:ident| $server_body:expr )?
        $(,)? // Handles an optional trailing comma at the very end of the macro call
    ) => {{
        use actix_web::{web, App, HttpServer};
        use std::net::SocketAddr;

        let bind_addr: SocketAddr = $addr.parse()
            .expect("Invalid socket address format");

        let mut server = HttpServer::new(move || {
            let mut app = App::new()
                .wrap(actix_web::middleware::NormalizePath::new(actix_web::middleware::TrailingSlash::Trim))
                .wrap(actix_web::middleware::Compress::default());
            
            // Evaluated at compile-time: Only included if state is passed
            $(
                app = app.app_data(web::Data::new($state.clone()));
            )?

            // Evaluated at compile-time: Only included if configure_app is passed
            $(
                let mut $app = app;
                $app = $app_body;
                app = $app;
            )?

            // Evaluated at compile-time: Only fallback to standard config if passed
            $(
                app = app.configure($config);
            )?

            app
        })
        .bind(bind_addr)?;

        // Evaluated at compile-time: Only included if configure_server is passed
        $(
            let mut $server = server;
            $server = $server_body;
            server = $server;
        )?

        server.run().await
    }};
}
