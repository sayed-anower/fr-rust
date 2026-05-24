use hmac::{Hmac, Mac};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use sha2::Sha256;
use redis::{Commands, Client};

type HmacSha256 = Hmac<Sha256>;

pub struct OtpService {
    secret: String,
    // Redis client handles connection pooling and is thread-safe (Sync + Send)
    redis_client: Client,
    ttl_secs: u64,
}

impl OtpService {
    pub fn new(secret: String, redis_url: &str, ttl_secs: u64) -> Self {
        let redis_client = Client::open(redis_url).expect("Invalid Redis URL");
        Self {
            secret,
            redis_client,
            ttl_secs,
        }
    }

    // Generate OTP
    pub fn generate_otp(&self, user_id: &str, digits: u32) -> String {
        let otp = Self::random_digits(digits);
        let hash = Self::hmac_hash(&self.secret, &otp);

        // Get a connection from the client
        let mut con = self.redis_client.get_connection().expect("Failed to connect to Redis");

        // Use Redis native EXPIRE (TTL) so expired tokens are automatically purged by Redis
        let redis_key = format!("otp:{}", user_id);
        let _: () = con.set_ex(redis_key, hash, self.ttl_secs).expect("Failed to save OTP to Redis");

        otp
    }

    // Verify OTP
    pub fn verify_otp(&self, user_id: &str, otp: &str) -> bool {
        let mut con = self.redis_client.get_connection().expect("Failed to connect to Redis");
        let redis_key = format!("otp:{}", user_id);

        // Fetch the stored hash directly from Redis
        let stored_hash: Option<String> = con.get(&redis_key).expect("Failed to fetch OTP from Redis");

        let hash_to_check = match stored_hash {
            Some(h) => h,
            None => return false, // Expired or never existed
        };

        let calculated_hash = Self::hmac_hash(&self.secret, otp);

        // Constant-time verification or direct string comparison
        let ok = calculated_hash == hash_to_check;

        if ok {
            // OTP can only be used once; delete it immediately upon successful verification
            let _: () = con.del(redis_key).expect("Failed to delete used OTP");
        }

        ok
    }

    // Helpers
    fn random_digits(digits: u32) -> String {
        // FIX: Correctly fetch random bytes into an actual mutable variable
        let mut bytes = [0u8; 8];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let num = u64::from_le_bytes(bytes);

        let otp = num % 10u64.pow(digits);
        format!("{:0width$}", otp, width = digits as usize)
    }

    fn hmac_hash(secret: &str, data: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");

        mac.update(data.as_bytes());

        let result = mac.finalize().into_bytes();
        general_purpose::STANDARD.encode(result)
    }
}