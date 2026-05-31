use rand::RngCore;

pub struct LinkVConfig {
    pub crypto: CryptoService,
    pub redis: RedisManager,
    pub ttl_secs: u64,
}

pub struct LinkV {
    config: LinkVConfig,
}

impl LinkV {
    /// Initializes the LinkV service with the provided configuration
    pub fn new(config: LinkVConfig) -> Self {
        Self { config }
    }

    /// Generates a secure token, saves it to Redis with a TTL, and returns the token string
    pub async fn generate_token(&self, user_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Generate 256 bytes of cryptographically secure random data
        let mut token_bytes = vec![0u8; 256];
        rand::thread_rng().fill_bytes(&mut token_bytes);
        
        // 2. Hex encode to get a clean URL-safe string representation
        let token = hex::encode(token_bytes);

        // 3. Construct a specific Redis key namespace for this user token
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);

        // 4. Save to Redis with the specified TTL from config
        // Setting value to "1" as a placeholder since we only care about key existence
        self.config.redis.set_ttl(&redis_key, "1", self.config.ttl_secs).await?;

        Ok(token)
    }

    /// Verifies if a token is valid, and immediately destroys it upon one look-up (One-Hit Expiry)
    pub async fn verify_token(&self, user_id: &str, token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let redis_key = format!("linkv:verify:{}:{}", user_id, token);

        // 1. Check if the key exists in Redis and hasn't expired
        let is_valid = self.config.redis.exists(&redis_key).await?;

        if is_valid {
            // 2. Burn after reading: delete it instantly so it can never be used again
            self.config.redis.del(&redis_key).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
