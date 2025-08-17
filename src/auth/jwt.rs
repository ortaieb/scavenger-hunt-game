use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub upn: String,
    pub groups: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantClaims {
    pub iss: String,
    pub upn: String,
    pub groups: Vec<String>,
    pub clg: i32, // Challenge ID - now integer for temporal challenges
    pub usr: i32, // User ID
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("JWT token creation failed: {0}")]
    TokenCreationFailed(#[from] jsonwebtoken::errors::Error),
    #[error("JWT token validation failed: {0}")]
    TokenValidationFailed(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid authorization header format")]
    InvalidAuthHeaderFormat,
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn create_user_token(
        &self,
        username: &str,
        roles: Vec<String>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let claims = Claims {
            iss: "scavenger-hunt-game".to_string(),
            upn: username.to_string(),
            groups: roles,
            exp: (now + Duration::hours(2)).timestamp(), // 2-hour expiration
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(AuthError::from)
    }

    pub fn create_participant_token(
        &self,
        user_id: i32,
        participant_id: Uuid,
        challenge_id: i32, // Now integer for temporal challenges
        roles: Vec<String>,
        challenge_end_time: chrono::DateTime<Utc>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let claims = ParticipantClaims {
            iss: "scavenger-hunt-challenge".to_string(),
            upn: participant_id.to_string(),
            groups: roles,
            clg: challenge_id,
            usr: user_id,
            exp: (challenge_end_time + Duration::hours(1)).timestamp(), // Challenge end + 1 hour
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(AuthError::from)
    }

    pub fn validate_user_token(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::default();

        match decode::<Claims>(token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                // Check if token is expired
                let now = Utc::now().timestamp();
                if token_data.claims.exp < now {
                    return Err(AuthError::TokenExpired);
                }

                // Validate issuer
                if token_data.claims.iss != "scavenger-hunt-game" {
                    return Err(AuthError::TokenValidationFailed(
                        "Invalid issuer".to_string(),
                    ));
                }

                Ok(token_data.claims)
            }
            Err(e) => Err(AuthError::TokenValidationFailed(e.to_string())),
        }
    }

    pub fn validate_participant_token(&self, token: &str) -> Result<ParticipantClaims, AuthError> {
        let validation = Validation::default();

        match decode::<ParticipantClaims>(token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                // Check if token is expired
                let now = Utc::now().timestamp();
                if token_data.claims.exp < now {
                    return Err(AuthError::TokenExpired);
                }

                // Validate issuer
                if token_data.claims.iss != "scavenger-hunt-challenge" {
                    return Err(AuthError::TokenValidationFailed(
                        "Invalid issuer".to_string(),
                    ));
                }

                Ok(token_data.claims)
            }
            Err(e) => Err(AuthError::TokenValidationFailed(e.to_string())),
        }
    }

    pub fn extract_token_from_header(auth_header: &str) -> Result<&str, AuthError> {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            Ok(token)
        } else {
            Err(AuthError::InvalidAuthHeaderFormat)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_validate_user_token() {
        let jwt_service = JwtService::new("test-secret-key-that-is-long-enough-32chars");
        let roles = vec![
            "user.verified".to_string(),
            "challenge.participant".to_string(),
        ];

        let token = jwt_service
            .create_user_token("test@example.com", roles.clone())
            .unwrap();
        let claims = jwt_service.validate_user_token(&token).unwrap();

        assert_eq!(claims.upn, "test@example.com");
        assert_eq!(claims.groups, roles);
        assert_eq!(claims.iss, "scavenger-hunt-game");
    }

    #[test]
    fn test_create_and_validate_participant_token() {
        let jwt_service = JwtService::new("test-secret-key-that-is-long-enough-32chars");
        let roles = vec!["challenge.participant".to_string()];
        let user_id = 1;
        let participant_id = Uuid::new_v4();
        let challenge_id = 123; // Now integer for temporal challenges
        let challenge_end_time = Utc::now() + Duration::hours(2);

        let token = jwt_service
            .create_participant_token(
                user_id,
                participant_id,
                challenge_id,
                roles.clone(),
                challenge_end_time,
            )
            .unwrap();

        let claims = jwt_service.validate_participant_token(&token).unwrap();

        assert_eq!(claims.upn, participant_id.to_string());
        assert_eq!(claims.usr, user_id);
        assert_eq!(claims.clg, challenge_id);
        assert_eq!(claims.groups, roles);
        assert_eq!(claims.iss, "scavenger-hunt-challenge");
    }

    #[test]
    fn test_extract_token_from_header() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtService::extract_token_from_header(header).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_invalid_auth_header_format() {
        let header = "InvalidFormat token";
        let result = JwtService::extract_token_from_header(header);
        assert!(matches!(result, Err(AuthError::InvalidAuthHeaderFormat)));
    }
}
