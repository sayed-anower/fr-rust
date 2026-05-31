pub mod redis;
pub use redis::{RedisManager};

// Re-export AsyncCommands from the external redis crate
pub use ::redis::AsyncCommands;
