use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use tokio::task;

// --- DATA STRUCTURES ---
// Kept intact so you don't have to change your existing handlers

pub struct EncryptedData {
    pub encrypted_text: String,
}

pub struct DecryptedData {
    pub text: String,
}

pub struct HashedData {
    pub hash: String,
}

// --- OOP SERVICE ---

#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Constructor: Initializes the AES cipher once.
    /// This fixes the performance issue of expanding the key schedule on every request.
    pub fn new(encryption_key: &[u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new_from_slice(encryption_key)
            .map_err(|_| anyhow!("Invalid encryption key length"))?;
        
        Ok(Self { cipher })
    }

    /// Encrypts plaintext into a base64-encoded string (Nonce + Ciphertext)
    pub async fn encrypt_text(&self, text: &str) -> Result<EncryptedData> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = self
            .cipher
            .encrypt(nonce, text.as_bytes())
            .map_err(|_| anyhow!("Encryption failed"))?;

        // Performance fix: Pre-allocate the exact capacity to avoid reallocation
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(EncryptedData {
            encrypted_text: general_purpose::STANDARD.encode(combined),
        })
    }

    /// Decrypts a base64-encoded string back to plaintext
    pub async fn decrypt_text(&self, encrypted_text: &str) -> Result<DecryptedData> {
        let decoded = general_purpose::STANDARD.decode(encrypted_text)?;

        if decoded.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }

        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the data
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Decryption failed"))?;

        Ok(DecryptedData {
            text: String::from_utf8(plaintext)?,
        })
    }

    /// Hashes a string using Argon2 (Runs on blocking thread pool)
    pub async fn hash_data(&self, data: &str) -> Result<HashedData> {
        let data = data.to_string();

        let hash = task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            argon2
                .hash_password(data.as_bytes(), &salt)
                .map(|h| h.to_string())
                .map_err(|e| anyhow!("Hashing failed: {}", e))
        })
        .await??;

        Ok(HashedData { hash })
    }

    /// Verifies a string against an Argon2 hash (Runs on blocking thread pool)
    pub async fn verify_hash(&self, data: &str, hash: &str) -> Result<bool> {
        let data = data.to_string();
        let hash = hash.to_string();

        let is_valid = task::spawn_blocking(move || {
            match PasswordHash::new(&hash) {
                Ok(parsed) => Argon2::default()
                    .verify_password(data.as_bytes(), &parsed)
                    .is_ok(),
                Err(_) => false,
            }
        })
        .await?;

        Ok(is_valid)
    }
}