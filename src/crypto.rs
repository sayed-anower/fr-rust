use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        rand_core::OsRng,
    },
};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use tokio::task;

// CONFIG
pub struct CryptoConfig<'a> {
    pub encryption_key: &'a [u8; 32],
}

// ENCRYPTION
pub struct EncryptedData {
    pub encrypted_text: String,
}

pub async fn encrypt_text(config: &CryptoConfig<'_>, text: &str) -> Result<EncryptedData> {
    let cipher = Aes256Gcm::new_from_slice(config.encryption_key)?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, text.as_bytes())
        .map_err(|_| anyhow!("Encryption failed"))?;

    let mut combined = Vec::new();
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(EncryptedData {
        encrypted_text: general_purpose::STANDARD.encode(combined),
    })
}

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

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed"))?;

    Ok(DecryptedData {
        text: String::from_utf8(plaintext)?,
    })
}

// HASHING
pub struct HashedData {
    pub hash: String,
}

pub async fn hash_data(data: &str) -> Result<HashedData> {
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

pub async fn verify_hash(data: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash)?;

    let ok = task::spawn_blocking(move || {
        Argon2::default()
            .verify_password(data.as_bytes(), &parsed)
            .is_ok()
    })
    .await?;

    Ok(ok)
}
