use deadpool_redis::{Config, Runtime, Connection, Pool};
use deadpool_redis::redis::AsyncCommands;

impl RedisManager {
   
    pub fn new(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cfg = Config::from_url(url).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(RedisManager { pool })
    }

  
    pub async fn get_connection(&self) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.pool.get().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(conn)
    }
}

