use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use redis::{
    aio::{ConnectionManager, PubSub},
    AsyncCommands, Client, RedisResult,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{broadcast, RwLock};

/// =======================================================
/// HIGH PERFORMANCE REDIS FRAMEWORK
/// - Supports:
///     - GET/SET
///     - TTL
///     - DELETE
///     - HASH
///     - LIST
///     - SETS
///     - PUB/SUB
///     - Multi server communication
///     - Typed JSON storage
///     - In-memory subscriptions
/// =======================================================

#[derive(Clone)]
pub struct RedisManager {
    client: Client,
    conn: ConnectionManager,

    /// local event bus
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl RedisManager {
    /// Create framework
    pub async fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        let conn = ConnectionManager::new(client.clone()).await?;

        Ok(Self {
            client,
            conn,
            channels: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// ===================================================
    /// BASIC KV
    /// ===================================================

    pub async fn set<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.set(key, value).await
    }

    pub async fn set_ttl<T>(
        &self,
        key: &str,
        value: T,
        ttl_seconds: u64,
    ) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.set_ex(key, value, ttl_seconds).await
    }

    pub async fn get<T>(&self, key: &str) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn.clone();

        let val: Option<String> = conn.get(key).await?;

        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v).unwrap())),
            None => Ok(None),
        }
    }

    pub async fn del(&self, key: &str) -> RedisResult<()> {
        let mut conn = self.conn.clone();

        conn.del(key).await
    }

    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.conn.clone();

        conn.exists(key).await
    }

    pub async fn expire(&self, key: &str, ttl_seconds: i64) -> RedisResult<()> {
        let mut conn = self.conn.clone();

        conn.expire(key, ttl_seconds).await
    }

    pub async fn ttl(&self, key: &str) -> RedisResult<i64> {
        let mut conn = self.conn.clone();

        conn.ttl(key).await
    }

    /// ===================================================
    /// HASH
    /// ===================================================

    pub async fn hset<T>(
        &self,
        key: &str,
        field: &str,
        value: T,
    ) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.hset(key, field, value).await
    }

    pub async fn hget<T>(
        &self,
        key: &str,
        field: &str,
    ) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn.clone();

        let val: Option<String> = conn.hget(key, field).await?;

        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v).unwrap())),
            None => Ok(None),
        }
    }

    pub async fn hdel(&self, key: &str, field: &str) -> RedisResult<()> {
        let mut conn = self.conn.clone();

        conn.hdel(key, field).await
    }

    /// ===================================================
    /// LIST
    /// ===================================================

    pub async fn lpush<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.lpush(key, value).await
    }

    pub async fn rpush<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.rpush(key, value).await
    }

    pub async fn lrange<T>(
        &self,
        key: &str,
        start: isize,
        stop: isize,
    ) -> RedisResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn.clone();

        let vals: Vec<String> = conn.lrange(key, start, stop).await?;

        Ok(vals
            .into_iter()
            .map(|v| serde_json::from_str(&v).unwrap())
            .collect())
    }

    /// ===================================================
    /// SET
    /// ===================================================

    pub async fn sadd<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let value = serde_json::to_string(&value).unwrap();

        conn.sadd(key, value).await
    }

    pub async fn smembers<T>(&self, key: &str) -> RedisResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn.clone();

        let vals: Vec<String> = conn.smembers(key).await?;

        Ok(vals
            .into_iter()
            .map(|v| serde_json::from_str(&v).unwrap())
            .collect())
    }

    /// ===================================================
    /// PUB/SUB
    /// ===================================================

    /// Send message to ALL servers
    pub async fn publish<T>(
        &self,
        channel: &str,
        message: T,
    ) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();

        let msg = serde_json::to_string(&message).unwrap();

        conn.publish(channel, msg).await
    }

    /// Subscribe to a redis channel
    ///
    /// Every server connected will receive message.
    pub async fn subscribe<F>(
        &self,
        channel: &str,
        mut handler: F,
    ) -> RedisResult<()>
    where
        F: FnMut(String) + Send + 'static,
    {
        let client = self.client.clone();
        let channel = channel.to_string();

        tokio::spawn(async move {
            let mut pubsub = client.get_async_pubsub().await.unwrap();

            pubsub.subscribe(&channel).await.unwrap();

            loop {
                let msg = pubsub.get_message().await.unwrap();

                let payload: String = msg.get_payload().unwrap();

                handler(payload);
            }
        });

        Ok(())
    }

    /// Typed subscribe
    pub async fn subscribe_json<T, F>(
        &self,
        channel: &str,
        mut handler: F,
    ) -> RedisResult<()>
    where
        T: DeserializeOwned + Send + 'static,
        F: FnMut(T) + Send + 'static,
    {
        let client = self.client.clone();
        let channel = channel.to_string();

        tokio::spawn(async move {
            let mut pubsub = client.get_async_pubsub().await.unwrap();

            pubsub.subscribe(&channel).await.unwrap();

            loop {
                let msg = pubsub.get_message().await.unwrap();

                let payload: String = msg.get_payload().unwrap();

                let data: T = serde_json::from_str(&payload).unwrap();

                handler(data);
            }
        });

        Ok(())
    }

    /// ===================================================
    /// PIPELINE
    /// ===================================================

    pub async fn pipeline_set(
        &self,
        data: Vec<(&str, &str)>,
    ) -> RedisResult<()> {
        let mut conn = self.conn.clone();

        let mut pipe = redis::pipe();

        for (k, v) in data {
            pipe.cmd("SET").arg(k).arg(v);
        }

        pipe.query_async(&mut conn).await
    }

    /// ===================================================
    /// CACHE PATTERN
    /// ===================================================

    pub async fn cache_or_fetch<T, F, Fut>(
        &self,
        key: &str,
        ttl: u64,
        fetcher: F,
    ) -> RedisResult<T>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some(val) = self.get::<T>(key).await? {
            return Ok(val);
        }

        let data = fetcher().await;

        self.set_ttl(key, &data, ttl).await?;

        Ok(data)
    }
}