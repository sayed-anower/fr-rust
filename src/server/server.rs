use dotenvy::dotenv;
use std::env;

// Initialized env
pub fn load_env() {
    dotenv().ok();
}
// Get var or default
pub fn env_or_default(name: &str, default_value: &str) -> &str {
    env::var(name)
    .as_str()
    .unwrap_or_else(|_| default_value)
}
// Get var
pub fn env(name: &str) -> &str {
    env::var(name)
    .as_str()
    .expect(&format!("Failed to load {}", name))
}