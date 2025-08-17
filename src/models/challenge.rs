use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Type};
use uuid::Uuid;

use crate::services::location_service::GeoLocation;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "challenge_type", rename_all = "UPPERCASE")]
pub enum ChallengeType {
    #[serde(rename = "REC")]
    Rec, // Recreational
    #[serde(rename = "COM")]
    Com, // Competitive
    #[serde(rename = "RES")]
    Res, // Restricted
}

impl std::fmt::Display for ChallengeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ChallengeType::Rec => "REC",
            ChallengeType::Com => "COM",
            ChallengeType::Res => "RES",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "waypoint_state", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaypointState {
    Presented,
    CheckedIn,
    Verified,
}

// New temporal challenge structure for JSON storage
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TemporalChallenge {
    pub challenge_id: i32,                 // Sequence-based ID
    pub challenge_version_id: i32,         // Unique version ID
    pub challenge_name: String,            // NOT NULL
    pub planned_start_time: DateTime<Utc>, // NOT NULL
    pub challenge: serde_json::Value,      // JSONB - the complete challenge data
    pub start_at: DateTime<Utc>,           // Temporal validity start
    pub end_at: Option<DateTime<Utc>>,     // Temporal validity end (NULL = current)
    pub created_at: DateTime<Utc>,         // Record creation time
    pub updated_at: DateTime<Utc>,         // Record update time
}

// JSON structure that will be stored in the challenge JSONB field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeData {
    pub challenge_id: i32, // For backwards compatibility
    pub challenge_description: Option<String>,
    pub challenge_moderator: i32,
    pub actual_start_time: Option<DateTime<Utc>>,
    pub duration_minutes: i32,
    pub challenge_type: ChallengeType,
    pub active: bool,
    pub waypoints: Vec<WaypointData>,
    pub metadata: ChallengeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointData {
    pub waypoint_id: Option<i32>, // Optional for new waypoints
    pub waypoint_sequence: i32,
    pub location: GeoLocation,
    pub radius_meters: f64,
    pub waypoint_clue: String,
    pub hints: Vec<String>,
    pub waypoint_time_minutes: Option<i32>,
    pub image_subject: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub migrated_from_relational: Option<bool>,
    pub version_notes: Option<String>,
}

