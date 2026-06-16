use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

// Custom error types
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Missing secret key")]
    MissingSecret,
    
    #[error("Token revoked")]
    TokenRevoked,
    
    #[error("Invalid claims")]
    InvalidClaims,
}

impl From<JwtError> for &'static str {
    fn from(err: JwtError) -> Self {
        match err {
            JwtError::TokenExpired => "Token has expired",
            JwtError::InvalidSignature => "Invalid token signature",
            JwtError::MissingSecret => "Secret key missing",
            JwtError::TokenRevoked => "Token has been revoked",
            JwtError::InvalidClaims => "Invalid token claims",
            JwtError::InvalidToken(_) => "Invalid token",
        }
    }
}

// Enhanced claims with standard JWT fields
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    // Subject (user id, email, etc.)
    pub sub: String,
    
    // Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    
    // Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    
    // Expiration time (as UTC timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<usize>,
    
    // Not before (as UTC timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<usize>,
    
    // Issued at (as UTC timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<usize>,
    
    // JWT ID (unique identifier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    
    // Custom claims
    #[serde(flatten)]
    pub custom: serde_json::Map<String, serde_json::Value>,
}

impl Claims {
    pub fn new(sub: String) -> Self {
        Self {
            sub,
            iss: None,
            aud: None,
            exp: None,
            nbf: None,
            iat: Some(Self::now()),
            jti: Some(uuid::Uuid::new_v4().to_string()),
            custom: serde_json::Map::new(),
        }
    }
    
    fn now() -> usize {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
    }
    
    pub fn with_expiration(mut self, seconds: u64) -> Self {
        self.exp = Some(Self::now() + seconds as usize);
        self
    }
    
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.iss = Some(issuer.to_string());
        self
    }
    
    pub fn with_audience(mut self, audience: &str) -> Self {
        self.aud = Some(audience.to_string());
        self
    }
    
    pub fn with_custom_claim(mut self, key: &str, value: serde_json::Value) -> Self {
        self.custom.insert(key.to_string(), value);
        self
    }
    
    pub fn is_expired(&self) -> bool {
        match self.exp {
            Some(exp) => Self::now() > exp,
            None => false,
        }
    }
}

// Token types for different use cases
#[derive(Debug, Clone)]
pub enum TokenType {
    Access,   // Short-lived (15 min)
    Refresh,  // Long-lived (7 days)
    Reset,    // Password reset (1 hour)
    Verify,   // Email verification (24 hours)
    Custom(Duration),
}

impl TokenType {
    fn get_duration(&self) -> Duration {
        match self {
            TokenType::Access => Duration::minutes(15),
            TokenType::Refresh => Duration::days(7),
            TokenType::Reset => Duration::hours(1),
            TokenType::Verify => Duration::hours(24),
            TokenType::Custom(duration) => *duration,
        }
    }
}

// Blacklist for revoked tokens (using memory store, can be replaced with Redis)
#[derive(Clone)]
pub struct TokenBlacklist {
    store: Arc<RwLock<HashMap<String, usize>>>, // token_id -> expiration
}

impl TokenBlacklist {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn add(&self, jti: &str, expiration: usize) {
        self.store.write().insert(jti.to_string(), expiration);
    }
    
    pub fn is_revoked(&self, jti: &str) -> bool {
        if let Some(&exp) = self.store.read().get(jti) {
            if Claims::now() > exp {
                // Clean up expired entries lazily
                self.store.write().remove(jti);
                return false;
            }
            return true;
        }
        false
    }
    
    pub fn remove_expired(&self) {
        let now = Claims::now();
        self.store.write().retain(|_, &mut exp| exp > now);
    }
}

// Main JWT framework
#[derive(Clone)]
pub struct Jwt {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    issuer: Option<String>,
    audience: Option<String>,
    blacklist: Option<TokenBlacklist>,
    validation: Validation,
}

