use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    GameAdmin,
    ChallengeManager,
    ChallengeModerator,
    ChallengeParticipant,
    ChallengeInvitee,
    UserVerified,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role_str = match self {
            UserRole::GameAdmin => "game.admin",
            UserRole::ChallengeManager => "challenge.manager",
            UserRole::ChallengeModerator => "challenge.moderator",
            UserRole::ChallengeParticipant => "challenge.participant",
            UserRole::ChallengeInvitee => "challenge.invitee",
            UserRole::UserVerified => "user.verified",
        };
        write!(f, "{role_str}")
    }
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.as_str() {
            "game.admin" => UserRole::GameAdmin,
            "challenge.manager" => UserRole::ChallengeManager,
            "challenge.moderator" => UserRole::ChallengeModerator,
            "challenge.participant" => UserRole::ChallengeParticipant,
            "challenge.invitee" => UserRole::ChallengeInvitee,
            "user.verified" => UserRole::UserVerified,
            _ => UserRole::UserVerified, // Default fallback
        }
    }
}

impl From<UserRole> for String {
    fn from(role: UserRole) -> Self {
        role.to_string()
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub user_id: i32, // SERIAL PRIMARY KEY - never null
    pub username: String,
    pub password: String,
    pub nickname: Option<String>,     // Can be null
    pub creation_date: DateTime<Utc>, // DEFAULT NOW() - never null
    pub updated_at: DateTime<Utc>,    // DEFAULT NOW() - never null
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub nickname: Option<String>,
    pub roles: Option<Vec<UserRole>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub roles: Vec<String>,
    pub creation_date: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    PasswordHashError(String),
    #[error("Password verification failed")]
    PasswordVerificationFailed,
    #[error("User not found")]
    UserNotFound,
    #[error("Username already exists")]
    UsernameAlreadyExists,
    #[error("Invalid username format")]
    InvalidUsername,
    #[error("Password too weak")]
    WeakPassword,
}

impl User {
    pub async fn create(
        pool: &PgPool,
        username: &str,
        password: &str,
        nickname: Option<&str>,
        roles: Option<Vec<UserRole>>,
    ) -> Result<User, UserError> {
        // Validate username (email format)
        if !Self::is_valid_email(username) {
            return Err(UserError::InvalidUsername);
        }

        // Validate password strength
        if password.len() < 8 {
            return Err(UserError::WeakPassword);
        }

        // Hash password
        let password_hash = Self::hash_password(password)?;

        // Start transaction
        let mut tx = pool.begin().await?;

        // Check if username already exists
        let existing_user = sqlx::query!("SELECT user_id FROM users WHERE username = $1", username)
            .fetch_optional(&mut *tx)
            .await?;

        if existing_user.is_some() {
            return Err(UserError::UsernameAlreadyExists);
        }

        // Create user
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, password, nickname)
            VALUES ($1, $2, $3)
            RETURNING user_id, username, password, nickname, 
                     COALESCE(creation_date, NOW()) as "creation_date!",
                     COALESCE(updated_at, NOW()) as "updated_at!"
            "#,
            username,
            password_hash,
            nickname
        )
        .fetch_one(&mut *tx)
        .await?;

        // Add default roles if none provided
        let user_roles =
            roles.unwrap_or_else(|| vec![UserRole::UserVerified, UserRole::ChallengeParticipant]);

        // Insert user roles
        for role in user_roles {
            sqlx::query!(
                "INSERT INTO user_roles (user_id, role_name) VALUES ($1, $2)",
                user.user_id,
                role.to_string()
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(user)
    }

    pub async fn authenticate(
        pool: &PgPool,
        username: &str,
        password: &str,
    ) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            r#"SELECT user_id, username, password, nickname, 
                     COALESCE(creation_date, NOW()) as "creation_date!",
                     COALESCE(updated_at, NOW()) as "updated_at!"
               FROM users WHERE username = $1"#,
            username
        )
        .fetch_optional(pool)
        .await?
        .ok_or(UserError::UserNotFound)?;

        // Verify password
        if Self::verify_password(password, &user.password)? {
            Ok(user)
        } else {
            Err(UserError::PasswordVerificationFailed)
        }
    }

    pub async fn get_user_roles(&self, pool: &PgPool) -> Result<Vec<UserRole>, UserError> {
        let roles = sqlx::query!(
            "SELECT role_name FROM user_roles WHERE user_id = $1",
            self.user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(roles
            .into_iter()
            .map(|row| UserRole::from(row.role_name))
            .collect())
    }

    pub async fn to_response(&self, pool: &PgPool) -> Result<UserResponse, UserError> {
        let roles = self.get_user_roles(pool).await?;
        let role_strings: Vec<String> = roles.into_iter().map(|r| r.to_string()).collect();

        Ok(UserResponse {
            user_id: self.user_id,
            username: self.username.clone(),
            nickname: self.nickname.clone(),
            roles: role_strings,
            creation_date: self.creation_date,
        })
    }

    fn hash_password(password: &str) -> Result<String, UserError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| UserError::PasswordHashError(e.to_string()))?
            .to_string()
            .pipe(Ok)
    }

    fn verify_password(password: &str, hash: &str) -> Result<bool, UserError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| UserError::PasswordHashError(e.to_string()))?;

        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn is_valid_email(email: &str) -> bool {
        // Simple email validation
        email.contains('@') && email.contains('.') && email.len() > 5
    }
}

// Helper trait for pipeline operations
trait Pipe<T> {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_conversion() {
        let role = UserRole::GameAdmin;
        let role_string: String = role.into();
        assert_eq!(role_string, "game.admin");

        let role_from_string = UserRole::from("challenge.moderator".to_string());
        assert_eq!(role_from_string, UserRole::ChallengeModerator);
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = User::hash_password(password).unwrap();

        // Hash should be different from password
        assert_ne!(hash, password);

        // Should be able to verify the password
        assert!(User::verify_password(password, &hash).unwrap());

        // Wrong password should fail
        assert!(!User::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_email_validation() {
        assert!(User::is_valid_email("test@example.com"));
        assert!(User::is_valid_email("user.name@domain.co.uk"));

        assert!(!User::is_valid_email("invalid_email"));
        assert!(!User::is_valid_email("@domain.com"));
        assert!(!User::is_valid_email("user@"));
        assert!(!User::is_valid_email("user"));
    }
}
