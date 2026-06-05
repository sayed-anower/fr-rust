use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::task;

// --- ERROR HANDLING ---

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid encryption key length")]
    InvalidKeyLength,

    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid encrypted data: too short")]
    InvalidDataLength,

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Invalid UTF-8 sequence: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Argon2 hashing error: {0}")]
    Argon2(#[from] argon2::password_hash::Error),

    #[error("Async task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, CryptoError>;

// --- OOP SERVICE ---

#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Constructor: Initializes the AES cipher once.
    pub fn new(encryption_key: &[u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new_from_slice(encryption_key)
            .map_err(|_| CryptoError::InvalidKeyLength)?;

        Ok(Self { cipher })
    }

    /// Encrypts plaintext into a base64-encoded string (Nonce + Ciphertext)
    /// Purely CPU-bound and fast: Kept synchronous to avoid async executor overhead.
    pub fn encrypt_text(&self, text: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = self
            .cipher
            .encrypt(nonce, text.as_bytes())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Pre-allocate exact capacity to prevent reallocation vectors
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        let encrypt_text = general_purpose::STANDARD.encode(combined);
        Ok(encrypt_text)
    }

    /// Decrypts a base64-encoded string back to plaintext
    /// Purely CPU-bound and fast: Kept synchronous.
    pub fn decrypt_text(&self, encrypted_text: &str) -> Result<String> {
        let decoded = general_purpose::STANDARD.decode(encrypted_text)?;

        if decoded.len() < 12 {
            return Err(CryptoError::InvalidDataLength);
        }

        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the data
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;
    let decrypt_text = String::from_utf8(plaintext)?;
        Ok(decrypt_text)
    }

    /// Hashes a string using SHA-256 and returns a hex-encoded string.
    /// Fast, non-blocking: Kept synchronous.
    pub fn sha256_hash(&self, data: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();

        // Format raw bytes directly into a 64-character lowercase hex string
        let hash = format!("{:x}", result);

        Ok(hash)
    }

    /// Hashes a string using Argon2.
    /// Heavy CPU/Memory usage: Must remain async and run on a blocking thread pool.
    pub async fn hash_data(&self, data: &str) -> Result<String> {
        let data = data.to_string();

        let hash = task::spawn_blocking(move || -> Result<String> {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            let hashed = argon2
                .hash_password(data.as_bytes(), &salt)?; // `?` cleanly converts to CryptoError::Argon2
            
            Ok(hashed.to_string())
        })
        .await??;

        Ok(hash)
    }

    /// Verifies a string against an Argon2 hash.
    /// Heavy CPU usage: Must remain async and run on a blocking thread pool.
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