// Legacy Challenge struct removed - now using TemporalChallenge with JSON storage
// Legacy Waypoint struct removed - now embedded in challenge JSON as WaypointData

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChallengeParticipant {
    pub participant_id: Uuid,                 // UUID PRIMARY KEY - never null
    pub challenge_id: i32,                    // NOT NULL FK to temporal_challenges - now integer
    pub user_id: i32,                         // NOT NULL FK to users
    pub participant_nickname: Option<String>, // Can be null
    pub current_waypoint_id: Option<i32>,     // Can be null - now obsolete (waypoints in JSON)
    pub current_state: WaypointState,         // DEFAULT 'PRESENTED' - never null
    pub joined_at: DateTime<Utc>,             // DEFAULT NOW() - never null
    pub last_updated: DateTime<Utc>,          // DEFAULT NOW() - never null
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateChallengeRequest {
    pub challenge_name: String,
    pub challenge_description: Option<String>,
    pub planned_start_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub challenge_type: ChallengeType,
    pub waypoints: Vec<CreateWaypointRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWaypointRequest {
    pub waypoint_sequence: i32,
    pub location: GeoLocation,
    pub radius_meters: f64,
    pub waypoint_clue: String,
    pub hints: Vec<String>,
    pub waypoint_time_minutes: Option<i32>,
    pub image_subject: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartChallengeRequest {
    #[serde(rename = "challenge-id")]
    pub challenge_id: i32, // Now integer instead of UUID
}

#[derive(Debug, Clone, Serialize)]
pub struct StartChallengeResponse {
    #[serde(rename = "challenge-id")]
    pub challenge_id: i32, // Now integer instead of UUID
    #[serde(rename = "planned-start-time")]
    pub planned_start_time: DateTime<Utc>,
    #[serde(rename = "actual-start-time")]
    pub actual_start_time: DateTime<Utc>,
    pub duration: i32,
    pub participants: Vec<ParticipantInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParticipantInfo {
    #[serde(rename = "user-id")]
    pub user_id: i32,
    #[serde(rename = "participant-id")]
    pub participant_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChallengeResponse {
    pub challenge: TemporalChallenge, // Now using TemporalChallenge
    pub waypoints: Vec<WaypointData>, // Now using WaypointData from JSON
    pub participants: Vec<ChallengeParticipant>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Challenge not found")]
    ChallengeNotFound,
    #[error("Waypoint not found")]
    WaypointNotFound,
    #[error("Participant not found")]
    ParticipantNotFound,
    #[error("Challenge already started")]
    ChallengeAlreadyStarted,
    #[error("Challenge not active")]
    ChallengeNotActive,
    #[error("User not moderator of challenge")]
    NotModerator,
    #[error("User already participant in challenge")]
    AlreadyParticipant,
    #[error("Invalid waypoint sequence")]
    InvalidWaypointSequence,
    #[error("Challenge validation failed: {0}")]
    ValidationFailed(String),
}

// Legacy Challenge implementation removed - now using TemporalChallenge

// Legacy Waypoint implementation removed - now using WaypointData in JSON

impl ChallengeParticipant {
    pub async fn get_by_id(
        pool: &PgPool,
        participant_id: Uuid,
    ) -> Result<ChallengeParticipant, ChallengeError> {
        let participant = sqlx::query_as!(
            ChallengeParticipant,
            r#"
            SELECT participant_id as "participant_id!", challenge_id as "challenge_id!", user_id as "user_id!", participant_nickname,
                   current_waypoint_id, 
                   COALESCE(current_state, 'PRESENTED') as "current_state!: WaypointState",
                   COALESCE(joined_at, NOW()) as "joined_at!",
                   COALESCE(last_updated, NOW()) as "last_updated!"
            FROM challenge_participants
            WHERE participant_id = $1
            "#,
            participant_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ChallengeError::ParticipantNotFound)?;

        Ok(participant)
    }

    pub async fn get_by_challenge_and_user(
        pool: &PgPool,
        challenge_id: i32,
        user_id: i32,
    ) -> Result<Option<ChallengeParticipant>, ChallengeError> {
        let participant = sqlx::query_as!(
            ChallengeParticipant,
            r#"
            SELECT participant_id as "participant_id!", challenge_id as "challenge_id!", user_id as "user_id!", participant_nickname,
                   current_waypoint_id, 
                   COALESCE(current_state, 'PRESENTED') as "current_state!: WaypointState",
                   COALESCE(joined_at, NOW()) as "joined_at!",
                   COALESCE(last_updated, NOW()) as "last_updated!"
            FROM challenge_participants
            WHERE challenge_id = $1 AND user_id = $2
            "#,
            challenge_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(participant)
    }

    pub async fn create_for_challenge(
        pool: &PgPool,
        challenge_id: i32,
        user_id: i32,
        nickname: Option<String>,
    ) -> Result<ChallengeParticipant, ChallengeError> {
        // Check if user is already a participant
        if (Self::get_by_challenge_and_user(pool, challenge_id, user_id).await?).is_some() {
            return Err(ChallengeError::AlreadyParticipant);
        }

        let participant = sqlx::query_as!(
            ChallengeParticipant,
            r#"
            INSERT INTO challenge_participants (challenge_id, user_id, participant_nickname)
            VALUES ($1, $2, $3)
            RETURNING participant_id as "participant_id!", challenge_id as "challenge_id!", user_id as "user_id!", participant_nickname,
                     current_waypoint_id, 
                     COALESCE(current_state, 'PRESENTED') as "current_state!: WaypointState",
                     COALESCE(joined_at, NOW()) as "joined_at!",
                     COALESCE(last_updated, NOW()) as "last_updated!"
            "#,
            challenge_id,
            user_id,
            nickname
        )
        .fetch_one(pool)
        .await?;

        Ok(participant)
    }

    pub async fn update_state(
        &mut self,
        pool: &PgPool,
        state: WaypointState,
    ) -> Result<(), ChallengeError> {
        let now = Utc::now();

        sqlx::query!(
            r#"
            UPDATE challenge_participants
            SET current_state = $1, last_updated = $2
            WHERE participant_id = $3
            "#,
            state as WaypointState,
            now,
            self.participant_id
        )
        .execute(pool)
        .await?;

        self.current_state = state;
        self.last_updated = now;

        Ok(())
    }

    pub async fn get_participants_for_challenge(
        pool: &PgPool,
        challenge_id: i32,
    ) -> Result<Vec<ChallengeParticipant>, ChallengeError> {
        let participants = sqlx::query_as!(
            ChallengeParticipant,
            r#"
            SELECT participant_id as "participant_id!", challenge_id as "challenge_id!", user_id as "user_id!", participant_nickname,
                   current_waypoint_id, 
                   COALESCE(current_state, 'PRESENTED') as "current_state!: WaypointState",
                   COALESCE(joined_at, NOW()) as "joined_at!",
                   COALESCE(last_updated, NOW()) as "last_updated!"
            FROM challenge_participants
            WHERE challenge_id = $1
            "#,
            challenge_id
        )
        .fetch_all(pool)
        .await?;

        Ok(participants)
    }
}

// Implementation for TemporalChallenge (new JSON-based storage)
impl TemporalChallenge {
    pub async fn create_new(
        pool: &PgPool,
        moderator_id: i32,
        request: CreateChallengeRequest,
    ) -> Result<TemporalChallenge, ChallengeError> {
        // Validate waypoint sequences
        Self::validate_waypoint_sequences(&request.waypoints)?;

        let mut tx = pool.begin().await?;

        // Get next challenge_id from sequence
        let challenge_id: i32 = sqlx::query_scalar!("SELECT nextval('challenge_id_seq')::int")
            .fetch_one(&mut *tx)
            .await?
            .unwrap();

        // Build waypoints data
        let waypoints_data: Vec<WaypointData> = request
            .waypoints
            .into_iter()
            .map(|wp| WaypointData {
                waypoint_id: None, // Will be generated if needed
                waypoint_sequence: wp.waypoint_sequence,
                location: wp.location,
                radius_meters: wp.radius_meters,
                waypoint_clue: wp.waypoint_clue,
                hints: wp.hints,
                waypoint_time_minutes: wp.waypoint_time_minutes,
                image_subject: wp.image_subject,
                created_at: Some(Utc::now()),
            })
            .collect();

        // Build challenge data
        let challenge_data = ChallengeData {
            challenge_id,
            challenge_description: request.challenge_description,
            challenge_moderator: moderator_id,
            actual_start_time: None,
            duration_minutes: request.duration_minutes,
            challenge_type: request.challenge_type,
            active: true,
            waypoints: waypoints_data,
            metadata: ChallengeMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                migrated_from_relational: None,
                version_notes: Some("Initial version".to_string()),
            },
        };

        // Convert to JSON
        let challenge_json = serde_json::to_value(&challenge_data).map_err(|e| {
            ChallengeError::ValidationFailed(format!("JSON serialization failed: {e}"))
        })?;

        // Insert into temporal_challenges table
        let temporal_challenge = sqlx::query_as!(
            TemporalChallenge,
            r#"
            INSERT INTO temporal_challenges (challenge_id, challenge_name, planned_start_time, challenge)
            VALUES ($1, $2, $3, $4)
            RETURNING challenge_id as "challenge_id!", challenge_version_id as "challenge_version_id!", 
                     challenge_name as "challenge_name!", planned_start_time as "planned_start_time!",
                     challenge as "challenge!", start_at as "start_at!", end_at, 
                     created_at as "created_at!", updated_at as "updated_at!"
            "#,
            challenge_id,
            request.challenge_name,
            request.planned_start_time,
            challenge_json
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(temporal_challenge)
    }

    pub async fn get_current_by_id(
        pool: &PgPool,
        challenge_id: i32,
    ) -> Result<TemporalChallenge, ChallengeError> {
        let temporal_challenge = sqlx::query_as!(
            TemporalChallenge,
            r#"
            SELECT challenge_id as "challenge_id!", challenge_version_id as "challenge_version_id!", 
                   challenge_name as "challenge_name!", planned_start_time as "planned_start_time!",
                   challenge as "challenge!", start_at as "start_at!", end_at, 
                   created_at as "created_at!", updated_at as "updated_at!"
            FROM temporal_challenges
            WHERE challenge_id = $1 AND end_at IS NULL
            "#,
            challenge_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ChallengeError::ChallengeNotFound)?;

        Ok(temporal_challenge)
    }

    pub async fn get_by_version_id(
        pool: &PgPool,
        version_id: i32,
    ) -> Result<TemporalChallenge, ChallengeError> {
        let temporal_challenge = sqlx::query_as!(
            TemporalChallenge,
            r#"
            SELECT challenge_id as "challenge_id!", challenge_version_id as "challenge_version_id!", 
                   challenge_name as "challenge_name!", planned_start_time as "planned_start_time!",
                   challenge as "challenge!", start_at as "start_at!", end_at, 
                   created_at as "created_at!", updated_at as "updated_at!"
            FROM temporal_challenges
            WHERE challenge_version_id = $1
            "#,
            version_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ChallengeError::ChallengeNotFound)?;

        Ok(temporal_challenge)
    }

    pub fn get_challenge_data(&self) -> Result<ChallengeData, ChallengeError> {
        serde_json::from_value(self.challenge.clone()).map_err(|e| {
            ChallengeError::ValidationFailed(format!("JSON deserialization failed: {e}"))
        })
    }

    pub async fn create_new_version(
        &self,
        pool: &PgPool,
        updated_data: ChallengeData,
        version_notes: Option<String>,
    ) -> Result<TemporalChallenge, ChallengeError> {
        let mut updated_challenge_data = updated_data;
        updated_challenge_data.metadata.updated_at = Utc::now();
        updated_challenge_data.metadata.version_notes = version_notes;

        let challenge_json = serde_json::to_value(&updated_challenge_data).map_err(|e| {
            ChallengeError::ValidationFailed(format!("JSON serialization failed: {e}"))
        })?;

        // TODO: For now, create a simple new version without using the stored function
        // In production, you'd want to use the create_challenge_version function
        let mut tx = pool.begin().await?;

        // End current version
        sqlx::query!(
            "UPDATE temporal_challenges SET end_at = NOW(), updated_at = NOW() WHERE challenge_id = $1 AND end_at IS NULL",
            self.challenge_id
        )
        .execute(&mut *tx)
        .await?;

        // Create new version
        let new_version = sqlx::query_as!(
            TemporalChallenge,
            r#"
            INSERT INTO temporal_challenges (challenge_id, challenge_name, planned_start_time, challenge)
            VALUES ($1, $2, $3, $4)
            RETURNING challenge_id as "challenge_id!", challenge_version_id as "challenge_version_id!", 
                     challenge_name as "challenge_name!", planned_start_time as "planned_start_time!",
                     challenge as "challenge!", start_at as "start_at!", end_at, 
                     created_at as "created_at!", updated_at as "updated_at!"
            "#,
            self.challenge_id,
            self.challenge_name,
            self.planned_start_time,
            challenge_json
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(new_version)
    }

    pub async fn start_challenge(
        &self,
        pool: &PgPool,
        moderator_id: i32,
    ) -> Result<TemporalChallenge, ChallengeError> {
        let mut challenge_data = self.get_challenge_data()?;

        // Check if user is moderator
        if challenge_data.challenge_moderator != moderator_id {
            return Err(ChallengeError::NotModerator);
        }

        // Check if challenge is active
        if !challenge_data.active {
            return Err(ChallengeError::ChallengeNotActive);
        }

        // Check if already started
        if challenge_data.actual_start_time.is_some() {
            return Err(ChallengeError::ChallengeAlreadyStarted);
        }

        // Update challenge with actual start time
        challenge_data.actual_start_time = Some(Utc::now());

        self.create_new_version(pool, challenge_data, Some("Challenge started".to_string()))
            .await
    }

    pub fn get_waypoints(&self) -> Result<Vec<WaypointData>, ChallengeError> {
        let challenge_data = self.get_challenge_data()?;
        Ok(challenge_data.waypoints)
    }

    pub fn get_first_waypoint(&self) -> Result<Option<WaypointData>, ChallengeError> {
        let waypoints = self.get_waypoints()?;
        Ok(waypoints.into_iter().min_by_key(|w| w.waypoint_sequence))
    }

    pub fn is_ended(&self) -> Result<bool, ChallengeError> {
        let challenge_data = self.get_challenge_data()?;
        if let Some(start_time) = challenge_data.actual_start_time {
            let end_time =
                start_time + chrono::Duration::minutes(challenge_data.duration_minutes as i64);
            Ok(Utc::now() > end_time)
        } else {
            Ok(false)
        }
    }

    pub fn get_end_time(&self) -> Result<Option<DateTime<Utc>>, ChallengeError> {
        let challenge_data = self.get_challenge_data()?;
        Ok(challenge_data
            .actual_start_time
            .map(|start| start + chrono::Duration::minutes(challenge_data.duration_minutes as i64)))
    }

    fn validate_waypoint_sequences(
        waypoints: &[CreateWaypointRequest],
    ) -> Result<(), ChallengeError> {
        if waypoints.is_empty() {
            return Err(ChallengeError::ValidationFailed(
                "Challenge must have at least one waypoint".to_string(),
            ));
        }

        // Check for duplicate sequences and gaps
        let mut sequences: Vec<i32> = waypoints.iter().map(|w| w.waypoint_sequence).collect();
        sequences.sort();

        for (i, &seq) in sequences.iter().enumerate() {
            let expected = i as i32 + 1;
            if seq != expected {
                return Err(ChallengeError::InvalidWaypointSequence);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ChallengeType::Rec).unwrap(),
            "\"REC\""
        );
        assert_eq!(
            serde_json::to_string(&ChallengeType::Com).unwrap(),
            "\"COM\""
        );
        assert_eq!(
            serde_json::to_string(&ChallengeType::Res).unwrap(),
            "\"RES\""
        );
    }

    #[test]
    fn test_waypoint_state_serialization() {
        assert_eq!(
            serde_json::to_string(&WaypointState::Presented).unwrap(),
            "\"Presented\""
        );
        assert_eq!(
            serde_json::to_string(&WaypointState::CheckedIn).unwrap(),
            "\"CheckedIn\""
        );
        assert_eq!(
            serde_json::to_string(&WaypointState::Verified).unwrap(),
            "\"Verified\""
        );
    }

    #[test]
    fn test_waypoint_sequence_validation() {
        use crate::services::location_service::GeoLocation;

        // Valid sequence
        let valid_waypoints = vec![
            CreateWaypointRequest {
                waypoint_sequence: 1,
                location: GeoLocation { lat: 1.0, lon: 1.0 },
                radius_meters: 50.0,
                waypoint_clue: "Test".to_string(),
                hints: vec![],
                waypoint_time_minutes: None,
                image_subject: "Test".to_string(),
            },
            CreateWaypointRequest {
                waypoint_sequence: 2,
                location: GeoLocation { lat: 2.0, lon: 2.0 },
                radius_meters: 50.0,
                waypoint_clue: "Test".to_string(),
                hints: vec![],
                waypoint_time_minutes: None,
                image_subject: "Test".to_string(),
            },
        ];
        assert!(TemporalChallenge::validate_waypoint_sequences(&valid_waypoints).is_ok());

        // Invalid sequence (gap)
        let invalid_waypoints = vec![
            CreateWaypointRequest {
                waypoint_sequence: 1,
                location: GeoLocation { lat: 1.0, lon: 1.0 },
                radius_meters: 50.0,
                waypoint_clue: "Test".to_string(),
                hints: vec![],
                waypoint_time_minutes: None,
                image_subject: "Test".to_string(),
            },
            CreateWaypointRequest {
                waypoint_sequence: 3, // Should be 2
                location: GeoLocation { lat: 2.0, lon: 2.0 },
                radius_meters: 50.0,
                waypoint_clue: "Test".to_string(),
                hints: vec![],
                waypoint_time_minutes: None,
                image_subject: "Test".to_string(),
            },
        ];
        assert!(TemporalChallenge::validate_waypoint_sequences(&invalid_waypoints).is_err());

        // Empty waypoints
        assert!(TemporalChallenge::validate_waypoint_sequences(&[]).is_err());
    }

    #[test]
    fn test_temporal_challenge_json_serialization() {
        use crate::services::location_service::GeoLocation;

        let waypoint_data = WaypointData {
            waypoint_id: None,
            waypoint_sequence: 1,
            location: GeoLocation {
                lat: 51.5074,
                lon: -0.1278,
            },
            radius_meters: 50.0,
            waypoint_clue: "Test clue".to_string(),
            hints: vec!["Hint 1".to_string()],
            waypoint_time_minutes: Some(15),
            image_subject: "Test subject".to_string(),
            created_at: Some(Utc::now()),
        };

        let challenge_data = ChallengeData {
            challenge_id: 1,
            challenge_description: Some("Test description".to_string()),
            challenge_moderator: 1,
            actual_start_time: None,
            duration_minutes: 120,
            challenge_type: ChallengeType::Com,
            active: true,
            waypoints: vec![waypoint_data],
            metadata: ChallengeMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                migrated_from_relational: None,
                version_notes: Some("Test version".to_string()),
            },
        };

        // Test serialization and deserialization
        let json = serde_json::to_value(&challenge_data).unwrap();
        let deserialized: ChallengeData = serde_json::from_value(json).unwrap();

        assert_eq!(challenge_data.challenge_id, deserialized.challenge_id);
        assert_eq!(challenge_data.challenge_type, deserialized.challenge_type);
        assert_eq!(challenge_data.waypoints.len(), deserialized.waypoints.len());
        assert_eq!(
            challenge_data.waypoints[0].waypoint_sequence,
            deserialized.waypoints[0].waypoint_sequence
        );
    }

    #[test]
    fn test_challenge_metadata_serialization() {
        let metadata = ChallengeMetadata {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            migrated_from_relational: Some(true),
            version_notes: Some("Migration from relational schema".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ChallengeMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(
            metadata.migrated_from_relational,
            deserialized.migrated_from_relational
        );
        assert_eq!(metadata.version_notes, deserialized.version_notes);
    }

    #[test]
    fn test_waypoint_data_serialization() {
        use crate::services::location_service::GeoLocation;

        let waypoint = WaypointData {
            waypoint_id: Some(42),
            waypoint_sequence: 1,
            location: GeoLocation {
                lat: 51.5074,
                lon: -0.1278,
            },
            radius_meters: 50.0,
            waypoint_clue: "Find the red box".to_string(),
            hints: vec!["Look for red".to_string(), "Used for posting".to_string()],
            waypoint_time_minutes: Some(15),
            image_subject: "Red post box".to_string(),
            created_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&waypoint).unwrap();
        let deserialized: WaypointData = serde_json::from_str(&json).unwrap();

        assert_eq!(waypoint.waypoint_id, deserialized.waypoint_id);
        assert_eq!(waypoint.waypoint_sequence, deserialized.waypoint_sequence);
        assert_eq!(waypoint.location.lat, deserialized.location.lat);
        assert_eq!(waypoint.hints.len(), deserialized.hints.len());
    }
}
