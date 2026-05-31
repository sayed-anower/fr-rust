use redis::AsyncCommands;

#[derive(Clone)]
pub struct RedisManager;

impl RedisManager {
    pub async fn new(url: String) -> redis::RedisResult<redis::aio::MultiplexedConnection> {
        let client = redis::Client::open(url).unwrap();
        let mut con = client.get_multiplexed_async_connection().await?;
        Ok(con)
    }
}