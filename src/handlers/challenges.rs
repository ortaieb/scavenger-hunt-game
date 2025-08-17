use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::auth::{AuthenticatedUser, ErrorResponse};
use crate::models::challenge::TemporalChallenge;
use crate::models::{
    AuditLog, Challenge, ChallengeError, ChallengeResponse, CreateChallengeRequest,
    StartChallengeRequest, StartChallengeResponse,
};
use crate::routes::AppState;

/// Create a new challenge
/// POST /challenges
pub async fn create_challenge(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(request): Json<CreateChallengeRequest>,
) -> Result<(StatusCode, Json<ChallengeResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Challenge creation request from user: {} for challenge: {}",
        auth_user.username,
        request.challenge_name
    );

    // Check if user has permission to create challenges
    if !auth_user.has_any_role(&["challenge.manager", "game.admin"]) {
        tracing::warn!(
            "User {} lacks permission to create challenges",
            auth_user.username
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "Insufficient permissions to create challenges".to_string(),
            }),
        ));
    }

    // Get user details
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

    match TemporalChallenge::create_new(&state.pool, user.user_id, request.clone()).await {
        Ok(temporal_challenge) => {
            // Convert to legacy format for response compatibility
            let challenge = temporal_challenge.to_legacy_challenge().map_err(|e| {
                tracing::error!(
                    "Failed to convert temporal challenge to legacy format: {}",
                    e
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Internal server error".to_string(),
                    }),
                )
            })?;

            // Get challenge data for logging
            let challenge_data = temporal_challenge.get_challenge_data().map_err(|e| {
                tracing::error!("Failed to get challenge data: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Internal server error".to_string(),
                    }),
                )
            })?;

            // Log challenge creation
            if let Err(e) = AuditLog::log_challenge_created(
                &state.pool,
                user.user_id,
                challenge.challenge_id,
                &challenge.challenge_name,
                &challenge_data.challenge_type.to_string(),
                request.waypoints.len() as i32,
            )
            .await
            {
                tracing::warn!("Failed to log challenge creation: {}", e);
            }

            // Get waypoints from temporal challenge
            let waypoints_data = temporal_challenge.get_waypoints().map_err(|e| {
                tracing::error!("Failed to get waypoints: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Internal server error".to_string(),
                    }),
                )
            })?;

            // Convert waypoints to legacy format
            let waypoints = waypoints_data
                .into_iter()
                .map(|wd| {
                    crate::models::Waypoint {
                        waypoint_id: wd.waypoint_id.unwrap_or(0), // Placeholder
                        challenge_id: challenge.challenge_id,
                        waypoint_sequence: wd.waypoint_sequence,
                        location_lat: wd.location.lat,
                        location_lon: wd.location.lon,
                        radius_meters: wd.radius_meters,
                        waypoint_clue: wd.waypoint_clue,
                        hints: Some(wd.hints),
                        waypoint_time_minutes: wd.waypoint_time_minutes,
                        image_subject: wd.image_subject,
                        created_at: wd.created_at.unwrap_or_else(chrono::Utc::now),
                    }
                })
                .collect();

            // For new challenges, participants list is empty
            let participants = vec![];

            tracing::info!(
                "Challenge created successfully: {} (temporal ID: {})",
                challenge.challenge_id,
                temporal_challenge.challenge_id
            );

            let response = ChallengeResponse {
                challenge,
                waypoints,
                participants,
            };

            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(ChallengeError::ValidationFailed(msg)) => {
            tracing::warn!("Challenge creation failed: {}", msg);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: format!("Validation failed: {msg}"),
                }),
            ))
        }
        Err(ChallengeError::InvalidWaypointSequence) => {
            tracing::warn!("Challenge creation failed: Invalid waypoint sequence");
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message:
                        "Invalid waypoint sequence. Sequences must start at 1 and be consecutive"
                            .to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Challenge creation failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Challenge creation failed".to_string(),
                }),
            ))
        }
    }
}

/// Get a challenge by ID
/// GET /challenges/{challenge_id}
pub async fn get_challenge(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<Uuid>,
) -> Result<(StatusCode, Json<ChallengeResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Challenge retrieval request from user: {} for challenge: {}",
        auth_user.username,
        challenge_id
    );

    match Challenge::get_by_id(&state.pool, challenge_id).await {
        Ok(challenge) => {
            let waypoints = challenge
                .get_waypoints(&state.pool)
                .await
                .unwrap_or_default();
            let participants = challenge
                .get_participants(&state.pool)
                .await
                .unwrap_or_default();

            let response = ChallengeResponse {
                challenge,
                waypoints,
                participants,
            };

            Ok((StatusCode::OK, Json(response)))
        }
        Err(ChallengeError::ChallengeNotFound) => {
            tracing::warn!("Challenge not found: {}", challenge_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Challenge not found".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Challenge retrieval failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Challenge retrieval failed".to_string(),
                }),
            ))
        }
    }
}

