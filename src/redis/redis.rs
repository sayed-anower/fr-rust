use redis::AsyncCommands;

#[derive(Clone)]
pub struct RedisManager {
    // 1. Store the connection inside the manager
    pub connection: redis::aio::MultiplexedConnection,
}

impl RedisManager {
    // 2. Return Self instead of just the connection
    pub async fn new(url: String) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self { connection })
    }
}