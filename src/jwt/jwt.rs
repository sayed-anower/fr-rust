use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<usize>,
}

#[derive(Clone)]
pub struct Jwt {
    secret: String,
}

impl Jwt {
    // Create a new instance
    pub fn new(secret: String) -> Self {
        Jwt { secret }
    }

    // 1. Modified: Removed 'secret' from params, uses self.secret instead
    pub fn generate_token(&self, key: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: key.to_owned(),
            exp: None,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    // Generate a token with a custom expiration timestamp
    pub fn generate_exp_token(&self, key: &str, exp: usize) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: key.to_owned(),
            exp: Some(exp),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    // Verify the token and return a boolean (true/false)
    // Modified: Fixed 'secret' variable error, now uses self.secret
    pub fn verify_token(&self, token: &str) -> bool {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; 

        let result = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        );

        result.is_ok()
    }

    // 2. Added: parse_token to convert tokens back to the real Claims content
    pub fn parse_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::HS256);
        // Turn off expiration check here as well, matching your verify_token logic. 
        // If you want expiration to trigger an error, change this to true.
        validation.validate_exp = false; 

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }
}