/// Start a challenge
/// POST /challenges/start
pub async fn start_challenge(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(request): Json<StartChallengeRequest>,
) -> Result<(StatusCode, Json<StartChallengeResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Challenge start request from user: {} for challenge: {}",
        auth_user.username,
        request.challenge_id
    );

    // Check if user has permission to start challenges
    if !auth_user.has_any_role(&["challenge.moderator", "challenge.manager", "game.admin"]) {
        tracing::warn!(
            "User {} lacks permission to start challenges",
            auth_user.username
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "Insufficient permissions to start challenges".to_string(),
            }),
        ));
    }

    // Get user details
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

    // Get challenge
    let mut challenge = match Challenge::get_by_id(&state.pool, request.challenge_id).await {
        Ok(challenge) => challenge,
        Err(ChallengeError::ChallengeNotFound) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Challenge not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get challenge: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Failed to get challenge".to_string(),
                }),
            ));
        }
    };

    match challenge.start_challenge(&state.pool, user.user_id).await {
        Ok(response) => {
            // Log challenge start
            if let Err(e) = AuditLog::log_challenge_started(
                &state.pool,
                user.user_id,
                challenge.challenge_id,
                &challenge.challenge_name,
                response.participants.len() as i32,
                response.planned_start_time,
                response.actual_start_time,
            )
            .await
            {
                tracing::warn!("Failed to log challenge start: {}", e);
            }

            tracing::info!("Challenge started successfully: {}", challenge.challenge_id);

            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(ChallengeError::NotModerator) => {
            tracing::warn!(
                "User {} is not moderator of challenge {}",
                auth_user.username,
                request.challenge_id
            );
            Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    message: "You are not the moderator of this challenge".to_string(),
                }),
            ))
        }
        Err(ChallengeError::ChallengeAlreadyStarted) => {
            tracing::warn!("Challenge already started: {}", request.challenge_id);
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    message: "Challenge has already been started".to_string(),
                }),
            ))
        }
        Err(ChallengeError::ChallengeNotActive) => {
            tracing::warn!("Challenge not active: {}", request.challenge_id);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Challenge is not active".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Challenge start failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Challenge start failed".to_string(),
                }),
            ))
        }
    }
}

/// Invite a user to participate in a challenge
/// POST /challenges/{challenge_id}/invite/{user_id}
pub async fn invite_participant(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path((challenge_id, user_id)): Path<(Uuid, i32)>,
    Json(nickname): Json<Option<String>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Participant invitation from user: {} for challenge: {}, inviting user: {}",
        auth_user.username,
        challenge_id,
        user_id
    );

    // Check if user has permission to invite participants
    if !auth_user.has_any_role(&["challenge.moderator", "challenge.manager", "game.admin"]) {
        tracing::warn!(
            "User {} lacks permission to invite participants",
            auth_user.username
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "Insufficient permissions to invite participants".to_string(),
            }),
        ));
    }

    // Get moderator user details
    let moderator = match state
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

    // Get challenge
    let challenge = match Challenge::get_by_id(&state.pool, challenge_id).await {
        Ok(challenge) => challenge,
        Err(ChallengeError::ChallengeNotFound) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Challenge not found".to_string(),
                }),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get challenge: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Failed to get challenge".to_string(),
                }),
            ));
        }
    };

    // Check if moderator is authorized for this challenge
    if challenge.challenge_moderator != moderator.user_id && !auth_user.has_role("game.admin") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "You are not authorized to modify this challenge".to_string(),
            }),
        ));
    }

    match challenge
        .invite_participant(&state.pool, user_id, nickname.clone())
        .await
    {
        Ok(participant) => {
            // Log participant invitation
            if let Err(e) = AuditLog::log_participant_invited(
                &state.pool,
                moderator.user_id,
                participant.participant_id,
                challenge_id,
                user_id,
                nickname.as_deref(),
            )
            .await
            {
                tracing::warn!("Failed to log participant invitation: {}", e);
            }

            tracing::info!(
                "Participant invited successfully: {} to challenge: {}",
                user_id,
                challenge_id
            );

            Ok(StatusCode::CREATED)
        }
        Err(ChallengeError::AlreadyParticipant) => {
            tracing::warn!(
                "User {} is already a participant in challenge {}",
                user_id,
                challenge_id
            );
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    message: "User is already a participant in this challenge".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Participant invitation failed with error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Participant invitation failed".to_string(),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::challenge::ChallengeType;

    #[test]
    fn test_create_challenge_request_deserialization() {
        let json = r#"{
            "challenge_name": "Test Challenge",
            "challenge_description": "A test challenge",
            "planned_start_time": "2025-01-01T10:00:00Z",
            "duration_minutes": 120,
            "challenge_type": "COM",
            "waypoints": [
                {
                    "waypoint_sequence": 1,
                    "location": {"lat": 51.5074, "long": -0.1278},
                    "radius_meters": 50.0,
                    "waypoint_clue": "Find the red post box",
                    "hints": ["Look near the main street"],
                    "waypoint_time_minutes": 15,
                    "image_subject": "Red post box"
                }
            ]
        }"#;

        let request: CreateChallengeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.challenge_name, "Test Challenge");
        assert_eq!(request.challenge_type, ChallengeType::Com);
        assert_eq!(request.waypoints.len(), 1);
        assert_eq!(request.waypoints[0].waypoint_sequence, 1);
    }

    #[test]
    fn test_start_challenge_request_deserialization() {
        let json = r#"{
            "challenge-id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let request: StartChallengeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.challenge_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
