use crate::prelude::*;
use rand::{rngs::OsRng, RngCore};

pub struct OtpConfig {
    pub secret: String,
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub ttl_secs: u64,
}

pub struct OtpService {
    config: OtpConfig,
}

impl OtpService {
    pub fn new(config: OtpConfig) -> Self {
        Self { config }
    }

    pub async fn generate_otp(&self, user_id: &str, digits: u32) -> anyhow::Result<String> {
        let otp = Self::random_digits(digits);
        let content_to_hash = format!("{}:{}", self.config.secret, otp);
        let hash = self.config.crypto.sha256_hash(&content_to_hash)?.hash;
        let redis_key = format!("otp:{}", user_id);

        // 3. Get a mutable handle to the connection
        let mut con = self.config.redis.connection.clone();
        
        // 4. Call the command on your mutable connection handle
        con.set_ex(&redis_key, &hash, self.config.ttl_secs).await?;
        
        Ok(otp)
    }

    pub async fn verify_otp(&self, user_id: &str, otp: &str) -> anyhow::Result<bool> {
        let redis_key = format!("otp:{}", user_id);
        
        // 3. Get a mutable handle here too
        let mut con = self.config.redis.connection.clone();

        // 4. Call the command on your mutable connection handle
        let stored_hash: Option<String> = con.get(&redis_key).await?;
        
        let hash_to_check = match stored_hash {
            Some(h) => h,
            None => return Ok(false),
        };

        let content_to_hash = format!("{}:{}", self.config.secret, otp);
        let calculated_hash = self.config.crypto.sha256_hash(&content_to_hash)?.hash;
        let ok = calculated_hash == hash_to_check;
        
        if ok {
            con.del(&redis_key).await?;
        }

        Ok(ok)
    }

    fn random_digits(digits: u32) -> String {
        let mut bytes = [0u8; 8];
        OsRng.fill_bytes(&mut bytes);
        let num = u64::from_le_bytes(bytes);
        let otp = num % 10u64.pow(digits);
        format!("{:0width$}", otp, width = digits as usize)
    }
}
