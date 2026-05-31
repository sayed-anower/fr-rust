use crate::prelude::*;
use rand::RngCore;

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
        self.config.redis.set_ex(&redis_key, "1", self.config.ttl_secs).await?;
        Ok(token)
    }
    pub async fn verify_token(&self, user_id: &str, token: &str) -> anyhow::Result<bool> {
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);
        let is_valid = self.config.redis.exists(&redis_key).await?;
        if is_valid {
            self.config.redis.del(&redis_key).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
