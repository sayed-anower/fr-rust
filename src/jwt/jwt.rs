use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey, dangerous_insecure_decode};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::Duration;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
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

#[derive(Debug, Clone)]
pub enum TokenType {
    Access,
    Refresh,
    Reset,
    Verify,
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

#[derive(Clone)]
pub struct TokenBlacklist {
    store: Arc<RwLock<HashMap<String, usize>>>,
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
        // PERF/SEC FIX: Isolate the read lock to prevent deadlocking against the write lock below
        let is_expired = {
            let read_lock = self.store.read();
            if let Some(&exp) = read_lock.get(jti) {
                if Claims::now() > exp {
                    true // Token is in map, but expired organically
                } else {
                    return true; // Token is in map and still unexpired -> it is revoked
                }
            } else {
                return false; // Not in blacklist
            }
        };

        // If it was in the map but already expired naturally, clean it up
        if is_expired {
            self.store.write().remove(jti);
        }
        
        false
    }
    
    pub fn remove_expired(&self) {
        let now = Claims::now();
        self.store.write().retain(|_, &mut exp| exp > now);
    }
}

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
    
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.issuer = Some(issuer.to_string());
        self.validation.set_issuer(&[issuer]);
        self
    }
    
    pub fn with_audience(mut self, audience: &str) -> Self {
        self.audience = Some(audience.to_string());
        self.validation.set_audience(&[audience]);
        self
    }
    
    pub fn with_blacklist(mut self, blacklist: TokenBlacklist) -> Self {
        self.blacklist = Some(blacklist);
        self
    }
    
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
    
    // COMPILER FIX: Added to support src/linkv/linkv.rs line 47
    pub fn generate_exp_token(&self, sub: &str, expiry_timestamp: usize) -> Result<String, JwtError> {
        let mut claims = Claims::new(sub.to_string());
        claims.exp = Some(expiry_timestamp);
        
        if let Some(ref issuer) = self.issuer {
            claims.iss = Some(issuer.clone());
        }
        if let Some(ref audience) = self.audience {
            claims.aud = Some(audience.clone());
        }
        
        Ok(encode(&Header::new(self.algorithm), &claims, &self.encoding_key)?)
    }

    pub fn generate_pair(&self, user_id: &str) -> Result<(String, String), JwtError> {
        let access_claims = Claims::new(user_id.to_string());
        let refresh_claims = Claims::new(user_id.to_string());
        
        let access_token = self.generate(access_claims, TokenType::Access)?;
        let refresh_token = self.generate(refresh_claims, TokenType::Refresh)?;
        
        Ok((access_token, refresh_token))
    }
    
    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;
        
        let claims = token_data.claims;
        
        if claims.is_expired() {
            return Err(JwtError::TokenExpired);
        }
        
        if let Some(ref blacklist) = self.blacklist {
            if let Some(ref jti) = claims.jti {
                if blacklist.is_revoked(jti) {
                    return Err(JwtError::TokenRevoked);
                }
            }
        }
        
        Ok(claims)
    }

    // COMPILER FIX: Added to support src/linkv/linkv.rs line 57
    pub fn verify_token(&self, token: &str) -> Result<Claims, JwtError> {
        self.verify(token)
    }
    
    pub fn refresh_access(&self, refresh_token: &str) -> Result<String, JwtError> {
        let claims = self.verify(refresh_token)?;
        
        if claims.exp.is_some() {
            // SEC FIX: The original code only allowed refreshing if LESS than 24h remained.
            // This is an anti-pattern. If a refresh token is valid, it should issue a new access token.
            let new_claims = Claims::new(claims.sub);
            self.generate(new_claims, TokenType::Access)
        } else {
            Err(JwtError::InvalidClaims)
        }
    }
    
    pub fn revoke(&self, token: &str) -> Result<(), JwtError> {
        let claims = self.verify(token)?;
        
        // COMPILER FIX: Removed "ref" from blacklist to satisfy the borrow checker
        if let (Some(blacklist), Some(jti), Some(exp)) = (&self.blacklist, claims.jti, claims.exp) {
            blacklist.add(&jti, exp);
            Ok(())
        } else {
            Err(JwtError::InvalidClaims)
        }
    }
    
    pub fn peek_user_id(token: &str) -> Option<String> {
        // SEC WARNING: This skips signature validation. Use ONLY for early routing 
        // (like extracting a DB shard or checking cache), NEVER for authorization.
        dangerous_insecure_decode::<Claims>(token)
            .ok()
            .map(|data| data.claims.sub)
    }
}

#[macro_export]
macro_rules! setup_jwt {
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
