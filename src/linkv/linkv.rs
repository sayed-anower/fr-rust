use crate::prelude::*;
use rand::Rng;
use deadpool_redis::redis::AsyncCommands; 
use thiserror::Error;

// --- ERROR HANDLING ---

#[derive(Error, Debug)]
pub enum LinkVError {
    #[error("Redis command error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),
}

pub type Result<T> = std::result::Result<T, LinkVError>;

// --- SERVICE IMPLEMENTATION ---

#[derive(Clone)]
pub struct LinkVConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub ttl_secs: u64,
}

#[derive(Clone)]
pub struct LinkV {
    config: LinkVConfig,
}

impl LinkV {
    pub fn new(config: LinkVConfig) -> Self {
        Self { config }
    }

    pub async fn generate_token(&self, user_id: &str) -> Result<String> {
        let mut token_bytes = vec![0u8; 256];
        rand::rng().fill_bytes(&mut token_bytes);
        let token = hex::encode(token_bytes);
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        
        let mut con = self.config.redis.get_connection().await?;
        
        // Force the type system to register the output as ()
        let _res: () = con.set_ex(&redis_key, "1", self.config.ttl_secs).await?;
        
        Ok(token)
    }

    pub async fn verify_token(&self, user_id: &str, token: &str) -> Result<bool> {
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        
        let mut con = self.config.redis.get_connection().await?;
        let is_valid: bool = con.exists(&redis_key).await?;
        
        if is_valid {
            // Force the type system to register the output as ()
            let _res: () = con.del(&redis_key).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
