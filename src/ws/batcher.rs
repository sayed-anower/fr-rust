use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tokio_postgres::{NoTls, Error as PgError};
use deadpool_postgres::{Pool, Config, ManagerConfig, RecyclingMethod, Runtime};
use std::sync::Arc;
use std::collections::VecDeque;
use thiserror::Error;
use futures_util::SinkExt; // Required for copy_writer.send()

#[derive(Error, Debug)]
pub enum BatcherError {
    #[error("PostgreSQL error: {0}")]
    Pg(#[from] PgError),
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("Pool creation error: {0}")]
    CreatePool(#[from] deadpool_postgres::CreatePoolError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, BatcherError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub time: u64,
    pub id: String,
    pub content: String,
}

pub struct MsgBatcher {
    pool: Pool,
    buffer: Arc<Mutex<VecDeque<Message>>>,
    batch_size: usize,
    flush_interval: Duration,
    max_buffer_size: usize,
    running: Arc<Mutex<bool>>,
}

impl MsgBatcher {
    /// Create new batcher with PostgreSQL connection string
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.url = Some(database_url.to_string());
        
        // Correctly assign recycling_method to ManagerConfig
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        // Use standard initialization to avoid missing QueueMode types
        cfg.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 16,
            ..Default::default()
        });
        
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        
        // Initialize table
        let client = pool.get().await?;
        client.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id BIGSERIAL PRIMARY KEY,
                time BIGINT NOT NULL,
                user_id TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            &[],
        ).await?;
        
        // Check if table is UNLOGGED for max performance
        client.execute(
            "ALTER TABLE IF EXISTS messages SET UNLOGGED",
            &[],
        ).await?;
        
        Ok(Self {
            pool,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(10000))),
            batch_size: 5000,
            flush_interval: Duration::from_secs(5),
            max_buffer_size: 10000,
            running: Arc::new(Mutex::new(true)),
        })
    }

    /// Configure batch size (default: 5000)
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Configure flush interval in seconds (default: 5)
    pub fn with_flush_interval(mut self, seconds: u64) -> Self {
        self.flush_interval = Duration::from_secs(seconds);
        self
    }

    /// Configure max buffer size (default: 10000)
    pub fn with_max_buffer(mut self, size: usize) -> Self {
        self.max_buffer_size = size;
        self
    }

    /// Append message to buffer (non-blocking, ~microseconds)
    pub async fn append(&self, msg: Message) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        buffer.push_back(msg);
        
        let len = buffer.len();
        
        // Emergency flush if buffer is too large
        if len >= self.max_buffer_size {
            drop(buffer);
            self.flush().await?;
        } else if len >= self.batch_size {
            let batch: Vec<Message> = buffer.drain(..len).collect();
            drop(buffer);
            self.flush_batch(batch).await?;
        }
        
        Ok(())
    }

    /// Manually flush all pending messages
    pub async fn flush(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }
        
        let batch: Vec<Message> = buffer.drain(..).collect();
        drop(buffer);
        
        self.flush_batch(batch).await
    }

    /// Background worker - call this in your main function
    pub async fn run_background(&self) -> Result<()> {
        let buffer = Arc::clone(&self.buffer);
        let pool = self.pool.clone();
        let batch_size = self.batch_size;
        let flush_interval = self.flush_interval;
        let running = Arc::clone(&self.running);
        let mut interval = interval(flush_interval);

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                
                // Check if we should stop
                let should_stop = !*running.lock().await;
                if should_stop {
                    break;
                }
                
                let mut guard = buffer.lock().await;
                if guard.is_empty() {
                    continue;
                }
                
                // Drain in chunks for efficiency
                let batches: Vec<Vec<Message>> = guard
                    .drain(..)
                    .collect::<Vec<Message>>()
                    .chunks(batch_size)
                    .map(|chunk| chunk.to_vec())
                    .collect();
                drop(guard);
                
                // Process each chunk
                for batch in batches {
                    if let Err(e) = Self::bulk_insert(&pool, batch).await {
                        eprintln!("Failed to flush batch: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Stop background worker gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        drop(running);
        
        // Final flush
        self.flush().await?;
        Ok(())
    }

    /// Fastest bulk insert using COPY
    async fn bulk_insert(pool: &Pool, messages: Vec<Message>) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }
        
        let client = pool.get().await?;
        
        // Use COPY for maximum performance
        let copy_stmt = "COPY messages (time, user_id, content) FROM STDIN (FORMAT CSV, DELIMITER ',')";
        let mut copy_writer = client.copy_in(copy_stmt).await?;
        
        // Pre-allocate buffer for performance
        let mut batch_buffer = String::with_capacity(messages.len() * 256);
        
        for msg in &messages {
            batch_buffer.push_str(&msg.time.to_string());
            batch_buffer.push(',');
            batch_buffer.push_str(&msg.id);
            batch_buffer.push_str(",\"");
            
            // Allocation-free CSV escaping
            for c in msg.content.chars() {
                if c == '"' {
                    batch_buffer.push_str("\"\"");
                } else {
                    batch_buffer.push(c);
                }
            }
            batch_buffer.push_str("\"\n");
        }
        
        // Efficiently pass the allocated string to the Sink
        copy_writer.send(bytes::Bytes::from(batch_buffer)).await?;
        
        // Correctly pin the writer to call finish()
        std::pin::pin!(copy_writer).finish().await?;
        
        Ok(())
    }

    async fn flush_batch(&self, messages: Vec<Message>) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }
        Self::bulk_insert(&self.pool, messages).await
    }

    /// Get current buffer size
    pub async fn buffer_size(&self) -> usize {
        self.buffer.lock().await.len()
    }
}
