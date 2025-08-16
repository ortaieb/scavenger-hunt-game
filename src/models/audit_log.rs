use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "audit_event_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEventType {
    UserRegistered,
    UserLogin,
    ChallengeCreated,
    ChallengeStarted,
    ChallengeEnded,
    ParticipantInvited,
    WaypointCheckedIn,
    WaypointProofSubmitted,
    WaypointVerified,
    LocationUpdated,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AuditLog {
    pub log_id: i32,                        // SERIAL PRIMARY KEY - never null
    pub event_time: DateTime<Utc>,          // DEFAULT NOW() - never null
    pub event_type: AuditEventType,         // NOT NULL
    pub user_id: Option<i32>,               // Can be null FK
    pub participant_id: Option<Uuid>,       // Can be null FK
    pub challenge_id: Option<Uuid>,         // Can be null FK
    pub waypoint_id: Option<i32>,           // Can be null FK
    pub event_data: Option<JsonValue>,      // Can be null JSONB
    pub outcome: Option<String>,            // Can be null VARCHAR(50)
    pub outcome_payload: Option<JsonValue>, // Can be null JSONB
    pub created_at: DateTime<Utc>,          // DEFAULT NOW() - never null
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegisteredData {
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoginData {
    pub username: String,
    pub success: bool,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeCreatedData {
    pub challenge_name: String,
    pub challenge_type: String,
    pub waypoint_count: i32,
    pub moderator_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeStartedData {
    pub challenge_name: String,
    pub participant_count: i32,
    pub planned_start_time: DateTime<Utc>,
    pub actual_start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInvitedData {
    pub user_id: i32,
    pub participant_nickname: Option<String>,
    pub invitation_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointCheckedInData {
    pub waypoint_sequence: i32,
    pub location_lat: f64,
    pub location_lon: f64,
    pub distance_from_target: f64,
    pub within_radius: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointProofSubmittedData {
    pub waypoint_sequence: i32,
    pub image_path: String,
    pub processing_id: String,
    pub submission_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointVerifiedData {
    pub waypoint_sequence: i32,
    pub verification_result: String,
    pub verification_reasons: Option<Vec<String>>,
    pub processing_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationUpdatedData {
    pub location_lat: f64,
    pub location_lon: f64,
    pub accuracy_meters: Option<f64>,
    pub update_source: String, // "check_in", "periodic_update", etc.
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid event data")]
    InvalidEventData,
}

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub event_type: AuditEventType,
    pub user_id: Option<i32>,
    pub participant_id: Option<Uuid>,
    pub challenge_id: Option<Uuid>,
    pub waypoint_id: Option<i32>,
    pub event_data: Option<JsonValue>,
    pub outcome: Option<String>,
    pub outcome_payload: Option<JsonValue>,
}

impl AuditLogEntry {
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_type,
            user_id: None,
            participant_id: None,
            challenge_id: None,
            waypoint_id: None,
            event_data: None,
            outcome: None,
            outcome_payload: None,
        }
    }

    pub fn with_user_id(mut self, user_id: i32) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_participant_id(mut self, participant_id: Uuid) -> Self {
        self.participant_id = Some(participant_id);
        self
    }

    pub fn with_challenge_id(mut self, challenge_id: Uuid) -> Self {
        self.challenge_id = Some(challenge_id);
        self
    }

    pub fn with_waypoint_id(mut self, waypoint_id: i32) -> Self {
        self.waypoint_id = Some(waypoint_id);
        self
    }

    pub fn with_event_data(mut self, event_data: JsonValue) -> Self {
        self.event_data = Some(event_data);
        self
    }

    pub fn with_outcome(mut self, outcome: String) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn with_outcome_payload(mut self, outcome_payload: JsonValue) -> Self {
        self.outcome_payload = Some(outcome_payload);
        self
    }
}

impl AuditLog {
    /// Create a new audit log entry
    pub async fn create(pool: &PgPool, entry: AuditLogEntry) -> Result<AuditLog, AuditError> {
        let audit_log = sqlx::query_as!(
            AuditLog,
            r#"
            INSERT INTO audit_log (event_type, user_id, participant_id, challenge_id, waypoint_id,
                                 event_data, outcome, outcome_payload)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING log_id, 
                     COALESCE(event_time, NOW()) as "event_time!",
                     event_type as "event_type: AuditEventType",
                     user_id, participant_id, challenge_id, waypoint_id, event_data,
                     outcome, outcome_payload,
                     COALESCE(created_at, NOW()) as "created_at!"
            "#,
            entry.event_type as AuditEventType,
            entry.user_id,
            entry.participant_id,
            entry.challenge_id,
            entry.waypoint_id,
            entry.event_data,
            entry.outcome,
            entry.outcome_payload
        )
        .fetch_one(pool)
        .await?;

        Ok(audit_log)
    }

    /// Log user registration event
    pub async fn log_user_registered(
        pool: &PgPool,
        user_id: i32,
        username: &str,
        roles: &[String],
    ) -> Result<AuditLog, AuditError> {
        let event_data = UserRegisteredData {
            username: username.to_string(),
            roles: roles.to_vec(),
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::UserRegistered)
                .with_user_id(user_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("success".to_string()),
        )
        .await
    }

    /// Log user login event
    pub async fn log_user_login(
        pool: &PgPool,
        user_id: Option<i32>,
        username: &str,
        success: bool,
        ip_address: Option<&str>,
    ) -> Result<AuditLog, AuditError> {
        let event_data = UserLoginData {
            username: username.to_string(),
            success,
            ip_address: ip_address.map(|s| s.to_string()),
        };

        let outcome = if success { "success" } else { "failed" };

        let mut entry = AuditLogEntry::new(AuditEventType::UserLogin)
            .with_event_data(serde_json::to_value(event_data)?)
            .with_outcome(outcome.to_string());

        if let Some(uid) = user_id {
            entry = entry.with_user_id(uid);
        }

        Self::create(pool, entry).await
    }

    /// Log challenge creation event
    pub async fn log_challenge_created(
        pool: &PgPool,
        user_id: i32,
        challenge_id: Uuid,
        challenge_name: &str,
        challenge_type: &str,
        waypoint_count: i32,
    ) -> Result<AuditLog, AuditError> {
        let event_data = ChallengeCreatedData {
            challenge_name: challenge_name.to_string(),
            challenge_type: challenge_type.to_string(),
            waypoint_count,
            moderator_id: user_id,
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::ChallengeCreated)
                .with_user_id(user_id)
                .with_challenge_id(challenge_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("success".to_string()),
        )
        .await
    }

    /// Log challenge started event
    pub async fn log_challenge_started(
        pool: &PgPool,
        user_id: i32,
        challenge_id: Uuid,
        challenge_name: &str,
        participant_count: i32,
        planned_start_time: DateTime<Utc>,
        actual_start_time: DateTime<Utc>,
    ) -> Result<AuditLog, AuditError> {
        let event_data = ChallengeStartedData {
            challenge_name: challenge_name.to_string(),
            participant_count,
            planned_start_time,
            actual_start_time,
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::ChallengeStarted)
                .with_user_id(user_id)
                .with_challenge_id(challenge_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("success".to_string()),
        )
        .await
    }

    /// Log participant invitation event
    pub async fn log_participant_invited(
        pool: &PgPool,
        moderator_id: i32,
        participant_id: Uuid,
        challenge_id: Uuid,
        invited_user_id: i32,
        participant_nickname: Option<&str>,
    ) -> Result<AuditLog, AuditError> {
        let event_data = ParticipantInvitedData {
            user_id: invited_user_id,
            participant_nickname: participant_nickname.map(|s| s.to_string()),
            invitation_time: Utc::now(),
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::ParticipantInvited)
                .with_user_id(moderator_id)
                .with_participant_id(participant_id)
                .with_challenge_id(challenge_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("success".to_string()),
        )
        .await
    }

    /// Log waypoint check-in event
    pub async fn log_waypoint_checked_in(
        pool: &PgPool,
        participant_id: Uuid,
        challenge_id: Uuid,
        waypoint_id: i32,
        waypoint_sequence: i32,
        location_lat: f64,
        location_lon: f64,
        distance_from_target: f64,
        within_radius: bool,
    ) -> Result<AuditLog, AuditError> {
        let event_data = WaypointCheckedInData {
            waypoint_sequence,
            location_lat,
            location_lon,
            distance_from_target,
            within_radius,
        };

        let outcome = if within_radius { "success" } else { "failed" };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::WaypointCheckedIn)
                .with_participant_id(participant_id)
                .with_challenge_id(challenge_id)
                .with_waypoint_id(waypoint_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome(outcome.to_string()),
        )
        .await
    }

    /// Log waypoint proof submission event
    pub async fn log_waypoint_proof_submitted(
        pool: &PgPool,
        participant_id: Uuid,
        challenge_id: Uuid,
        waypoint_id: i32,
        waypoint_sequence: i32,
        image_path: &str,
        processing_id: &str,
    ) -> Result<AuditLog, AuditError> {
        let event_data = WaypointProofSubmittedData {
            waypoint_sequence,
            image_path: image_path.to_string(),
            processing_id: processing_id.to_string(),
            submission_time: Utc::now(),
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::WaypointProofSubmitted)
                .with_participant_id(participant_id)
                .with_challenge_id(challenge_id)
                .with_waypoint_id(waypoint_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("submitted".to_string()),
        )
        .await
    }

    /// Log waypoint verification event
    pub async fn log_waypoint_verified(
        pool: &PgPool,
        participant_id: Uuid,
        challenge_id: Uuid,
        waypoint_id: i32,
        waypoint_sequence: i32,
        verification_result: &str,
        verification_reasons: Option<&[String]>,
        processing_time_seconds: f64,
        outcome_payload: Option<JsonValue>,
    ) -> Result<AuditLog, AuditError> {
        let event_data = WaypointVerifiedData {
            waypoint_sequence,
            verification_result: verification_result.to_string(),
            verification_reasons: verification_reasons.map(|r| r.to_vec()),
            processing_time_seconds,
        };

        let mut entry = AuditLogEntry::new(AuditEventType::WaypointVerified)
            .with_participant_id(participant_id)
            .with_challenge_id(challenge_id)
            .with_waypoint_id(waypoint_id)
            .with_event_data(serde_json::to_value(event_data)?)
            .with_outcome(verification_result.to_string());

        if let Some(payload) = outcome_payload {
            entry = entry.with_outcome_payload(payload);
        }

        Self::create(pool, entry).await
    }

    /// Log location update event
    pub async fn log_location_updated(
        pool: &PgPool,
        participant_id: Uuid,
        challenge_id: Uuid,
        location_lat: f64,
        location_lon: f64,
        accuracy_meters: Option<f64>,
        update_source: &str,
    ) -> Result<AuditLog, AuditError> {
        let event_data = LocationUpdatedData {
            location_lat,
            location_lon,
            accuracy_meters,
            update_source: update_source.to_string(),
        };

        Self::create(
            pool,
            AuditLogEntry::new(AuditEventType::LocationUpdated)
                .with_participant_id(participant_id)
                .with_challenge_id(challenge_id)
                .with_event_data(serde_json::to_value(event_data)?)
                .with_outcome("success".to_string()),
        )
        .await
    }

    /// Get audit logs for a challenge
    pub async fn get_challenge_logs(
        pool: &PgPool,
        challenge_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let limit = limit.unwrap_or(100);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT log_id, 
                   COALESCE(event_time, NOW()) as "event_time!",
                   event_type as "event_type: AuditEventType",
                   user_id, participant_id, challenge_id, waypoint_id, event_data,
                   outcome, outcome_payload,
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM audit_log
            WHERE challenge_id = $1
            ORDER BY event_time DESC
            LIMIT $2
            "#,
            challenge_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// Get audit logs for a participant
    pub async fn get_participant_logs(
        pool: &PgPool,
        participant_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let limit = limit.unwrap_or(100);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT log_id, 
                   COALESCE(event_time, NOW()) as "event_time!",
                   event_type as "event_type: AuditEventType",
                   user_id, participant_id, challenge_id, waypoint_id, event_data,
                   outcome, outcome_payload,
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM audit_log
            WHERE participant_id = $1
            ORDER BY event_time DESC
            LIMIT $2
            "#,
            participant_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// Get audit logs for a user
    pub async fn get_user_logs(
        pool: &PgPool,
        user_id: i32,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let limit = limit.unwrap_or(100);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT log_id, 
                   COALESCE(event_time, NOW()) as "event_time!",
                   event_type as "event_type: AuditEventType",
                   user_id, participant_id, challenge_id, waypoint_id, event_data,
                   outcome, outcome_payload,
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM audit_log
            WHERE user_id = $1
            ORDER BY event_time DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// Get audit logs by event type
    pub async fn get_logs_by_type(
        pool: &PgPool,
        event_type: AuditEventType,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let limit = limit.unwrap_or(100);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT log_id, 
                   COALESCE(event_time, NOW()) as "event_time!",
                   event_type as "event_type: AuditEventType",
                   user_id, participant_id, challenge_id, waypoint_id, event_data,
                   outcome, outcome_payload,
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM audit_log
            WHERE event_type = $1
            ORDER BY event_time DESC
            LIMIT $2
            "#,
            event_type as AuditEventType,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&AuditEventType::UserRegistered).unwrap(),
            "\"UserRegistered\""
        );
        assert_eq!(
            serde_json::to_string(&AuditEventType::WaypointCheckedIn).unwrap(),
            "\"WaypointCheckedIn\""
        );
    }

    #[test]
    fn test_event_data_serialization() {
        let data = UserRegisteredData {
            username: "test@example.com".to_string(),
            roles: vec!["user.verified".to_string()],
        };

        let json = serde_json::to_value(data).unwrap();
        assert!(json["username"].as_str().unwrap() == "test@example.com");
        assert!(json["roles"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_location_data_serialization() {
        let data = LocationUpdatedData {
            location_lat: 51.5074,
            location_lon: -0.1278,
            accuracy_meters: Some(10.0),
            update_source: "check_in".to_string(),
        };

        let json = serde_json::to_value(data).unwrap();
        assert_eq!(json["location_lat"].as_f64().unwrap(), 51.5074);
        assert_eq!(json["location_lon"].as_f64().unwrap(), -0.1278);
        assert_eq!(json["accuracy_meters"].as_f64().unwrap(), 10.0);
        assert_eq!(json["update_source"].as_str().unwrap(), "check_in");
    }
}