impl Jwt {
    // Create with HS256 symmetric key
    pub fn new_hs256(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            algorithm: Algorithm::HS256,
            issuer: None,
            audience: None,
            blacklist: None,
            validation,
        }
    }
    
    // Create with RSA (more secure for distributed systems)
    pub fn new_rs256(private_key: &str, public_key: &str) -> Result<Self, JwtError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        
        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key.as_bytes())?,
            decoding_key: DecodingKey::from_rsa_pem(public_key.as_bytes())?,
            algorithm: Algorithm::RS256,
            issuer: None,
            audience: None,
            blacklist: None,
            validation,
        })
    }
    
    // Configure issuer validation
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.issuer = Some(issuer.to_string());
        self.validation.set_issuer(&[issuer]);
        self
    }
    
    // Configure audience validation
    pub fn with_audience(mut self, audience: &str) -> Self {
        self.audience = Some(audience.to_string());
        self.validation.set_audience(&[audience]);
        self
    }
    
    // Enable token blacklist
    pub fn with_blacklist(mut self, blacklist: TokenBlacklist) -> Self {
        self.blacklist = Some(blacklist);
        self
    }
    
    // Generate token with custom claims
    pub fn generate(&self, mut claims: Claims, token_type: TokenType) -> Result<String, JwtError> {
        let duration = token_type.get_duration();
        let now = Claims::now();
        
        claims.iat = Some(now);
        claims.exp = Some(now + duration.num_seconds() as usize);
        
        if let Some(ref issuer) = self.issuer {
            claims.iss = Some(issuer.clone());
        }
        
        if let Some(ref audience) = self.audience {
            claims.aud = Some(audience.clone());
        }
        
        Ok(encode(&Header::new(self.algorithm), &claims, &self.encoding_key)?)
    }
    
    // Generate access and refresh token pair
    pub fn generate_pair(&self, user_id: &str) -> Result<(String, String), JwtError> {
        let access_claims = Claims::new(user_id.to_string());
        let refresh_claims = Claims::new(user_id.to_string());
        
        let access_token = self.generate(access_claims, TokenType::Access)?;
        let refresh_token = self.generate(refresh_claims, TokenType::Refresh)?;
        
        Ok((access_token, refresh_token))
    }
    
    // Verify and decode token
    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;
        
        let claims = token_data.claims;
        
        // Check expiration
        if claims.is_expired() {
            return Err(JwtError::TokenExpired);
        }
        
        // Check if revoked
        if let Some(ref blacklist) = self.blacklist {
            if let Some(ref jti) = claims.jti {
                if blacklist.is_revoked(jti) {
                    return Err(JwtError::TokenRevoked);
                }
            }
        }
        
        Ok(claims)
    }
    
    // Refresh token (issue new access token from valid refresh token)
    pub fn refresh_access(&self, refresh_token: &str) -> Result<String, JwtError> {
        let claims = self.verify(refresh_token)?;
        
        // Check if it's a refresh token (should have longer expiry)
        if let Some(exp) = claims.exp {
            let remaining = exp - Claims::now();
            if remaining < 86400 { // Less than 24 hours remaining
                // Issue new access token
                let new_claims = Claims::new(claims.sub);
                self.generate(new_claims, TokenType::Access)
            } else {
                Err(JwtError::InvalidClaims)
            }
        } else {
            Err(JwtError::InvalidClaims)
        }
    }
    
    // Revoke token
    pub fn revoke(&self, token: &str) -> Result<(), JwtError> {
        let claims = self.verify(token)?;
        
        if let (Some(ref blacklist), Some(jti), Some(exp)) = (&self.blacklist, claims.jti, claims.exp) {
            blacklist.add(&jti, exp);
            Ok(())
        } else {
            Err(JwtError::InvalidClaims)
        }
    }
    
    // Extract user ID from token without full validation (for early routing)
    pub fn peek_user_id(token: &str) -> Option<String> {
        use jsonwebtoken::dangerous_unsafe_decode;
        dangerous_unsafe_decode::<Claims>(token)
            .ok()
            .map(|data| data.claims.sub)
    }
}

// Convenience macro for quick JWT setup
#[macro_export]
macro_rules! set_jwt {
    ($secret:expr) => {
        Jwt::new_hs256($secret)
    };
    ($secret:expr, $issuer:expr) => {
        Jwt::new_hs256($secret).with_issuer($issuer)
    };
    ($secret:expr, $issuer:expr, $audience:expr) => {
        Jwt::new_hs256($secret)
            .with_issuer($issuer)
            .with_audience($audience)
    };
}

// Example usage with Actix-web
#[cfg(feature = "actix")]
mod actix_integration {
    use super::*;
    use actix_web::{dev::ServiceRequest, Error, FromRequest, HttpMessage};
    use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
    use std::future::{ready, Ready};
    
    impl FromRequest for Claims {
        type Error = Error;
        type Future = Ready<Result<Self, Self::Error>>;
        
        fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
            let set_jwt = req.app_data::<actix_web::web::Data<Jwt>>().unwrap();
            
            let auth_header = req.headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Bearer "));
            
            match auth_header {
                Some(token) => match set_jwt.verify(token) {
                    Ok(claims) => ready(Ok(claims)),
                    Err(e) => ready(Err(actix_web::error::ErrorUnauthorized(e.to_string()))),
                },
                None => ready(Err(actix_web::error::ErrorUnauthorized("Missing token"))),
            }
        }
    }
}
