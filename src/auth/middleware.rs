use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::jwt::{AuthError, JwtService};

#[derive(Clone)]
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedParticipant {
    pub participant_id: String,
    pub user_id: i32,
    pub challenge_id: String,
    pub roles: Vec<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

// Extractor for authenticated users
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Missing authorization header".to_string(),
                }),
            ))?
            .to_str()
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        message: "Invalid authorization header format".to_string(),
                    }),
                )
            })?;

        // Extract token from header
        let token = JwtService::extract_token_from_header(auth_header).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Invalid authorization header format".to_string(),
                }),
            )
        })?;

        // Get JWT service from extensions (set by middleware)
        let jwt_service = parts.extensions.get::<Arc<JwtService>>().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "JWT service not available".to_string(),
            }),
        ))?;

        // Validate token
        let claims = jwt_service.validate_user_token(token).map_err(|e| {
            let status = match e {
                AuthError::TokenExpired => StatusCode::UNAUTHORIZED,
                _ => StatusCode::UNAUTHORIZED,
            };
            (
                status,
                Json(ErrorResponse {
                    message: format!("Token validation failed: {e}"),
                }),
            )
        })?;

        Ok(AuthenticatedUser {
            username: claims.upn,
            roles: claims.groups,
        })
    }
}

// Extractor for authenticated participants
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedParticipant
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Missing authorization header".to_string(),
                }),
            ))?
            .to_str()
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        message: "Invalid authorization header format".to_string(),
                    }),
                )
            })?;

        // Extract token from header
        let token = JwtService::extract_token_from_header(auth_header).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Invalid authorization header format".to_string(),
                }),
            )
        })?;

        // Get JWT service from extensions (set by middleware)
        let jwt_service = parts.extensions.get::<Arc<JwtService>>().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "JWT service not available".to_string(),
            }),
        ))?;

        // Validate participant token
        let claims = jwt_service.validate_participant_token(token).map_err(|e| {
            let status = match e {
                AuthError::TokenExpired => StatusCode::UNAUTHORIZED,
                _ => StatusCode::UNAUTHORIZED,
            };
            (
                status,
                Json(ErrorResponse {
                    message: format!("Token validation failed: {e}"),
                }),
            )
        })?;

        Ok(AuthenticatedParticipant {
            participant_id: claims.upn,
            user_id: claims.usr,
            challenge_id: claims.clg.to_string(),
            roles: claims.groups,
        })
    }
}

// Middleware to inject JWT service into request extensions
pub async fn jwt_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Add JWT service to request extensions
    request
        .extensions_mut()
        .insert(auth_state.jwt_service.clone());

    next.run(request).await
}

// Role validation helper functions
impl AuthenticatedUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    pub fn require_role(&self, role: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    message: format!("Required role '{}' not found", role),
                }),
            ))
        }
    }
}

impl AuthenticatedParticipant {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    pub fn require_role(&self, role: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    message: format!("Required role '{}' not found", role),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_user_role_validation() {
        let user = AuthenticatedUser {
            username: "test@example.com".to_string(),
            roles: vec![
                "user.verified".to_string(),
                "challenge.participant".to_string(),
            ],
        };

        assert!(user.has_role("user.verified"));
        assert!(user.has_role("challenge.participant"));
        assert!(!user.has_role("game.admin"));

        assert!(user.has_any_role(&["user.verified", "game.admin"]));
        assert!(!user.has_any_role(&["game.admin", "challenge.moderator"]));

        assert!(user.require_role("user.verified").is_ok());
        assert!(user.require_role("game.admin").is_err());
    }
}
