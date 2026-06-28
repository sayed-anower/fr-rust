use dotenvy::dotenv;
use std::env;

// Initialized env
pub fn load_env() {
    dotenv().ok();
}

// Get var or default
pub fn env_or_default(name: &str, default_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| default_value.to_string())
}

// Get var
pub fn env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        eprintln!("Warning: Failed to load env variable '{}'. Falling back to empty string.", name);
        String::new()
    })
}
