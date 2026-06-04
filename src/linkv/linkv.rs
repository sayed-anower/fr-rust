use crate::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use deadpool_redis::redis::AsyncCommands; 

// --- ERROR HANDLING ---

#[derive(thiserror::Error, Debug)]
pub enum LinkVError {
    #[error("Redis manager error: {0}")]
    RedisManager(#[from] RedisManagerError),

    #[error("Redis error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),
    
    #[error("JWT error: {0}")]
    JwtError(String), 

    // --- ADD THIS VARIANT ---
    #[error("Invalid or expired token")]
    InvalidToken,
}

pub type Result<T> = std::result::Result<T, LinkVError>;

// --- SERVICE IMPLEMENTATION ---

#[derive(Clone)]
pub struct LinkVConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub ttl_secs: u64,
    pub jwt: Jwt,
}

#[derive(Clone)]
pub struct LinkV {
    config: LinkVConfig,
}

impl LinkV {
    pub fn new(config: LinkVConfig) -> Self {
        Self { config }
    }

    /// Generates a token using JWT. If `expiring` is true, it sets an expiration timestamp.
    /// It also tracks the token in Redis for verification.
    pub async fn generate_token(&self, user_id: &str, expiry_time: u32) -> Result<String> {
        // 1. Calculate the absolute timestamp using the provided expiry_time (in seconds)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LinkVError::JwtError(e.to_string()))?
            .as_secs() as usize;
        
        let expiry_timestamp = current_time + expiry_time as usize;
        
        // 2. Generate the token using your JWT service
        let token = self.config.jwt.generate_exp_token(user_id, expiry_timestamp)
            .map_err(|e| LinkVError::JwtError(format!("{:?}", e)))?;
    
        // 3. Unique Redis key combining user and the specific token
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        let mut con = self.config.redis.get_connection().await?;
        
        // 4. Force the type system to register the output as ()
        // We use the same expiry_time (converted to u64) for the Redis key TTL
        let _res: () = con.set_ex(&redis_key, "1", expiry_time as u64).await?;
        
        Ok(token)
    }

    /// Verifies the token. If valid, deletes it from Redis (one-time use) and returns the token itself.
    /// If invalid, returns false.
    pub async fn verify_token(&self, user_id: &str, token: &str) -> Result {
            // 1. Verify structural/signature integrity via JWT first
            if !self.config.jwt.verify_token(token) {
                return Err(LinkVError::InvalidToken);
            }
    
            // 2. Check Redis blocklist / whitelist status
            let redis_key = format!("linkv:verify:{}:{}", user_id, token);
            let mut con = self.config.redis.get_connection().await?;
            
            // Check if the key exists
            let is_valid: bool = con.exists(&redis_key).await?;
            
            if is_valid {
                // Delete it so it's strictly one-time use
                let _res: () = con.del(&redis_key).await?;
                // Returns the token on success
                Ok(token.to_string()) 
            } else {
                // Key didn't exist or expired in Redis
                Err(LinkVError::InvalidToken)
            }
        }
}
