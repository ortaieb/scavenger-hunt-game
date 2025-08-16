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

impl ToString for ChallengeType {
    fn to_string(&self) -> String {
        match self {
            ChallengeType::Rec => "REC".to_string(),
            ChallengeType::Com => "COM".to_string(),
            ChallengeType::Res => "RES".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "waypoint_state", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaypointState {
    Presented,
    CheckedIn,
    Verified,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Challenge {
    pub challenge_id: Uuid,                       // UUID PRIMARY KEY - never null
    pub challenge_name: String,                   // NOT NULL
    pub challenge_description: Option<String>,    // Can be null
    pub challenge_moderator: i32,                 // NOT NULL FK to users
    pub planned_start_time: DateTime<Utc>,        // NOT NULL
    pub actual_start_time: Option<DateTime<Utc>>, // Can be null
    pub duration_minutes: i32,                    // NOT NULL
    pub challenge_type: ChallengeType,            // NOT NULL with default
    pub active: bool,                             // DEFAULT true - never null
    pub created_at: DateTime<Utc>,                // DEFAULT NOW() - never null
    pub updated_at: DateTime<Utc>,                // DEFAULT NOW() - never null
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Waypoint {
    pub waypoint_id: i32,                   // SERIAL PRIMARY KEY - never null
    pub challenge_id: Uuid,                 // NOT NULL FK
    pub waypoint_sequence: i32,             // NOT NULL
    pub location_lat: f64,                  // NOT NULL
    pub location_lon: f64,                  // NOT NULL
    pub radius_meters: f64,                 // NOT NULL with default 50.0
    pub waypoint_clue: String,              // NOT NULL
    pub hints: Option<Vec<String>>,         // Can be null (TEXT[])
    pub waypoint_time_minutes: Option<i32>, // DEFAULT -1, but can be null
    pub image_subject: String,              // NOT NULL
    pub created_at: DateTime<Utc>,          // DEFAULT NOW() - never null
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChallengeParticipant {
    pub participant_id: Uuid,                 // UUID PRIMARY KEY - never null
    pub challenge_id: Uuid,                   // NOT NULL FK
    pub user_id: i32,                         // NOT NULL FK to users
    pub participant_nickname: Option<String>, // Can be null
    pub current_waypoint_id: Option<i32>,     // Can be null FK to waypoints
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
    pub challenge_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartChallengeResponse {
    #[serde(rename = "challenge-id")]
    pub challenge_id: Uuid,
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
    pub challenge: Challenge,
    pub waypoints: Vec<Waypoint>,
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

impl Challenge {
    pub async fn create(
        pool: &PgPool,
        moderator_id: i32,
        request: CreateChallengeRequest,
    ) -> Result<Challenge, ChallengeError> {
        // Validate waypoint sequences
        Self::validate_waypoint_sequences(&request.waypoints)?;

        let mut tx = pool.begin().await?;

        // Create challenge
        let challenge = sqlx::query_as!(
            Challenge,
            r#"
            INSERT INTO challenges (challenge_name, challenge_description, challenge_moderator, 
                                  planned_start_time, duration_minutes, challenge_type)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING challenge_id, challenge_name, challenge_description, 
                     challenge_moderator as "challenge_moderator!",
                     planned_start_time, actual_start_time, duration_minutes, 
                     challenge_type as "challenge_type: ChallengeType", 
                     COALESCE(active, true) as "active!",
                     COALESCE(created_at, NOW()) as "created_at!",
                     COALESCE(updated_at, NOW()) as "updated_at!"
            "#,
            request.challenge_name,
            request.challenge_description,
            moderator_id,
            request.planned_start_time,
            request.duration_minutes,
            request.challenge_type as ChallengeType
        )
        .fetch_one(&mut *tx)
        .await?;

        // Create waypoints
        for waypoint_req in request.waypoints {
            sqlx::query!(
                r#"
                INSERT INTO waypoints (challenge_id, waypoint_sequence, location_lat, location_lon,
                                     radius_meters, waypoint_clue, hints, waypoint_time_minutes, image_subject)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                challenge.challenge_id,
                waypoint_req.waypoint_sequence,
                waypoint_req.location.lat,
                waypoint_req.location.lon,
                waypoint_req.radius_meters,
                waypoint_req.waypoint_clue,
                &waypoint_req.hints,
                waypoint_req.waypoint_time_minutes.unwrap_or(-1),
                waypoint_req.image_subject
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(challenge)
    }

    pub async fn get_by_id(pool: &PgPool, challenge_id: Uuid) -> Result<Challenge, ChallengeError> {
        let challenge = sqlx::query_as!(
            Challenge,
            r#"
            SELECT challenge_id, challenge_name, challenge_description, 
                   challenge_moderator as "challenge_moderator!",
                   planned_start_time, actual_start_time, duration_minutes,
                   challenge_type as "challenge_type: ChallengeType", 
                   COALESCE(active, true) as "active!",
                   COALESCE(created_at, NOW()) as "created_at!",
                   COALESCE(updated_at, NOW()) as "updated_at!"
            FROM challenges
            WHERE challenge_id = $1
            "#,
            challenge_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ChallengeError::ChallengeNotFound)?;

        Ok(challenge)
    }

    pub async fn start_challenge(
        &mut self,
        pool: &PgPool,
        moderator_id: i32,
    ) -> Result<StartChallengeResponse, ChallengeError> {
        // Check if user is moderator
        if self.challenge_moderator != moderator_id {
            return Err(ChallengeError::NotModerator);
        }

        // Check if challenge is active
        if !self.active {
            return Err(ChallengeError::ChallengeNotActive);
        }

        // Check if already started
        if self.actual_start_time.is_some() {
            return Err(ChallengeError::ChallengeAlreadyStarted);
        }

        let mut tx = pool.begin().await?;

        // Update challenge with actual start time
        let now = Utc::now();
        sqlx::query!(
            "UPDATE challenges SET actual_start_time = $1, updated_at = $1 WHERE challenge_id = $2",
            now,
            self.challenge_id
        )
        .execute(&mut *tx)
        .await?;

        self.actual_start_time = Some(now);

        // Get all participants
        let participants = sqlx::query!(
            r#"
            SELECT cp.participant_id as "participant_id!", cp.user_id as "user_id!"
            FROM challenge_participants cp
            WHERE cp.challenge_id = $1
            "#,
            self.challenge_id
        )
        .fetch_all(&mut *tx)
        .await?;

        // Set participants to first waypoint if they exist
        if let Some(first_waypoint) = self.get_first_waypoint(pool).await? {
            for participant in &participants {
                sqlx::query!(
                    r#"
                    UPDATE challenge_participants 
                    SET current_waypoint_id = $1, current_state = 'PRESENTED', last_updated = $2
                    WHERE participant_id = $3
                    "#,
                    first_waypoint.waypoint_id,
                    now,
                    participant.participant_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(StartChallengeResponse {
            challenge_id: self.challenge_id,
            planned_start_time: self.planned_start_time,
            actual_start_time: now,
            duration: self.duration_minutes,
            participants: participants
                .into_iter()
                .map(|p| ParticipantInfo {
                    user_id: p.user_id,
                    participant_id: p.participant_id,
                })
                .collect(),
        })
    }

    pub async fn get_waypoints(&self, pool: &PgPool) -> Result<Vec<Waypoint>, ChallengeError> {
        let waypoints = sqlx::query_as!(
            Waypoint,
            r#"
            SELECT waypoint_id as "waypoint_id!", challenge_id as "challenge_id!", waypoint_sequence as "waypoint_sequence!", 
                   location_lat as "location_lat!", location_lon as "location_lon!",
                   radius_meters as "radius_meters!", waypoint_clue as "waypoint_clue!", hints, 
                   waypoint_time_minutes, image_subject as "image_subject!", 
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM waypoints
            WHERE challenge_id = $1
            ORDER BY waypoint_sequence
            "#,
            self.challenge_id
        )
        .fetch_all(pool)
        .await?;

        Ok(waypoints)
    }

    pub async fn get_first_waypoint(
        &self,
        pool: &PgPool,
    ) -> Result<Option<Waypoint>, ChallengeError> {
        let waypoint = sqlx::query_as!(
            Waypoint,
            r#"
            SELECT waypoint_id as "waypoint_id!", challenge_id as "challenge_id!", waypoint_sequence as "waypoint_sequence!", 
                   location_lat as "location_lat!", location_lon as "location_lon!",
                   radius_meters as "radius_meters!", waypoint_clue as "waypoint_clue!", hints, 
                   waypoint_time_minutes, image_subject as "image_subject!", 
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM waypoints
            WHERE challenge_id = $1
            ORDER BY waypoint_sequence
            LIMIT 1
            "#,
            self.challenge_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(waypoint)
    }

    pub async fn get_participants(
        &self,
        pool: &PgPool,
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
            self.challenge_id
        )
        .fetch_all(pool)
        .await?;

        Ok(participants)
    }

    pub async fn invite_participant(
        &self,
        pool: &PgPool,
        user_id: i32,
        nickname: Option<String>,
    ) -> Result<ChallengeParticipant, ChallengeError> {
        // Check if user is already a participant
        let existing = sqlx::query!(
            "SELECT participant_id FROM challenge_participants WHERE challenge_id = $1 AND user_id = $2",
            self.challenge_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
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
            self.challenge_id,
            user_id,
            nickname
        )
        .fetch_one(pool)
        .await?;

        Ok(participant)
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

    pub fn is_ended(&self) -> bool {
        if let Some(start_time) = self.actual_start_time {
            let end_time = start_time + chrono::Duration::minutes(self.duration_minutes as i64);
            Utc::now() > end_time
        } else {
            false
        }
    }

    pub fn get_end_time(&self) -> Option<DateTime<Utc>> {
        self.actual_start_time
            .map(|start| start + chrono::Duration::minutes(self.duration_minutes as i64))
    }
}

impl Waypoint {
    pub async fn get_by_id(pool: &PgPool, waypoint_id: i32) -> Result<Waypoint, ChallengeError> {
        let waypoint = sqlx::query_as!(
            Waypoint,
            r#"
            SELECT waypoint_id as "waypoint_id!", challenge_id as "challenge_id!", waypoint_sequence as "waypoint_sequence!", 
                   location_lat as "location_lat!", location_lon as "location_lon!",
                   radius_meters as "radius_meters!", waypoint_clue as "waypoint_clue!", hints, 
                   waypoint_time_minutes, image_subject as "image_subject!", 
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM waypoints
            WHERE waypoint_id = $1
            "#,
            waypoint_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ChallengeError::WaypointNotFound)?;

        Ok(waypoint)
    }

    pub fn get_location(&self) -> GeoLocation {
        GeoLocation {
            lat: self.location_lat,
            lon: self.location_lon,
        }
    }

    pub async fn get_next_waypoint(
        &self,
        pool: &PgPool,
    ) -> Result<Option<Waypoint>, ChallengeError> {
        let next_waypoint = sqlx::query_as!(
            Waypoint,
            r#"
            SELECT waypoint_id as "waypoint_id!", challenge_id as "challenge_id!", waypoint_sequence as "waypoint_sequence!", 
                   location_lat as "location_lat!", location_lon as "location_lon!",
                   radius_meters as "radius_meters!", waypoint_clue as "waypoint_clue!", hints, 
                   waypoint_time_minutes, image_subject as "image_subject!", 
                   COALESCE(created_at, NOW()) as "created_at!"
            FROM waypoints
            WHERE challenge_id = $1 AND waypoint_sequence = $2
            "#,
            self.challenge_id,
            self.waypoint_sequence + 1
        )
        .fetch_optional(pool)
        .await?;

        Ok(next_waypoint)
    }
}

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

    pub async fn update_waypoint_state(
        &mut self,
        pool: &PgPool,
        waypoint_id: i32,
        state: WaypointState,
    ) -> Result<(), ChallengeError> {
        let now = Utc::now();

        sqlx::query!(
            r#"
            UPDATE challenge_participants
            SET current_waypoint_id = $1, current_state = $2, last_updated = $3
            WHERE participant_id = $4
            "#,
            waypoint_id,
            state as WaypointState,
            now,
            self.participant_id
        )
        .execute(pool)
        .await?;

        self.current_waypoint_id = Some(waypoint_id);
        self.current_state = state;
        self.last_updated = now;

        Ok(())
    }

    pub async fn advance_to_next_waypoint(
        &mut self,
        pool: &PgPool,
    ) -> Result<Option<Waypoint>, ChallengeError> {
        if let Some(current_waypoint_id) = self.current_waypoint_id {
            let current_waypoint = Waypoint::get_by_id(pool, current_waypoint_id).await?;
            let next_waypoint = current_waypoint.get_next_waypoint(pool).await?;

            if let Some(next) = &next_waypoint {
                self.update_waypoint_state(pool, next.waypoint_id, WaypointState::Presented)
                    .await?;
            }

            Ok(next_waypoint)
        } else {
            Ok(None)
        }
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
        assert!(Challenge::validate_waypoint_sequences(&valid_waypoints).is_ok());

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
        assert!(Challenge::validate_waypoint_sequences(&invalid_waypoints).is_err());

        // Empty waypoints
        assert!(Challenge::validate_waypoint_sequences(&[]).is_err());
    }
}
