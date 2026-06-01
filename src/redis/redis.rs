use deadpool_redis::{Config, Runtime, Connection, Pool};
use deadpool_redis::redis::AsyncCommands;
use std::sync::Arc;

#[derive(Clone)]
pub struct RedisManager {
    pool: Pool,
}

impl RedisManager {
    pub fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cfg = Config::from_url(url)?;
        
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        
        Ok(RedisManager { pool })
    }

    pub async fn get_connection(&self) -> Result<Connection, Box<dyn std::error::Error>> {
        let conn = self.pool.get().await?;
        Ok(conn)
    }
}
