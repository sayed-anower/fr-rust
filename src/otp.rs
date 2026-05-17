use rand::{RngCore, rngs::OsRng};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct OtpRecord {
    hash: String,
    expires_at: u64,
}

pub struct OtpService {
    secret: String,
    store: std::sync::Mutex<HashMap<String, OtpRecord>>,
    ttl_secs: u64,
}

impl OtpService {
    pub fn new(secret: String, ttl_secs: u64) -> Self {
        Self {
            secret,
            store: std::sync::Mutex::new(HashMap::new()),
            ttl_secs,
        }
    }

    // Generate OTP
    pub fn generate_otp(&self, user_id: &str, digits: u32) -> String {
        let otp = Self::random_digits(digits);

        let expires_at = Self::now() + self.ttl_secs;

        let hash = Self::hmac_hash(&self.secret, &otp);

        let mut store = self.store.lock().unwrap();
        store.insert(
            user_id.to_string(),
            OtpRecord { hash, expires_at },
        );

        otp
    }

    // Verify OTP
    pub fn verify_otp(&self, user_id: &str, otp: &str) -> bool {
        let mut store = self.store.lock().unwrap();

        let record = match store.get(user_id) {
            Some(r) => r.clone(),
            None => return false,
        };

        if Self::now() > record.expires_at {
            store.remove(user_id);
            return false;
        }

        let hash = Self::hmac_hash(&self.secret, otp);

        let ok = hash == record.hash;

        if ok {
            store.remove(user_id); // OTP can only be used once
        }

        ok
    }

    // Helpers
    fn random_digits(digits: u32) -> String {
        let mut num = 0u64;
        OsRng.fill_bytes(&mut num.to_le_bytes());

        let otp = num % 10u64.pow(digits);

        format!("{:0width$}", otp, width = digits as usize)
    }

    fn hmac_hash(secret: &str, data: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");

        mac.update(data.as_bytes());

        let result = mac.finalize().into_bytes();
        base64::encode(result)
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}