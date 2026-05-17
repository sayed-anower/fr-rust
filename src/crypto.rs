use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use tokio::task;

// Config struct to replace Env Vars
pub struct CryptoConfig<'a> {
    pub encryption_key: &'a [u8; 32], // 32 bytes for AES-256
}

// ENCRYPTION
pub struct EncryptedData {
    pub encrypted_text: String,
}

pub async fn encrypt_text(config: &CryptoConfig<'_>, text: &str) -> Result<EncryptedData> {
    let cipher = Aes256Gcm::new_from_slice(config.encryption_key)?;

    // 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher
        .encrypt(nonce, text.as_bytes())
        .map_err(|_| anyhow!("Encryption failed"))?;

    // Combine nonce + ciphertext
    let mut combined = Vec::with_capacity(nonce_bytes.len() + encrypted.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&encrypted);

    Ok(EncryptedData {
        encrypted_text: general_purpose::STANDARD.encode(combined),
    })
}

// DECRYPTION
pub struct DecryptedData {
    pub text: String,
}

pub async fn decrypt_text(
    config: &CryptoConfig<'_>,
    encrypted_text: &str,
) -> Result<DecryptedData> {
    let cipher = Aes256Gcm::new_from_slice(config.encryption_key)?;

    let decoded = general_purpose::STANDARD.decode(encrypted_text)?;

    if decoded.len() < 12 {
        return Err(anyhow!("Invalid encrypted data"));
    }

    let (nonce_bytes, ciphertext) = decoded.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed"))?;

    Ok(DecryptedData {
        text: String::from_utf8(decrypted)?,
    })
}

// HASHING
pub struct HashedData {
    pub hash: String,
}

pub async fn hash_data(data: &str) -> Result<HashedData> {
    let data = data.to_string(); // Move to heap for the blocking task

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

pub async fn verify_hash(data: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)?;
    let result = Argon2::default().verify_password(data.as_bytes(), &parsed_hash);
    Ok(result.is_ok())
}
