use deadpool_redis::{Config, Runtime, Connection, Pool};

use deadpool_redis::redis;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::redis::Client;

#[derive(Clone)]
pub struct RedisManager {
    url: String,
    pool: Pool,
}

impl RedisManager {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        Ok(RedisManager { 
            url: url.to_string(), 
            pool 
        })
    }

    pub async fn get_connection(&self) -> anyhow::Result<Connection> {
        let conn = self.pool.get().await?;
        Ok(conn)
    }


   pub async fn publish(&self, event_name: &str, content: &str) -> anyhow::Result<()> {
      let mut conn = self.pool.get().await?;

      conn.publish::<_, _, ()>(event_name, content).await?;
      Ok(())
   }

    // FIXED: Changed return type signature to `redis::aio::PubSubStream` 
    pub async fn subscribe(&self, event_name: &str) -> anyhow::Result<redis::aio::PubSubStream> {
        let client = Client::open(self.url.as_str())?;
        let mut pubsub = client.get_async_pubsub().await?;

        pubsub.subscribe(event_name).await?;

        // Yields a PubSubStream consuming the parent struct
        Ok(pubsub.into_on_message())
    }
}

