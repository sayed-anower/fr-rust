use dotenvy::dotenv;
use std::env;

// Initialized env
pub fn load_env() {
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