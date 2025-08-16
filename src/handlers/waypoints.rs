use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::auth::{AuthenticatedParticipant, ErrorResponse};
use crate::models::{AuditLog, ChallengeParticipant, Waypoint, WaypointState};
use crate::routes::AppState;
use crate::services::LocationValidationRequest;

#[derive(serde::Serialize)]
pub struct CheckInResponse {
    #[serde(rename = "challenge-id")]
    pub challenge_id: String,
    #[serde(rename = "participant-id")]
    pub participant_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "waypoint-id")]
    pub waypoint_id: i32,
    pub state: String,
    pub proof: String,
}

#[derive(serde::Serialize)]
pub struct ProofResponse {
    #[serde(rename = "challenge-id")]
    pub challenge_id: String,
    #[serde(rename = "participant-id")]
    pub participant_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "waypoint-id")]
    pub waypoint_id: i32,
    pub state: String,
}

/// Handle waypoint check-in
/// POST /challenges/waypoints/{waypoint_id}/checkin
pub async fn check_in_waypoint(
    auth_participant: AuthenticatedParticipant,
    State(state): State<AppState>,
    Path(waypoint_id): Path<i32>,
    Json(request): Json<LocationValidationRequest>,
) -> Result<Json<CheckInResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Waypoint check-in from participant: {} for waypoint: {}",
        auth_participant.participant_id,
        waypoint_id
    );

    // Get participant
    let participant_id = Uuid::parse_str(&auth_participant.participant_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Invalid participant ID".to_string(),
            }),
        )
    })?;

    let mut participant = match ChallengeParticipant::get_by_id(&state.pool, participant_id).await {
        Ok(participant) => participant,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Participant not found".to_string(),
                }),
            ));
        }
    };

    // Get waypoint
    let waypoint = match Waypoint::get_by_id(&state.pool, waypoint_id).await {
        Ok(waypoint) => waypoint,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Waypoint not found".to_string(),
                }),
            ));
        }
    };

    // Verify participant belongs to the same challenge as the waypoint
    if participant.challenge_id != waypoint.challenge_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "Waypoint does not belong to participant's challenge".to_string(),
            }),
        ));
    }

    // Check if this is the participant's current waypoint
    if participant.current_waypoint_id != Some(waypoint_id) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "This is not your current waypoint".to_string(),
            }),
        ));
    }

    // Validate location
    let validation_result = match state
        .location_service
        .validate_waypoint_location(waypoint_id, &request.location)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Location validation failed: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Location validation failed".to_string(),
                }),
            ));
        }
    };

    // Log the check-in attempt
    if let Err(e) = AuditLog::log_waypoint_checked_in(
        &state.pool,
        crate::models::audit_log::WaypointCheckInParams {
            participant_id,
            challenge_id: participant.challenge_id,
            waypoint_id,
            waypoint_sequence: waypoint.waypoint_sequence,
            location_lat: request.location.lat,
            location_lon: request.location.lon,
            distance_from_target: validation_result.distance_meters,
            within_radius: validation_result.is_valid,
        },
    )
    .await
    {
        tracing::warn!("Failed to log waypoint check-in: {}", e);
    }

    // Log participant location
    if let Err(e) = state
        .location_service
        .log_participant_location(participant_id, &request.location, None)
        .await
    {
        tracing::warn!("Failed to log participant location: {}", e);
    }

    if !validation_result.is_valid {
        tracing::warn!(
            "Check-in failed for participant {} at waypoint {}: too far from target",
            participant_id,
            waypoint_id
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Your checkin attempt is too far from the target".to_string(),
            }),
        ));
    }

    // Update participant state to CHECKED_IN
    if let Err(e) = participant
        .update_waypoint_state(&state.pool, waypoint_id, WaypointState::CheckedIn)
        .await
    {
        tracing::error!("Failed to update participant state: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Failed to update check-in state".to_string(),
            }),
        ));
    }

    tracing::info!(
        "Check-in successful for participant {} at waypoint {}",
        participant_id,
        waypoint_id
    );

    Ok(Json(CheckInResponse {
        challenge_id: participant.challenge_id.to_string(),
        participant_id: participant_id.to_string(),
        timestamp: chrono::Utc::now(),
        waypoint_id,
        state: "CHECKED_IN".to_string(),
        proof: waypoint.image_subject.clone(),
    }))
}

