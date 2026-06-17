use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use dashmap::DashMap;
use uuid::Uuid;
use chrono::Duration;

// ============ Error Types ============
#[derive(Debug, Error, Clone)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Token revoked")]
    TokenRevoked,
    #[error("Invalid issuer")]
    InvalidIssuer,
    #[error("Invalid audience")]
    InvalidAudience,
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
    #[error("Key generation error: {0}")]
    KeyError(String),
}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => JwtError::InvalidIssuer,
            jsonwebtoken::errors::ErrorKind::InvalidAudience => JwtError::InvalidAudience,
            _ => JwtError::InvalidToken(err.to_string()),
        }
    }
}

// ============ Claims ============
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<usize>,
    #[serde(flatten)]
    pub custom: serde_json::Map<String, serde_json::Value>,
}

impl Claims {
    #[inline]
    pub fn new(sub: impl Into<String>) -> Self {
        let now = Self::now();
        Self {
            sub: sub.into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::now_v7().to_string(),
            iss: None,
            aud: None,
            nbf: None,
            custom: serde_json::Map::new(),
        }
    }

    #[inline]
    pub fn now() -> usize {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize
    }

    #[inline]
    pub fn with_expiration(mut self, seconds: u64) -> Self {
        self.exp = Self::now() + seconds as usize;
        self
    }

    #[inline]
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    #[inline]
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.aud = Some(audience.into());
        self
    }

    #[inline]
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    #[inline]
    pub fn is_expired(&self) -> bool {
        Self::now() > self.exp
    }

    #[inline]
    pub fn remaining_time(&self) -> Option<Duration> {
        if self.is_expired() {
            return None;
        }
        let remaining = self.exp - Self::now();
        Some(Duration::seconds(remaining as i64))
    }
}

// ============ Token Types ============
#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    Access,
    Refresh,
    Reset,
    Verify,
    Custom(u64),
}

impl TokenType {
    #[inline]
    pub const fn duration_seconds(&self) -> u64 {
        match self {
            TokenType::Access => 900,        // 15 minutes
            TokenType::Refresh => 604800,    // 7 days
            TokenType::Reset => 3600,        // 1 hour
            TokenType::Verify => 86400,      // 24 hours
            TokenType::Custom(secs) => *secs,
        }
    }
}

// ============ Blacklist with Sharded Storage ============
#[derive(Clone)]
pub struct TokenBlacklist {
    store: Arc<DashMap<String, usize>>,
    cleanup_interval: tokio::time::Duration,
}

