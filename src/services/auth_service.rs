use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{AuthError, JwtService};
use crate::models::user::{CreateUserRequest, LoginRequest, User, UserError};

#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    #[serde(rename = "user-auth-token")]
    pub user_auth_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParticipantAuthResponse {
    #[serde(rename = "participant-auth-token")]
    pub participant_auth_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParticipantTokenRequest {
    #[serde(rename = "challenge-id")]
    pub challenge_id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthServiceError {
    #[error("User error: {0}")]
    UserError(#[from] UserError),
    #[error("JWT error: {0}")]
    JwtError(#[from] AuthError),
    #[error("Challenge not found")]
    ChallengeNotFound,
    #[error("User not invited to challenge")]
    UserNotInvited,
    #[error("Challenge not active")]
    ChallengeNotActive,
    #[error("Invalid request: {0}")]
    #[allow(dead_code)]
    InvalidRequest(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub struct AuthService {
    jwt_service: Arc<JwtService>,
    pool: PgPool,
}

impl AuthService {
    pub fn new(jwt_service: Arc<JwtService>, pool: PgPool) -> Self {
        Self { jwt_service, pool }
    }

    pub async fn register_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<AuthResponse, AuthServiceError> {
        // Create user with provided or default roles
        let user = User::create(
            &self.pool,
            &request.username,
            &request.password,
            request.nickname.as_deref(),
            request.roles,
        )
        .await?;

        // Get user roles
        let roles = user.get_user_roles(&self.pool).await?;
        let role_strings: Vec<String> = roles.into_iter().map(|r| r.to_string()).collect();

        // Create JWT token
        let token = self
            .jwt_service
            .create_user_token(&user.username, role_strings)?;

        Ok(AuthResponse {
            user_auth_token: token,
            expires_in: 7200, // 2 hours in seconds
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn login_user(
        &self,
        request: LoginRequest,
    ) -> Result<AuthResponse, AuthServiceError> {
        // Authenticate user
        let user = User::authenticate(&self.pool, &request.username, &request.password).await?;

        // Get user roles
        let roles = user.get_user_roles(&self.pool).await?;
        let role_strings: Vec<String> = roles.into_iter().map(|r| r.to_string()).collect();

        // Create JWT token
        let token = self
            .jwt_service
            .create_user_token(&user.username, role_strings)?;

        Ok(AuthResponse {
            user_auth_token: token,
            expires_in: 7200, // 2 hours in seconds
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn create_participant_token(
        &self,
        user_id: i32,
        challenge_id: Uuid,
    ) -> Result<ParticipantAuthResponse, AuthServiceError> {
        // Check if user is invited to the challenge
        let participant = sqlx::query!(
            r#"
            SELECT participant_id, participant_nickname 
            FROM challenge_participants 
            WHERE challenge_id = $1 AND user_id = $2
            "#,
            challenge_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthServiceError::UserNotInvited)?;

        // Get challenge details
        let challenge = sqlx::query!(
            r#"
            SELECT challenge_id, planned_start_time, duration_minutes, active
            FROM challenges 
            WHERE challenge_id = $1
            "#,
            challenge_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthServiceError::ChallengeNotFound)?;

        if !challenge.active.unwrap_or(false) {
            return Err(AuthServiceError::ChallengeNotActive);
        }

        // Calculate challenge end time
        let planned_start = challenge.planned_start_time;
        let duration = challenge.duration_minutes;
        let challenge_end_time = planned_start + chrono::Duration::minutes(duration as i64);

        // Get user roles
        let user = sqlx::query_as!(
            User,
            r#"SELECT user_id, username, password, nickname, 
                     COALESCE(creation_date, NOW()) as "creation_date!",
                     COALESCE(updated_at, NOW()) as "updated_at!"
               FROM users WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        let roles = user.get_user_roles(&self.pool).await?;
        let role_strings: Vec<String> = roles.into_iter().map(|r| r.to_string()).collect();

        // Create participant token
        let token = self.jwt_service.create_participant_token(
            user_id,
            participant.participant_id,
            challenge_id,
            role_strings,
            challenge_end_time,
        )?;

        // Calculate expires_in (challenge end time + 1 hour - now)
        let expires_in = (challenge_end_time + chrono::Duration::hours(1) - Utc::now())
            .num_seconds()
            .max(0);

        Ok(ParticipantAuthResponse {
            participant_auth_token: token,
            expires_in,
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<User, AuthServiceError> {
        let user = sqlx::query_as!(
            User,
            r#"SELECT user_id, username, password, nickname, 
                     COALESCE(creation_date, NOW()) as "creation_date!",
                     COALESCE(updated_at, NOW()) as "updated_at!"
               FROM users WHERE username = $1"#,
            username
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(UserError::UserNotFound)?;

        Ok(user)
    }

    #[allow(dead_code)]
    pub async fn validate_user_permissions(
        &self,
        user_id: i32,
        required_role: &str,
    ) -> Result<bool, AuthServiceError> {
        let role_exists = sqlx::query!(
            "SELECT 1 as exists FROM user_roles WHERE user_id = $1 AND role_name = $2",
            user_id,
            required_role
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(role_exists.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::JwtService;

    #[allow(dead_code)]
    async fn create_test_auth_service() -> AuthService {
        // Note: This would need a test database setup in real tests
        // For now, we'll focus on the structure
        let jwt_service = Arc::new(JwtService::new(
            "test-secret-key-that-is-long-enough-32chars",
        ));
        let pool = PgPool::connect("postgresql://test:test@localhost/test")
            .await
            .expect("Failed to connect to test database");

        AuthService::new(jwt_service, pool)
    }

    #[test]
    fn test_auth_response_serialization() {
        let response = AuthResponse {
            user_auth_token: "test-token".to_string(),
            expires_in: 7200,
            token_type: "Bearer".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("user-auth-token"));
        assert!(json.contains("expires_in"));
        assert!(json.contains("token_type"));
    }

    #[test]
    fn test_participant_token_request_deserialization() {
        let json = r#"{"challenge-id": "550e8400-e29b-41d4-a716-446655440000"}"#;
        let request: ParticipantTokenRequest = serde_json::from_str(json).unwrap();

        assert_eq!(
            request.challenge_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
