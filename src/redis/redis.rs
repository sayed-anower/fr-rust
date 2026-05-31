use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{broadcast, mpsc};

// Type aliases for cleaner code signatures
pub type RedisResult<T> = Result<T, redis::RedisError>;
use redis::{Client, AsyncCommands, ErrorKind, RedisError};
use deadpool_redis::{Pool, Config as DeadpoolConfig, PoolConfig, Runtime};

/// =======================================================
/// HIGH PERFORMANCE REDIS FRAMEWORK
/// =======================================================

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    /// Max Redis channels handled by one background pubsub task
    pub max_channels_per_pubsub: usize,
    /// Flush a partially-filled pubsub batch every N ms
    pub flush_interval_ms: u64,
    /// deadpool max size (command connections)
    pub pool_max_size: usize,
}

#[derive(Clone)]
pub struct RedisManager {
    #[allow(dead_code)]
    client: Client,
    pool: Pool,
    #[allow(dead_code)]
    config: RedisConfig,
    /// In-memory broadcast bus: Redis channel -> local sender
    local_channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    /// Request new Redis subscriptions here
    sub_tx: mpsc::Sender<String>,
}

impl RedisManager {
    pub async fn new(config: RedisConfig) -> RedisResult<Self> {
        let client = Client::open(config.url.as_str())?;

        let mut dp_cfg = DeadpoolConfig::from_url(&config.url);
        dp_cfg.pool = Some(PoolConfig {
            max_size: config.pool_max_size,
            ..Default::default()
        });

        let pool = dp_cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| {
                RedisError::from((
                    ErrorKind::IoError,
                    "Failed to create Redis pool",
                    e.to_string(),
                ))
            })?;

        // Warm-up connection check
        let _ = pool.get().await.map_err(|e| {
            RedisError::from((
                ErrorKind::IoError,
                "Pool warm-up connection failed",
                e.to_string(),
            ))
        })?;

        let local_channels = Arc::new(RwLock::new(HashMap::new()));
        let (sub_tx, sub_rx) = mpsc::channel::<String>(4096);

        tokio::spawn(pubsub_coordinator(
            sub_rx,
            client.clone(),
            local_channels.clone(),
            config.max_channels_per_pubsub,
            Duration::from_millis(config.flush_interval_ms),
        ));

