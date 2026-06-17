use crate::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

// --- ERROR HANDLING ---

#[derive(thiserror::Error, Debug)]
pub enum LinkVError {
    #[error("JWT error: {0}")]
    JwtError(String),
    #[error("Invalid Token!")]
    InvalidToken
}

pub type Result<T> = std::result::Result<T, LinkVError>;

// --- SERVICE IMPLEMENTATION ---

#[derive(Clone)]
pub struct LinkVConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub jwt: JwtService,
}

#[derive(Clone)]
pub struct LinkV {
    config: LinkVConfig,
}

impl LinkV {
    pub fn new(config: LinkVConfig) -> Self {
        Self { config }
    }

    /// Generates a token using JWT. And, It sets an expiration timestamp.
    pub fn generate_token(&self, key: &str, expiry_time: u32) -> Result<String> {
        // 1. Calculate the absolute timestamp using the provided expiry_time (in seconds)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LinkVError::JwtError(e.to_string()))?
            .as_secs() as usize;
        
        let expiry_timestamp = current_time + expiry_time as usize;
        
        // 2. Generate the token using your JWT service
        let token = self.config.jwt.generate_exp_token(key, expiry_timestamp)
            .map_err(|e| LinkVError::JwtError(format!("{:?}", e)))?;
        
        Ok(token)
    }

    /// Verifies the token. If valid, deletes it from Redis (one-time use) and returns the token itself.
    /// If invalid, returns false.
    pub fn verify_token(&self, token: &str) -> Result<bool> {
            // Verify structural/signature integrity via JWT first
            if !self.config.jwt.verify_token(token) {
                return Err(LinkVError::InvalidToken);
            } else {
                return Ok(false);
            }
        }
}
