use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use hex::ToHex;
use rand::RngExt;
use sha2::{Digest, Sha256};
use std::io::{BufReader, Read};
use thiserror::Error;
use tokio::task;

// ========== Error Handling ==========

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
    #[error("Argon2 error: {0}")]
    Argon2(#[from] argon2::password_hash::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Async task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, CryptoError>;

// ========== Core Service ==========

#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
    argon2: Argon2<'static>, // cached Argon2 context
}

impl CryptoService {
    // Creates a new service with the given 32‑byte encryption key.
    pub fn new(key: &[u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        // Default Argon2 parameters (recommended) – cached for reuse.
        let argon2 = Argon2::default();
        Ok(Self { cipher, argon2 })
    }

    // ---------- Synchronous fast operations ----------

    #[inline]
    pub fn sha_hash(&self, data: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        // Encode directly to hex without extra allocations.
        let mut hex = String::with_capacity(64);
        let hex_string: String = hash.encode_hex();
        Ok(hex_string)
    }

    #[inline]
    pub fn verify_sha_hash(&self, data: &str, hash: &str) -> Result<bool> {
        let computed = self.sha_hash(data)?;
        // Constant‑time comparison is not needed for SHA (used for integrity, not secrets)
        Ok(computed == hash)
    }

    #[inline]
    pub fn encrypt_text(&self, text: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher.encrypt(nonce, text.as_bytes())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        Ok(general_purpose::STANDARD.encode(combined))
    }

    #[inline]
    pub fn decrypt_text(&self, encrypted_text: &str) -> Result<String> {
        let decoded = general_purpose::STANDARD.decode(encrypted_text)?;
        if decoded.len() < 12 {
            return Err(CryptoError::InvalidDataLength);
        }
        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(String::from_utf8(plaintext)?)
    }

    // ---------- Asynchronous heavy operations ----------

    pub async fn argon2_hash(&self, data: &str) -> Result<String> {
        let data = data.to_string();
        let argon2 = self.argon2.clone(); // cheap clone

        task::spawn_blocking(move || -> Result<String> {
            let salt = SaltString::generate(&mut OsRng);
            let hashed = argon2.hash_password(data.as_bytes(), &salt)?;
            Ok(hashed.to_string())
        })
        .await?
    }

    pub async fn verify_argon2_hash(&self, data: &str, hash: &str) -> Result<bool> {
        let data = data.to_string();
        let hash = hash.to_string();
        let argon2 = self.argon2.clone();

        task::spawn_blocking(move || -> bool {
            match PasswordHash::new(&hash) {
                Ok(parsed) => argon2.verify_password(data.as_bytes(), &parsed).is_ok(),
                Err(_) => false,
            }
        })
        .await
        .map_err(Into::into)
    }

    // ---------- File operations (async) ----------

    pub async fn sha_file_hash(&self, path: &str) -> Result<String> {
        let path = path.to_string();
        task::spawn_blocking(move || -> Result<String> {
            let file = std::fs::File::open(&path)?;
            let mut reader = BufReader::new(file);
            let mut hasher = Sha256::new();
            let mut buffer = [0; 8192];
            loop {
                let n = reader.read(&mut buffer)?;
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            let hash = hasher.finalize();
            let mut hex = String::with_capacity(64);
            let hex_string: String = hash.encode_hex();
            Ok(hex_string)
        })
        .await?
    }

    pub async fn verify_sha_file_hash(&self, path: &str, hash: &str) -> Result<bool> {
        let computed = self.sha_file_hash(path).await?;
        Ok(computed == hash)
    }

    pub async fn argon2_file_hash(&self, path: &str) -> Result<String> {
        // Reads whole file (bad for huge files, but Argon2 is not meant for large data anyway)
        let content = tokio::fs::read(path).await?;
        let data = String::from_utf8(content)?;
        self.argon2_hash(&data).await
    }

    pub async fn verify_argon2_file_hash(&self, path: &str, hash: &str) -> Result<bool> {
        let content = tokio::fs::read(path).await?;
        let data = String::from_utf8(content)?;
        self.verify_argon2_hash(&data, hash).await
    }

    pub async fn encrypt_file(&self, path: &str) -> Result<String> {
        // Read file, encrypt, write to new file with ".enc" extension.
        let content = tokio::fs::read(path).await?;
        let text = String::from_utf8(content)?;
        let encrypted = self.encrypt_text(&text)?;
        let new_path = format!("{}.enc", path);
        tokio::fs::write(&new_path, encrypted.as_bytes()).await?;
        Ok(new_path)
    }

    pub async fn decrypt_file(&self, path: &str) -> Result<String> {
        let content = tokio::fs::read_to_string(path).await?;
        let decrypted = self.decrypt_text(&content)?;
        let new_path = path.trim_end_matches(".enc").to_string();
        tokio::fs::write(&new_path, decrypted.as_bytes()).await?;
        Ok(new_path)
    }
}