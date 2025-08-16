use axum::{
    body::{Body, to_bytes},
    http::{self, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use scavenger_hunt_game_server::{
    auth::{AuthState, JwtService},
    config::Config,
    create_api_router, create_connection_pool, run_migrations,
    models::challenge::ChallengeType,
    routes::AppState,
    services::{AuthService, ImageService, LocationService},
};

/// Helper function to setup test environment
async fn setup_test_environment() -> (axum::Router, PgPool) {
    // Load test configuration
    std::env::set_var(
        "DATABASE_URL",
        "postgresql://test:test@localhost/scavenger_test",
    );
    std::env::set_var(
        "JWT_SECRET",
        "test-secret-key-that-is-long-enough-for-testing-32chars",
    );
    std::env::set_var("IMAGE_CHECKER_URL", "http://localhost:8080");
    std::env::set_var("IMAGE_BASE_DIR", "/tmp/test-images");

    let config = Config::from_env().expect("Failed to load test config");

    // Create database pool
    let pool = create_connection_pool(&config)
        .await
        .expect("Failed to create test database pool");

    // Run migrations
    run_migrations(&pool)
        .await
        .expect("Failed to run test migrations");

    // Initialize services
    let jwt_service = Arc::new(JwtService::new(&config.jwt_secret));
    let auth_service = Arc::new(AuthService::new(jwt_service.clone(), pool.clone()));
    let location_service = Arc::new(LocationService::new(pool.clone()));
    let image_service = Arc::new(ImageService::new(
        config.image_checker_url,
        config.image_base_dir,
    ));

    // Create auth state
    let auth_state = AuthState { jwt_service };

    // Create router
    let app_state = AppState {
        pool: pool.clone(),
        auth_service,
        location_service,
        image_service,
        auth_state,
    };
    let app = create_api_router(app_state);

    (app, pool)
}

/// Helper function to create a test user
async fn create_test_user(pool: &PgPool) -> i32 {
    sqlx::query!(
        r#"
        INSERT INTO users (username, password, nickname)
        VALUES ($1, $2, $3)
        RETURNING user_id
        "#,
        "test@example.com",
        "$argon2id$v=19$m=65536,t=3,p=4$random_salt$hash", // Mock hash
        "TestUser"
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
    .user_id
}

/// Helper function to create a test challenge
async fn create_test_challenge(pool: &PgPool, moderator_id: i32) -> Uuid {
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        challenge_id,
        "Test Challenge",
        "A test challenge",
        moderator_id,
        chrono::Utc::now() + chrono::Duration::hours(1),
        120,
        ChallengeType::Com as ChallengeType
    )
    .execute(pool)
    .await
    .expect("Failed to create test challenge");

    challenge_id
}

/// Helper function to invite user to challenge
async fn invite_user_to_challenge(pool: &PgPool, challenge_id: Uuid, user_id: i32) {
    sqlx::query!(
        r#"
        INSERT INTO challenge_participants (challenge_id, user_id, participant_nickname)
        VALUES ($1, $2, $3)
        "#,
        challenge_id,
        user_id,
        "TestParticipant"
    )
    .execute(pool)
    .await
    .expect("Failed to invite user to challenge");
}

#[tokio::test]
async fn test_user_registration_success() {
    let (app, _pool) = setup_test_environment().await;

    let request_body = json!({
        "username": "newuser@example.com",
        "password": "password123",
        "nickname": "NewUser",
        "roles": ["user.verified", "challenge.participant"]
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["user-auth-token"].is_string());
    assert_eq!(response_json["expires_in"].as_i64().unwrap(), 7200);
    assert_eq!(response_json["token_type"].as_str().unwrap(), "Bearer");
}

#[tokio::test]
async fn test_user_registration_duplicate_email() {
    let (app, pool) = setup_test_environment().await;

    // Create a test user first
    create_test_user(&pool).await;

    let request_body = json!({
        "username": "test@example.com", // Same email as test user
        "password": "password123",
        "nickname": "DuplicateUser"
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Username already exists"
    );
}

#[tokio::test]
async fn test_user_registration_invalid_email() {
    let (app, _pool) = setup_test_environment().await;

    let request_body = json!({
        "username": "invalid-email",
        "password": "password123",
        "nickname": "TestUser"
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Invalid username format"
    );
}

#[tokio::test]
async fn test_user_registration_weak_password() {
    let (app, _pool) = setup_test_environment().await;

    let request_body = json!({
        "username": "test@example.com",
        "password": "weak", // Too short
        "nickname": "TestUser"
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["message"]
        .as_str()
        .unwrap()
        .contains("8 characters"));
}

#[tokio::test]
async fn test_user_login_success() {
    let (app, pool) = setup_test_environment().await;

    // First register a user
    let register_body = json!({
        "username": "login@example.com",
        "password": "password123",
        "nickname": "LoginUser"
    });

    let register_request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(register_body.to_string()))
        .unwrap();

    let _register_response = app.clone().oneshot(register_request).await.unwrap();

    // Now test login
    let login_body = json!({
        "username": "login@example.com",
        "password": "password123"
    });

    let login_request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(login_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["user-auth-token"].is_string());
    assert_eq!(response_json["expires_in"].as_i64().unwrap(), 7200);
    assert_eq!(response_json["token_type"].as_str().unwrap(), "Bearer");
}

#[tokio::test]
async fn test_user_login_invalid_credentials() {
    let (app, _pool) = setup_test_environment().await;

    let login_body = json!({
        "username": "nonexistent@example.com",
        "password": "wrongpassword"
    });

    let login_request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(login_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Invalid username or password"
    );
}

#[tokio::test]
async fn test_participant_token_creation_success() {
    let (app, pool) = setup_test_environment().await;

    // Register a user
    let register_body = json!({
        "username": "participant@example.com",
        "password": "password123",
        "nickname": "ParticipantUser"
    });

    let register_request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(register_body.to_string()))
        .unwrap();

    let register_response = app.clone().oneshot(register_request).await.unwrap();
    let register_body = to_bytes(register_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let register_json: Value = serde_json::from_slice(&register_body).unwrap();
    let user_token = register_json["user-auth-token"].as_str().unwrap();

    // Get user ID and create challenge
    let user_id = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1",
        "participant@example.com"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .user_id;

    let challenge_id = create_test_challenge(&pool, user_id).await;
    invite_user_to_challenge(&pool, challenge_id, user_id).await;

    // Request participant token
    let token_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let token_request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenge/authentication")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", user_token),
        )
        .body(Body::from(token_body.to_string()))
        .unwrap();

    let response = app.oneshot(token_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["participant-auth-token"].is_string());
    assert!(response_json["expires_in"].as_i64().unwrap() > 0);
    assert_eq!(response_json["token_type"].as_str().unwrap(), "Bearer");
}

#[tokio::test]
async fn test_participant_token_user_not_invited() {
    let (app, pool) = setup_test_environment().await;

    // Register a user
    let register_body = json!({
        "username": "notinvited@example.com",
        "password": "password123",
        "nickname": "NotInvitedUser"
    });

    let register_request = Request::builder()
        .method(http::Method::POST)
        .uri("/authentication/register")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(register_body.to_string()))
        .unwrap();

    let register_response = app.clone().oneshot(register_request).await.unwrap();
    let register_body = to_bytes(register_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let register_json: Value = serde_json::from_slice(&register_body).unwrap();
    let user_token = register_json["user-auth-token"].as_str().unwrap();

    // Get user ID and create challenge (but don't invite user)
    let user_id = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1",
        "notinvited@example.com"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .user_id;

    let challenge_id = create_test_challenge(&pool, user_id).await;
    // Note: Not inviting user to challenge

    // Request participant token
    let token_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let token_request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenge/authentication")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", user_token),
        )
        .body(Body::from(token_body.to_string()))
        .unwrap();

    let response = app.oneshot(token_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "no participant attached to the challenge for this user"
    );
}

#[tokio::test]
async fn test_protected_endpoint_without_token() {
    let (app, _pool) = setup_test_environment().await;

    let challenge_id = Uuid::new_v4();
    let token_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let token_request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenge/authentication")
        .header(http::header::CONTENT_TYPE, "application/json")
        // No authorization header
        .body(Body::from(token_body.to_string()))
        .unwrap();

    let response = app.oneshot(token_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Missing authorization header"
    );
}

#[tokio::test]
async fn test_protected_endpoint_with_invalid_token() {
    let (app, _pool) = setup_test_environment().await;

    let challenge_id = Uuid::new_v4();
    let token_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let token_request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenge/authentication")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, "Bearer invalid-token")
        .body(Body::from(token_body.to_string()))
        .unwrap();

    let response = app.oneshot(token_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["message"]
        .as_str()
        .unwrap()
        .contains("Token validation failed"));
}
