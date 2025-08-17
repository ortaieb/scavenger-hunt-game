use axum::{extract::State, http::StatusCode, Json};

use crate::auth::{AuthenticatedUser, ErrorResponse};
use crate::models::{CreateUserRequest, LoginRequest, UserError};
use crate::routes::AppState;
use crate::services::{
    AuthResponse, AuthServiceError, ParticipantAuthResponse, ParticipantTokenRequest,
};

/// Handle user registration
/// POST /authentication/register
pub async fn register_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "User registration attempt for username: {}",
        request.username
    );

    match state.auth_service.register_user(request).await {
        Ok(response) => {
            tracing::info!("User registration successful");
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(AuthServiceError::UserError(UserError::UsernameAlreadyExists)) => {
            tracing::warn!("Registration failed: Username already exists");
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    message: "Username already exists".to_string(),
                }),
            ))
        }
        Err(AuthServiceError::UserError(UserError::InvalidUsername)) => {
            tracing::warn!("Registration failed: Invalid username format");
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid username format".to_string(),
                }),
            ))
        }
        Err(AuthServiceError::UserError(UserError::WeakPassword)) => {
            tracing::warn!("Registration failed: Password too weak");
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Password must be at least 8 characters long".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Registration failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Registration failed".to_string(),
                }),
            ))
        }
    }
}

/// Handle user login
/// POST /authentication/login
pub async fn login_user(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("User login attempt for username: {}", request.username);

    match state.auth_service.login_user(request).await {
        Ok(response) => {
            tracing::info!("User login successful");
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(AuthServiceError::UserError(UserError::UserNotFound)) => {
            tracing::warn!("Login failed: User not found");
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Invalid username or password".to_string(),
                }),
            ))
        }
        Err(AuthServiceError::UserError(UserError::PasswordVerificationFailed)) => {
            tracing::warn!("Login failed: Password verification failed");
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Invalid username or password".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Login failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Login failed".to_string(),
                }),
            ))
        }
    }
}

/// Handle participant token creation
/// POST /challenge/authentication
pub async fn create_participant_token(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(request): Json<ParticipantTokenRequest>,
) -> Result<(StatusCode, Json<ParticipantAuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Participant token request for user: {} and challenge: {}",
        auth_user.username,
        request.challenge_id
    );

    // Get user by username
    let user = match state
        .auth_service
        .get_user_by_username(&auth_user.username)
        .await
    {
        Ok(user) => user,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "User not found".to_string(),
                }),
            ));
        }
    };

    match state
        .auth_service
        .create_participant_token(user.user_id, request.challenge_id)
        .await
    {
        Ok(response) => {
            tracing::info!("Participant token created successfully");
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(AuthServiceError::UserNotInvited) => {
            tracing::warn!("Participant token failed: User not invited to challenge");
            Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    message: "no participant attached to the challenge for this user".to_string(),
                }),
            ))
        }
        Err(AuthServiceError::ChallengeNotFound) => {
            tracing::warn!("Participant token failed: Challenge not found");
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Challenge not found".to_string(),
                }),
            ))
        }
        Err(AuthServiceError::ChallengeNotActive) => {
            tracing::warn!("Participant token failed: Challenge not active");
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Challenge is not active".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Participant token creation failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Token creation failed".to_string(),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;

    #[tokio::test]
    async fn test_register_user_request_deserialization() {
        let json = r#"{
            "username": "test@example.com",
            "password": "Password123",
            "nickname": "TestUser",
            "roles": ["UserVerified", "ChallengeParticipant"]
        }"#;

        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "test@example.com");
        assert_eq!(request.password, "Password123");
        assert_eq!(request.nickname, Some("TestUser".to_string()));
        assert_eq!(
            request.roles,
            Some(vec![UserRole::UserVerified, UserRole::ChallengeParticipant])
        );
    }

    #[tokio::test]
    async fn test_login_request_deserialization() {
        let json = r#"{
            "username": "test@example.com",
            "password": "Password123"
        }"#;

        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "test@example.com");
        assert_eq!(request.password, "Password123");
    }

    #[tokio::test]
    async fn test_participant_token_request_deserialization() {
        let json = r#"{
            "challenge-id": 123
        }"#;

        let request: ParticipantTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.challenge_id, 123);
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
}
