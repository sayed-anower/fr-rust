use actix_web::{App, HttpServer, dev::Server, web};
use dotenvy::dotenv;
use std::env;

// Run the server
pub fn run_server<F>(app_config: F, address: &str) -> Result<Server, std::io::Error>
where
    F: Fn(&mut web::ServiceConfig) + Send + Clone + 'static,
{
    let server = HttpServer::new(move || App::new().configure(app_config.clone()))
        .bind(address)?
        .run();

    Ok(server)
}

// Initialized env
pub fn init_env() {
    dotenv().ok();
}
// Get var or default
pub fn env_var_or_default(name: &str, default_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| default_value.to_string())
}
// Get var
pub fn env_var(name: &str) -> String {
    env::var(name).expect(&format!("Failed to load {}", name))
}