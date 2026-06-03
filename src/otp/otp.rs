use crate::prelude::*;
use rand::Rng;
use deadpool_redis::redis::AsyncCommands;
use thiserror::Error;
use crate::crypto::crypto::CryptoError;

// --- ERROR HANDLING ---

#[derive(Error, Debug)]
pub enum OtpError {
    #[error("Redis command error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),

    #[error("Cryptography service error: {0}")]
    Crypto(#[from] CryptoError), // Assuming CryptoError is the name of your error enum from earlier
}

pub type Result<T> = std::result::Result<T, OtpError>;

// --- SERVICE IMPLEMENTATION ---

#[derive(Clone)]
pub struct OtpConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub ttl_secs: u64,
}

#[derive(Clone)]
pub struct OtpService {
    config: OtpConfig,
}

impl OtpService {
    pub fn new(config: OtpConfig) -> Self {
        Self { config }
    }

    pub async fn generate_otp(&self, user_id: &str, digits: u32) -> Result<String> {
        let otp = Self::random_digits(digits);
        let content_to_hash = format!("{}:{}", self.config.secret, otp);
        let hash = self.config.crypto.sha256_hash(&content_to_hash)?.hash;
        let redis_key = format!("otp:{}", user_id);
        
        let mut con = self.config.redis.get_connection().await?;
        
        let _res: () = con.set_ex(&redis_key, &hash, self.config.ttl_secs).await?;
        
        Ok(otp)
    }

    pub async fn verify_otp(&self, user_id: &str, otp: &str) -> Result<bool> {
        let redis_key = format!("otp:{}", user_id);
        
        let mut con = self.config.redis.get_connection().await?;
        let stored_hash: Option<String> = con.get(&redis_key).await?;
        
        let hash_to_check = match stored_hash {
            Some(h) => h,
            None => return Ok(false),
        };
        
        let content_to_hash = format!("{}:{}", self.config.secret, otp);
        let calculated_hash = self.config.crypto.sha256_hash(&content_to_hash)?.hash;
        let ok = calculated_hash == hash_to_check;
        
        if ok {
            let _res: () = con.del(&redis_key).await?;
        }
        
        Ok(ok)
    }

    fn random_digits(digits: u32) -> String {
        let mut bytes = [0u8; 8];
        
        // rand::rng() gets the new, optimized cryptographically secure thread-local generator
        rand::rng().fill_bytes(&mut bytes);
        
        let num = u64::from_le_bytes(bytes);
        let otp = num % 10u64.pow(digits);
        format!("{:0width$}", otp, width = digits as usize)
    }
}
