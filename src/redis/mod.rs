pub mod redis;
pub use redis::{RedisManager, RedisManagerError};

// Re-export AsyncCommands from the external redis crate
pub use deadpool_redis::redis::AsyncCommands;