        Ok(Self {
            client,
            pool,
            config,
            local_channels,
            sub_tx,
        })
    }

    fn serialize<T: Serialize>(value: T) -> RedisResult<String> {
        serde_json::to_string(&value).map_err(|e| {
            RedisError::from((
                ErrorKind::TypeError,
                "JSON serialization error",
                e.to_string(),
            ))
        })
    }

    fn deserialize<T: DeserializeOwned>(value: &str) -> RedisResult<T> {
        serde_json::from_str(value).map_err(|e| {
            RedisError::from((
                ErrorKind::TypeError,
                "JSON deserialization error",
                e.to_string(),
            ))
        })
    }

    async fn conn(&self) -> RedisResult<deadpool_redis::Connection> {
        self.pool.get().await.map_err(|e| {
            RedisError::from((
                ErrorKind::IoError,
                "Redis pool exhausted",
                e.to_string(),
            ))
        })
    }

    // Basic KV
    pub async fn set<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.set(key, Self::serialize(value)?).await
    }

    pub async fn set_ttl<T>(&self, key: &str, value: T, ttl_seconds: u64) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.set_ex(key, Self::serialize(value)?, ttl_seconds).await
    }

    pub async fn get<T>(&self, key: &str) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn().await?;
        let val: Option<String> = conn.get(key).await?;
        match val {
            Some(v) => Ok(Some(Self::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub async fn del(&self, key: &str) -> RedisResult<()> {
        let mut conn = self.conn().await?;
        conn.del(key).await.map(|_| ())
    }

    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.conn().await?;
        conn.exists(key).await
    }

    pub async fn expire(&self, key: &str, ttl_seconds: i64) -> RedisResult<()> {
        let mut conn = self.conn().await?;
        conn.expire(key, ttl_seconds).await.map(|_| ())
    }

    pub async fn ttl(&self, key: &str) -> RedisResult<i64> {
        let mut conn = self.conn().await?;
        conn.ttl(key).await
    }

    // Hash Map Actions
    pub async fn hset<T>(&self, key: &str, field: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.hset(key, field, Self::serialize(value)?).await.map(|_| ())
    }

    pub async fn hget<T>(&self, key: &str, field: &str) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn().await?;
        let val: Option<String> = conn.hget(key, field).await?;
        match val {
            Some(v) => Ok(Some(Self::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub async fn hdel(&self, key: &str, field: &str) -> RedisResult<()> {
        let mut conn = self.conn().await?;
        conn.hdel(key, field).await.map(|_| ())
    }

    // Lists
    pub async fn lpush<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.lpush(key, Self::serialize(value)?).await.map(|_| ())
    }

    pub async fn rpush<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.rpush(key, Self::serialize(value)?).await.map(|_| ())
    }

    pub async fn lrange<T>(&self, key: &str, start: isize, stop: isize) -> RedisResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn().await?;
        let vals: Vec<String> = conn.lrange(key, start, stop).await?;
        vals.into_iter().map(|v| Self::deserialize(&v)).collect()
    }

    // Sets
    pub async fn sadd<T>(&self, key: &str, value: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        conn.sadd(key, Self::serialize(value)?).await.map(|_| ())
    }

    pub async fn smembers<T>(&self, key: &str) -> RedisResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.conn().await?;
        let vals: Vec<String> = conn.smembers(key).await?;
        vals.into_iter().map(|v| Self::deserialize(&v)).collect()
    }

    // Pipelines
    pub async fn pipeline_set(&self, data: Vec<(&str, String)>) -> RedisResult<()> {
        let mut conn = self.conn().await?;
        let mut pipe = redis::pipe();
        for (k, v) in &data {
            pipe.set(*k, v);
        }
        
        // Fix: Explicitly declare the type and pass the un-dereferenced connection
        let _: Vec<redis::Value> = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    // Cache Pattern
    pub async fn cache_or_fetch<T, F, Fut>(&self, key: &str, ttl: u64, fetcher: F) -> RedisResult<T>
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

    // Pub/Sub
    pub async fn publish<T>(&self, channel: &str, message: T) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.conn().await?;
        let msg = Self::serialize(message)?;
        conn.publish(channel, msg).await.map(|_| ())
    }

    /// Fixed Race Condition & Guard Lifetimes
    pub async fn subscribe_channel(&self, channel: &str) -> broadcast::Receiver<String> {
        // Fix: Execute the borrow inside a confined scope so the guard correctly drops
        // before we reach the async/await `.send()` call.
        let (rx, is_new) = {
            let mut guard = self.local_channels.write().unwrap();
            
            match guard.entry(channel.to_string()) {
                Entry::Occupied(entry) => (entry.get().subscribe(), false),
                Entry::Vacant(entry) => {
                    let (tx, rx) = broadcast::channel(1024);
                    entry.insert(tx);
                    (rx, true)
                }
            }
        };

        // Guard is now dropped securely, Future remains Send
        if is_new {
            let _ = self.sub_tx.send(channel.to_string()).await;
        }
        
        rx
    }

    pub async fn subscribe<F>(&self, channel: &str, mut handler: F) -> RedisResult<()>
    where
        F: FnMut(String) + Send + 'static,
    {
        let mut rx = self.subscribe_channel(channel).await;
        tokio::spawn(async move {
            // Fix: Catch RecvError::Lagged to prevent silent exit
            loop {
                match rx.recv().await {
                    Ok(msg) => handler(msg),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(())
    }

    pub async fn subscribe_json<T, F>(&self, channel: &str, mut handler: F) -> RedisResult<()>
    where
        T: DeserializeOwned + Send + 'static,
        F: FnMut(T) + Send + 'static,
    {
        let mut rx = self.subscribe_channel(channel).await;
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if let Ok(serde_str) = Self::deserialize::<String>(&msg) {
                            if let Ok(data) = Self::deserialize::<T>(&serde_str) {
                                handler(data);
                                continue;
                            }
                        }
                        // Fallback direct deserialize
                        if let Ok(data) = Self::deserialize::<T>(&msg) {
                            handler(data);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(())
    }

    pub async fn unsubscribe(&self, channel: &str) {
        let mut guard = self.local_channels.write().unwrap();
        guard.remove(channel);
    }
}

async fn pubsub_coordinator(
    mut rx: mpsc::Receiver<String>,
    client: Client,
    local_channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    max_channels: usize,
    flush_interval: Duration,
) {
    let mut pending: Vec<String> = Vec::with_capacity(max_channels);
    let mut interval = tokio::time::interval(flush_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            Some(channel) = rx.recv() => {
                pending.push(channel);
                if pending.len() >= max_channels {
                    // Fix: Use replace instead of take to maintain capacity
                    let batch = std::mem::replace(&mut pending, Vec::with_capacity(max_channels));
                    spawn_pubsub_worker(client.clone(), batch, local_channels.clone());
                }
            }
            _ = interval.tick() => {
                if !pending.is_empty() {
                    let batch = std::mem::replace(&mut pending, Vec::with_capacity(max_channels));
                    spawn_pubsub_worker(client.clone(), batch, local_channels.clone());
                }
            }
        }
    }
}

fn spawn_pubsub_worker(
    client: Client,
    channels: Vec<String>,
    local_channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
) {
    tokio::spawn(async move {
        let mut backoff = Duration::from_secs(1);
        loop {
            let mut pubsub = match client.get_async_pubsub().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("PubSub connection failed: {e}. Retrying...");
                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
                    continue;
                }
            };
            
            backoff = Duration::from_secs(1);
            let mut subscribed_any = false;
            for ch in &channels {
                if let Err(e) = pubsub.subscribe(ch).await {
                    eprintln!("Failed to subscribe to {ch}: {e}");
                } else {
                    subscribed_any = true;
                }
            }

            if !subscribed_any { return; }

            let mut stream = pubsub.on_message();
            while let Some(msg) = stream.next().await {
                let channel = msg.get_channel_name();
                if let Ok(p) = msg.get_payload::<String>() {
                    if let Ok(guard) = local_channels.read() {
                        if let Some(tx) = guard.get(channel) {
                            let _ = tx.send(p);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}