use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<usize>,
}

pub struct Jwt;

impl Jwt {
    // Create a new instance
    pub fn new() -> Self {
        Jwt
    }

    // Generate a standard token (lives forever, stateless)
    pub fn generate_token(&self, user_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: user_id.to_owned(),
            exp: None,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
    }

    // Generate a token with a custom expiration timestamp
    pub fn generate_exp_token(&self, user_id: &str, secret: &str, exp: usize) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: user_id.to_owned(),
            exp: Some(exp),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
    }

    // Verify the token and return a boolean (true/false)
    pub fn verify_token(&self, token: &str, secret: &str) -> bool {
        let mut validation = Validation::new(Algorithm::HS256);
        
        validation.validate_exp = false; 

        let result = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        );

        result.is_ok()
    }
}