/// Handle waypoint proof submission
/// POST /challenges/waypoints/{waypoint_id}/proof
pub async fn submit_waypoint_proof(
    auth_participant: AuthenticatedParticipant,
    State(state): State<AppState>,
    Path(waypoint_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Json<ProofResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        "Waypoint proof submission from participant: {} for waypoint: {}",
        auth_participant.participant_id,
        waypoint_id
    );

    // Get participant
    let participant_id = Uuid::parse_str(&auth_participant.participant_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Invalid participant ID".to_string(),
            }),
        )
    })?;

    let mut participant = match ChallengeParticipant::get_by_id(&state.pool, participant_id).await {
        Ok(participant) => participant,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Participant not found".to_string(),
                }),
            ));
        }
    };

    // Get waypoint
    let waypoint = match Waypoint::get_by_id(&state.pool, waypoint_id).await {
        Ok(waypoint) => waypoint,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "Waypoint not found".to_string(),
                }),
            ));
        }
    };

    // Verify participant belongs to the same challenge as the waypoint
    if participant.challenge_id != waypoint.challenge_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                message: "Waypoint does not belong to participant's challenge".to_string(),
            }),
        ));
    }

    // Check if participant is checked in to this waypoint
    if participant.current_waypoint_id != Some(waypoint_id)
        || participant.current_state != WaypointState::CheckedIn
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "You must check in to this waypoint before submitting proof".to_string(),
            }),
        ));
    }

    // Extract image from multipart form
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read multipart field: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Invalid multipart data".to_string(),
            }),
        )
    })? {
        if field.name() == Some("image") {
            image_filename = field.file_name().map(|s| s.to_string());
            image_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to read image data: {}", e);
                        (
                            StatusCode::BAD_REQUEST,
                            Json(ErrorResponse {
                                message: "Failed to read image data".to_string(),
                            }),
                        )
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let _image_data = image_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "No image provided".to_string(),
            }),
        )
    })?;

    let image_filename = image_filename.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "No image filename provided".to_string(),
            }),
        )
    })?;

    // Validate image format
    if let Err(e) = state.image_service.validate_image_format(&image_filename) {
        tracing::warn!("Invalid image format: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: format!("Invalid image format: {e}"),
            }),
        ));
    }

    // Create a unique filename for the image
    let unique_filename = format!(
        "{}/{}/{}_{}_{}",
        participant.challenge_id,
        participant_id,
        waypoint_id,
        chrono::Utc::now().timestamp(),
        image_filename
    );

    // TODO: Save image to storage (local or cloud)
    // For now, we'll use the unique filename as the image path
    let image_path = unique_filename;

    // Get waypoint location for validation
    let waypoint_location = waypoint.get_location();

    // Submit image for validation
    let processing_id = uuid::Uuid::new_v4().to_string();

    // Log proof submission
    if let Err(e) = AuditLog::log_waypoint_proof_submitted(
        &state.pool,
        participant_id,
        participant.challenge_id,
        waypoint_id,
        waypoint.waypoint_sequence,
        &image_path,
        &processing_id,
    )
    .await
    {
        tracing::warn!("Failed to log waypoint proof submission: {}", e);
    }

    // Validate image with the external service
    let validation_start = std::time::Instant::now();
    let validation_result = match state
        .image_service
        .validate_image(
            &image_path,
            &waypoint.image_subject,
            Some(&waypoint_location),
            Some(waypoint.radius_meters),
            None, // TODO: Add time constraints if needed
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Image validation failed: {}", e);

            // Log validation failure
            if let Err(log_err) = AuditLog::log_waypoint_verified(
                &state.pool,
                crate::models::audit_log::WaypointVerificationParams {
                    participant_id,
                    challenge_id: participant.challenge_id,
                    waypoint_id,
                    waypoint_sequence: waypoint.waypoint_sequence,
                    verification_result: "failed",
                    verification_reasons: Some(&[format!("Validation service error: {e}")]),
                    processing_time_seconds: validation_start.elapsed().as_secs_f64(),
                    outcome_payload: None,
                },
            )
            .await
            {
                tracing::warn!("Failed to log waypoint verification failure: {}", log_err);
            }

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Image validation service unavailable".to_string(),
                }),
            ));
        }
    };

    let processing_time = validation_start.elapsed().as_secs_f64();

    // Check validation result
    if validation_result.resolution == "accepted" {
        // Update participant state to VERIFIED and advance to next waypoint
        if let Err(e) = participant
            .update_waypoint_state(&state.pool, waypoint_id, WaypointState::Verified)
            .await
        {
            tracing::error!("Failed to update participant state to verified: {}", e);
        }

        // Try to advance to next waypoint
        match participant.advance_to_next_waypoint(&state.pool).await {
            Ok(next_waypoint) => {
                if next_waypoint.is_some() {
                    tracing::info!("Participant {} advanced to next waypoint", participant_id);
                } else {
                    tracing::info!("Participant {} completed all waypoints", participant_id);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to advance participant to next waypoint: {}", e);
            }
        }

        // Log successful verification
        if let Err(e) = AuditLog::log_waypoint_verified(
            &state.pool,
            crate::models::audit_log::WaypointVerificationParams {
                participant_id,
                challenge_id: participant.challenge_id,
                waypoint_id,
                waypoint_sequence: waypoint.waypoint_sequence,
                verification_result: "accepted",
                verification_reasons: validation_result.reasons.as_deref(),
                processing_time_seconds: processing_time,
                outcome_payload: None,
            },
        )
        .await
        {
            tracing::warn!("Failed to log waypoint verification success: {}", e);
        }

        tracing::info!(
            "Proof verification successful for participant {} at waypoint {}",
            participant_id,
            waypoint_id
        );

        Ok(Json(ProofResponse {
            challenge_id: participant.challenge_id.to_string(),
            participant_id: participant_id.to_string(),
            timestamp: chrono::Utc::now(),
            waypoint_id,
            state: "VERIFIED".to_string(),
        }))
    } else {
        // Log failed verification
        if let Err(e) = AuditLog::log_waypoint_verified(
            &state.pool,
            crate::models::audit_log::WaypointVerificationParams {
                participant_id,
                challenge_id: participant.challenge_id,
                waypoint_id,
                waypoint_sequence: waypoint.waypoint_sequence,
                verification_result: "rejected",
                verification_reasons: validation_result.reasons.as_deref(),
                processing_time_seconds: processing_time,
                outcome_payload: None,
            },
        )
        .await
        {
            tracing::warn!("Failed to log waypoint verification failure: {}", e);
        }

        // Build error message with reasons
        let mut error_message = "Failed to provide a proof.".to_string();
        if let Some(reasons) = validation_result.reasons {
            for (i, reason) in reasons.iter().enumerate() {
                error_message.push_str(&format!(" [{}] {}", i + 1, reason));
            }
        }

        tracing::warn!(
            "Proof verification failed for participant {} at waypoint {}: {}",
            participant_id,
            waypoint_id,
            error_message
        );

        Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: error_message,
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_validation_request_deserialization() {
        let json = r#"{
            "location": {
                "lat": 51.5074,
                "long": -0.1278
            }
        }"#;

        let request: LocationValidationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.location.lat, 51.5074);
        assert_eq!(request.location.lon, -0.1278);
    }

    #[test]
    fn test_check_in_response_serialization() {
        use chrono::Utc;

        let response = CheckInResponse {
            challenge_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            participant_id: "participant-123".to_string(),
            timestamp: Utc::now(),
            waypoint_id: 1,
            state: "CHECKED_IN".to_string(),
            proof: "Red post box".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("challenge-id"));
        assert!(json.contains("participant-id"));
        assert!(json.contains("waypoint-id"));
        assert!(json.contains("CHECKED_IN"));
    }

    #[test]
    fn test_proof_response_serialization() {
        use chrono::Utc;

        let response = ProofResponse {
            challenge_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            participant_id: "participant-123".to_string(),
            timestamp: Utc::now(),
            waypoint_id: 1,
            state: "VERIFIED".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("challenge-id"));
        assert!(json.contains("participant-id"));
        assert!(json.contains("waypoint-id"));
        assert!(json.contains("VERIFIED"));
    }
}
