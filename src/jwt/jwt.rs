// jwt.rs - Production-ready JWT module
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use dashmap::DashMap;
use tracing::{info, warn};
use uuid::Uuid;

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
}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
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
            exp: now + 900, // 15 minutes default
            iat: now,
            jti: Uuid::new_v4().to_string(),
            iss: None,
            aud: None,
            nbf: None,
            custom: serde_json::Map::new(),
        }
    }

    #[inline]
    fn now() -> usize {
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
    fn duration_seconds(&self) -> u64 {
        match self {
            TokenType::Access => 900,        // 15 minutes
            TokenType::Refresh => 604800,    // 7 days
            TokenType::Reset => 3600,        // 1 hour
            TokenType::Verify => 86400,      // 24 hours
            TokenType::Custom(secs) => *secs,
        }
    }
}

// ============ Blacklist ============
#[derive(Clone)]
pub struct TokenBlacklist {
    store: Arc<DashMap<String, usize>>,
    cleanup_interval: tokio::time::Duration,
}

impl TokenBlacklist {
    pub fn new(cleanup_interval_seconds: u64) -> Self {
        let blacklist = Self {
            store: Arc::new(DashMap::new()),
            cleanup_interval: tokio::time::Duration::from_secs(cleanup_interval_seconds),
        };
        
        // Start background cleanup
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
        if let Some((_, exp)) = self.store.get(jti) {
            if *exp > Claims::now() {
                return true;
            }
            // Expired entry - remove it
            drop(exp);
            self.store.remove(jti);
        }
        false
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }
}

impl Default for TokenBlacklist {
    fn default() -> Self {
        Self::new(300) // Cleanup every 5 minutes
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
        validation.required_spec_claims = vec!["exp".to_string(), "iat".to_string(), "jti".to_string()];
        
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
        let private_key = private_key.as_ref();
        let public_key = public_key.as_ref();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.required_spec_claims = vec!["exp".to_string(), "iat".to_string(), "jti".to_string()];
        
        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_rsa_pem(private_key)
                .map_err(|e| JwtError::InvalidToken(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_rsa_pem(public_key)
                .map_err(|e| JwtError::InvalidToken(e.to_string()))?),
            algorithm: Algorithm::RS256,
            validation: Arc::new(validation),
            blacklist: None,
            issuer: None,
            audience: None,
        })
    }

    pub fn new_ecdsa_p256(private_key: impl AsRef<[u8]>, public_key: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::ES256);
        validation.validate_exp = true;
        validation.required_spec_claims = vec!["exp".to_string(), "iat".to_string(), "jti".to_string()];
        
        Ok(Self {
            encoding_key: Arc::new(EncodingKey::from_ec_pem(private_key.as_ref())
                .map_err(|e| JwtError::InvalidToken(e.to_string()))?),
            decoding_key: Arc::new(DecodingKey::from_ec_pem(public_key.as_ref())
                .map_err(|e| JwtError::InvalidToken(e.to_string()))?),
            algorithm: Algorithm::ES256,
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
        validation.leeway = seconds as i64;
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

    #[inline]
    pub fn generate_with_claims(&self, mut claims: Claims, token_type: TokenType) -> Result<String, JwtError> {
        let duration = token_type.duration_seconds();
        claims.exp = Claims::now() + duration as usize;
        claims.iat = Claims::now();
        claims.jti = Uuid::new_v4().to_string();
        
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

    // ===== Token Verification =====
    
    #[inline]
    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        // Decode and validate signature
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;
        
        let claims = token_data.claims;
        
        // Check blacklist
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

    // ===== Refresh & Revoke =====
    
    #[inline]
    pub fn refresh_access(&self, refresh_token: &str) -> Result<String, JwtError> {
        let claims = self.verify(refresh_token)?;
        
        // Ensure refresh token hasn't expired
        if claims.is_expired() {
            return Err(JwtError::TokenExpired);
        }
        
        // Generate new access token with same subject
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
            // No blacklist configured - can't revoke
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

    // ===== Utilities =====
    
    /// Extract claims without validation (for debugging only)
    #[inline]
    pub fn peek_claims(token: &str) -> Option<Claims> {
        jsonwebtoken::dangerous_unsafe::decode_insecure::<Claims>(token)
            .ok()
            .map(|data| data.claims)
    }

    #[inline]
    pub fn extract_subject(token: &str) -> Option<String> {
        Self::peek_claims(token).map(|c| c.sub)
    }

    #[inline]
    pub fn get_token_expiry(token: &str) -> Option<usize> {
        Self::peek_claims(token).map(|c| c.exp)
    }
}

// ============ Macro for quick setup ============
#[macro_export]
macro_rules! jwt_service {
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

// ============ Tests ============
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_generate_verify() {
        let service = JwtService::new_hs256("test_secret");
        let token = service.generate("user123", TokenType::Access).unwrap();
        
        let claims = service.verify(&token).unwrap();
        assert_eq!(claims.sub, "user123");
    }

    #[test]
    fn test_blacklist_revocation() {
        let blacklist = TokenBlacklist::new(60);
        let service = JwtService::new_hs256("test_secret")
            .with_blacklist(blacklist);
        
        let token = service.generate("user123", TokenType::Access).unwrap();
        let claims = service.verify(&token).unwrap();
        
        service.revoke_token(&token).unwrap();
        let result = service.verify(&token);
        assert!(matches!(result, Err(JwtError::TokenRevoked)));
    }

    #[test]
    fn test_refresh_token() {
        let service = JwtService::new_hs256("test_secret");
        let refresh_token = service.generate("user123", TokenType::Refresh).unwrap();
        
        let new_access = service.refresh_access(&refresh_token).unwrap();
        let claims = service.verify(&new_access).unwrap();
        assert_eq!(claims.sub, "user123");
    }

    #[test]
    #[serial]
    fn test_token_expiry() {
        let service = JwtService::new_hs256("test_secret");
        let token = service.generate_with_claims(
            Claims::new("user123").with_expiration(1),
            TokenType::Custom(1)
        ).unwrap();
        
        // Wait for expiry
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        let result = service.verify(&token);
        assert!(matches!(result, Err(JwtError::TokenExpired)));
    }
}