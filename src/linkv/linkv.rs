use crate::prelude::*;
use rand::RngCore;
use redis::AsyncCommands; // Ensure this trait is in scope for .set_ex(), .exists(), and .del()

pub struct LinkVConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager, // This now holds our connection from the previous fix
    pub ttl_secs: u64,
}

pub struct LinkV {
    config: LinkVConfig,
}

impl LinkV {
    pub fn new(config: LinkVConfig) -> Self {
        Self { config }
    }

    pub async fn generate_token(&self, user_id: &str) -> anyhow::Result<String> {
        let mut token_bytes = vec![0u8; 256];
        rand::thread_rng().fill_bytes(&mut token_bytes);
        let token = hex::encode(token_bytes);
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        
        // 1. Clone the cheap inner multiplexed connection handle
        let mut con = self.config.redis.connection.clone();
        
        // 2. Execute using the mutable connection handle
        con.set_ex(&redis_key, "1", self.config.ttl_secs).await?;
        
        Ok(token)
    }

    pub async fn verify_token(&self, user_id: &str, token: &str) -> anyhow::Result<bool> {
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        
        // 1. Grab your mutable connection handle here too
        let mut con = self.config.redis.connection.clone();
        
        // 2. Pass &mut con implicitly by calling the method on it
        let is_valid: bool = con.exists(&redis_key).await?;
        
        if is_valid {
            con.del(&redis_key).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}