use deadpool_redis::{Config, Runtime, Connection, Pool};
use deadpool_redis::redis;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::redis::Client;
use thiserror::Error;

/// Custom error type for Redis operations
#[derive(Error, Debug)]
pub enum RedisManagerError {
    #[error("Failed to create Redis pool: {0}")]
    CreatePool(#[from] deadpool_redis::CreatePoolError),

    #[error("Failed to get connection from pool: {0}")]
    Pool(#[from] deadpool_redis::PoolError),

    #[error("Redis command error: {0}")]
    Redis(#[from] redis::RedisError),
}

/// A specialized Result type for convenience
pub type Result<T> = std::result::Result<T, RedisManagerError>;

#[derive(Clone)]
pub struct RedisManager {
    url: String,
    pool: Pool,
}

impl RedisManager {
    pub fn new(url: &str) -> Result<Self> {
        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        Ok(RedisManager { 
            url: url.to_string(), 
            pool 
        })
    }

    pub async fn get_connection(&self) -> Result<Connection> {
        let conn = self.pool.get().await?;
        Ok(conn)
    }

    pub async fn publish(&self, event_name: &str, content: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;

        conn.publish::<_, _, ()>(event_name, content).await?;
        Ok(())
    }

    pub async fn subscribe(&self, event_name: &str) -> Result<redis::aio::PubSubStream> {
        let client = Client::open(self.url.as_str())?;
        let mut pubsub = client.get_async_pubsub().await?;

        pubsub.subscribe(event_name).await?;

        Ok(pubsub.into_on_message())
    }
}