impl TokenBlacklist {
    pub fn new(cleanup_interval_seconds: u64) -> Self {
        let blacklist = Self {
            store: Arc::new(DashMap::with_capacity(10000)),
            cleanup_interval: tokio::time::Duration::from_secs(cleanup_interval_seconds),
        };

        let store = blacklist.store.clone();
        let interval = blacklist.cleanup_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                let now = Claims::now();
                store.retain(|_, &mut exp| exp > now);
            }
        });

        blacklist
    }

    #[inline]
    pub fn revoke(&self, jti: &str, exp: usize) {
        self.store.insert(jti.to_string(), exp);
    }

    #[inline]
    pub fn is_revoked(&self, jti: &str) -> bool {
        if let Some(entry) = self.store.get_mut(jti) {
            let exp = *entry;
            if exp > Claims::now() {
                return true;
            }
            drop(entry);
            self.store.remove(jti);
        }
        false
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

impl Default for TokenBlacklist {
    fn default() -> Self {
        Self::new(300)
    }
}

// ============ Main JWT Service ============
#[derive(Clone)]
pub struct JwtService {
    encoding_key: Arc<EncodingKey>,
    decoding_key: Arc<DecodingKey>,
    algorithm: Algorithm,
    validation: Arc<Validation>,
    blacklist: Option<TokenBlacklist>,
    issuer: Option<String>,
    audience: Option<String>,
}

impl JwtService {
    // ===== Factory Methods =====

    pub fn new_hs256(secret: impl AsRef<[u8]>) -> Self {
        let secret = secret.as_ref();
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from([
            "exp".to_string(),
            "iat".to_string(),
            "jti".to_string(),
        ]);

        Self {
            encoding_key: Arc::new(EncodingKey::from_secret(secret)),
            decoding_key: Arc::new(DecodingKey::from_secret(secret)),
            algorithm: Algorithm::HS256,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        }
    }

    pub fn new_rs256(private_key: impl AsRef<[u8]>, public_key: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from([
            "exp".to_string(),
            "iat".to_string(),
            "jti".to_string(),
        ]);

        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_rsa_pem(private_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_rsa_pem(public_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            algorithm: Algorithm::RS256,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        })
    }

    pub fn new_rs384(private_key: impl AsRef<[u8]>, public_key: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::RS384);
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from([
            "exp".to_string(),
            "iat".to_string(),
            "jti".to_string(),
        ]);

        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_rsa_pem(private_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_rsa_pem(public_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            algorithm: Algorithm::RS384,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        })
    }

    pub fn new_ecdsa_p256(private_key: impl AsRef<[u8]>, public_key: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::ES256);
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from([
            "exp".to_string(),
            "iat".to_string(),
            "jti".to_string(),
        ]);

        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_ec_pem(private_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_ec_pem(public_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            algorithm: Algorithm::ES256,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        })
    }

    pub fn new_ed25519(private_key: impl AsRef<[u8]>, public_key: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from([
            "exp".to_string(),
            "iat".to_string(),
            "jti".to_string(),
        ]);

        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_ed_pem(private_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_ed_pem(public_key.as_ref())
                .map_err(|e| JwtError::KeyError(e.to_string()))?),
            algorithm: Algorithm::EdDSA,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        })
    }

    // ===== Configuration =====

    #[inline]
    pub fn with_blacklist(mut self, blacklist: TokenBlacklist) -> Self {
        self.blacklist = Some(blacklist);
        self
    }

    #[inline]
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        let issuer = issuer.into();
        self.issuer = Some(issuer.clone());
        let validation = Arc::make_mut(&mut self.validation);
        validation.set_issuer(&[issuer]);
        self
    }

    #[inline]
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        let audience = audience.into();
        self.audience = Some(audience.clone());
        let validation = Arc::make_mut(&mut self.validation);
        validation.set_audience(&[audience]);
        self
    }

    #[inline]
    pub fn with_leeway(mut self, seconds: u64) -> Self {
        let validation = Arc::make_mut(&mut self.validation);
        validation.leeway = seconds;
        self
    }

    #[inline]
    pub fn disable_exp_validation(mut self) -> Self {
        let validation = Arc::make_mut(&mut self.validation);
        validation.validate_exp = false;
        self
    }

    // ===== Token Generation =====

    #[inline]
    pub fn generate(&self, sub: impl Into<String>, token_type: TokenType) -> Result<String, JwtError> {
        let mut claims = Claims::new(sub);
        let duration = token_type.duration_seconds();
        claims.exp = Claims::now() + duration as usize;

        if let Some(ref iss) = self.issuer {
            claims.iss = Some(iss.clone());
        }
        if let Some(ref aud) = self.audience {
            claims.aud = Some(aud.clone());
        }

        let header = Header::new(self.algorithm);
        Ok(encode(&header, &claims, &self.encoding_key)?)
    }

    /// Generate a token with a custom expiration timestamp (in seconds since epoch)
    #[inline]
    pub fn generate_exp_token(&self, sub: impl Into<String>, exp: usize) -> Result<String, JwtError> {
        let mut claims = Claims::new(sub);
        claims.exp = exp;

        if let Some(ref iss) = self.issuer {
            claims.iss = Some(iss.clone());
        }
        if let Some(ref aud) = self.audience {
            claims.aud = Some(aud.clone());
        }

        let header = Header::new(self.algorithm);
        Ok(encode(&header, &claims, &self.encoding_key)?)
    }

    #[inline]
    pub fn generate_with_claims(&self, mut claims: Claims, token_type: TokenType) -> Result<String, JwtError> {
        let duration = token_type.duration_seconds();
        claims.exp = Claims::now() + duration as usize;
        claims.iat = Claims::now();
        claims.jti = Uuid::now_v7().to_string();

        let header = Header::new(self.algorithm);
        Ok(encode(&header, &claims, &self.encoding_key)?)
    }

    #[inline]
    pub fn generate_pair(&self, sub: impl Into<String>) -> Result<(String, String), JwtError> {
        let sub_str = sub.into();
        let access = self.generate(sub_str.clone(), TokenType::Access)?;
        let refresh = self.generate(sub_str, TokenType::Refresh)?;
        Ok((access, refresh))
    }

    #[inline]
    pub fn generate_access_refresh_with_claims(&self, claims: Claims) -> Result<(String, String), JwtError> {
        let access_claims = claims.clone();
        let refresh_claims = claims;

        let access = self.generate_with_claims(access_claims, TokenType::Access)?;
        let refresh = self.generate_with_claims(refresh_claims, TokenType::Refresh)?;

        Ok((access, refresh))
    }

    // ===== Token Verification =====

    #[inline]
    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;

        let claims = token_data.claims;

        if let Some(ref blacklist) = self.blacklist {
            if blacklist.is_revoked(&claims.jti) {
                return Err(JwtError::TokenRevoked);
            }
        }

        Ok(claims)
    }

    #[inline]
    pub fn verify_token(&self, token: &str) -> bool {
        self.verify(token).is_ok()
    }

    #[inline]
    pub fn verify_without_expiry(&self, token: &str) -> Result<Claims, JwtError> {
        let validation = Validation {
            validate_exp: false,
            ..(*self.validation).clone()
        };

        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &validation,
        )?;

        Ok(token_data.claims)
    }

    // ===== Refresh & Revoke =====

    #[inline]
    pub fn refresh_access(&self, refresh_token: &str) -> Result<String, JwtError> {
        let claims = self.verify(refresh_token)?;

        if claims.is_expired() {
            return Err(JwtError::TokenExpired);
        }

        let new_claims = Claims::new(claims.sub);
        self.generate_with_claims(new_claims, TokenType::Access)
    }

    #[inline]
    pub fn revoke_token(&self, token: &str) -> Result<(), JwtError> {
        let claims = self.verify(token)?;

        if let Some(ref blacklist) = self.blacklist {
            blacklist.revoke(&claims.jti, claims.exp);
            Ok(())
        } else {
            Err(JwtError::InvalidToken("Blacklist not configured".to_string()))
        }
    }

    #[inline]
    pub fn revoke_by_jti(&self, jti: &str, exp: usize) -> Result<(), JwtError> {
        if let Some(ref blacklist) = self.blacklist {
            blacklist.revoke(jti, exp);
            Ok(())
        } else {
            Err(JwtError::InvalidToken("Blacklist not configured".to_string()))
        }
    }

    #[inline]
    pub fn is_revoked(&self, jti: &str) -> bool {
        self.blacklist
            .as_ref()
            .map(|b| b.is_revoked(jti))
            .unwrap_or(false)
    }

    // ===== Utilities =====

    /// Extract claims without validation (for debugging only)
    #[inline]
    pub fn peek_claims(&self, token: &str) -> Option<Claims> {
        // Disable all validation checks
        let mut validation = Validation::default();
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        // Issuer and subject validation are skipped by not setting `iss` or `sub`.
        // The `validate_iss` and `validate_sub` fields do not exist in jsonwebtoken 10.x.

        decode::<Claims>(token, &self.decoding_key, &validation)
            .ok()
            .map(|data| data.claims)
    }

    #[inline]
    pub fn extract_subject(&self, token: &str) -> Option<String> {
        self.peek_claims(token).map(|c| c.sub)
    }

    #[inline]
    pub fn get_token_expiry(&self, token: &str) -> Option<usize> {
        self.peek_claims(token).map(|c| c.exp)
    }

    #[inline]
    pub fn get_token_jti(&self, token: &str) -> Option<String> {
        self.peek_claims(token).map(|c| c.jti)
    }

    #[inline]
    pub fn get_token_issuer(&self, token: &str) -> Option<String> {
        self.peek_claims(token).and_then(|c| c.iss)
    }

    #[inline]
    pub fn get_token_audience(&self, token: &str) -> Option<String> {
        self.peek_claims(token).and_then(|c| c.aud)
    }
}

// ============ Macro for quick setup ============
#[macro_export]
macro_rules! setup_jwt {
    ($secret:expr) => {
        JwtService::new_hs256($secret)
    };
    ($secret:expr, $issuer:expr) => {
        JwtService::new_hs256($secret).with_issuer($issuer)
    };
    ($secret:expr, $issuer:expr, $audience:expr) => {
        JwtService::new_hs256($secret)
            .with_issuer($issuer)
            .with_audience($audience)
    };
}