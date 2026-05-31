use crate::prelude::*;
use rand::RngCore;
use ::redis::AsyncCommands; // Fixed ambiguity with leading `::`

pub struct LinkVConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
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
        
        let mut con = self.config.redis.connection.clone();
        
        // Added turbofish <_, _, ()> to specify the expected return type is ()
        con.set_ex::<_, _, ()>(&redis_key, "1", self.config.ttl_secs).await?;
        
        Ok(token)
    }

    pub async fn verify_token(&self, user_id: &str, token: &str) -> anyhow::Result<bool> {
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        
        let mut con = self.config.redis.connection.clone();
        let is_valid: bool = con.exists(&redis_key).await?;
        
        if is_valid {
            // Added turbofish <_, ()> to specify the expected return type is ()
            con.del::<_, ()>(&redis_key).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
