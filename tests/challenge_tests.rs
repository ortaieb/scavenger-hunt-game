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

/// Helper function to register a user and get auth token
async fn register_user_and_get_token(
    app: &axum::Router,
    username: &str,
    roles: Vec<&str>,
) -> String {
    let register_body = json!({
        "username": username,
        "password": "password123",
        "nickname": "TestUser",
        "roles": roles
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

    register_json["user-auth-token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn test_create_challenge_success() {
    let (app, _pool) = setup_test_environment().await;

    // Register a user with challenge manager role
    let token = register_user_and_get_token(
        &app,
        "manager@example.com",
        vec!["challenge.manager", "user.verified"],
    )
    .await;

    let challenge_body = json!({
        "challenge_name": "Test Challenge",
        "challenge_description": "A test challenge for integration testing",
        "planned_start_time": "2025-12-01T10:00:00Z",
        "duration_minutes": 120,
        "challenge_type": "COM",
        "waypoints": [
            {
                "waypoint_sequence": 1,
                "location": {"lat": 51.5074, "long": -0.1278},
                "radius_meters": 50.0,
                "waypoint_clue": "Find the red post box near the main street",
                "hints": ["Look for a red cylindrical object", "It's used for posting letters"],
                "waypoint_time_minutes": 15,
                "image_subject": "Red post box"
            },
            {
                "waypoint_sequence": 2,
                "location": {"lat": 51.5080, "long": -0.1290},
                "radius_meters": 30.0,
                "waypoint_clue": "Find the historic clock tower",
                "hints": ["Look up high", "It shows the time"],
                "waypoint_time_minutes": 20,
                "image_subject": "Clock tower"
            }
        ]
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(challenge_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    // Verify challenge details
    assert_eq!(
        response_json["challenge"]["challenge_name"]
            .as_str()
            .unwrap(),
        "Test Challenge"
    );
    assert_eq!(
        response_json["challenge"]["challenge_type"]
            .as_str()
            .unwrap(),
        "COM"
    );
    assert_eq!(
        response_json["challenge"]["duration_minutes"]
            .as_i64()
            .unwrap(),
        120
    );

    // Verify waypoints
    let waypoints = response_json["waypoints"].as_array().unwrap();
    assert_eq!(waypoints.len(), 2);
    assert_eq!(waypoints[0]["waypoint_sequence"].as_i64().unwrap(), 1);
    assert_eq!(waypoints[1]["waypoint_sequence"].as_i64().unwrap(), 2);
    assert_eq!(
        waypoints[0]["waypoint_clue"].as_str().unwrap(),
        "Find the red post box near the main street"
    );
}

#[tokio::test]
async fn test_create_challenge_insufficient_permissions() {
    let (app, _pool) = setup_test_environment().await;

    // Register a user without challenge manager role
    let token = register_user_and_get_token(
        &app,
        "user@example.com",
        vec!["user.verified", "challenge.participant"],
    )
    .await;

    let challenge_body = json!({
        "challenge_name": "Unauthorized Challenge",
        "planned_start_time": "2025-12-01T10:00:00Z",
        "duration_minutes": 60,
        "challenge_type": "REC",
        "waypoints": [
            {
                "waypoint_sequence": 1,
                "location": {"lat": 51.5074, "long": -0.1278},
                "radius_meters": 50.0,
                "waypoint_clue": "Test clue",
                "hints": [],
                "image_subject": "Test subject"
            }
        ]
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(challenge_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Insufficient permissions to create challenges"
    );
}

#[tokio::test]
async fn test_create_challenge_invalid_waypoint_sequence() {
    let (app, _pool) = setup_test_environment().await;

    // Register a user with challenge manager role
    let token = register_user_and_get_token(
        &app,
        "manager2@example.com",
        vec!["challenge.manager", "user.verified"],
    )
    .await;

    let challenge_body = json!({
        "challenge_name": "Invalid Sequence Challenge",
        "planned_start_time": "2025-12-01T10:00:00Z",
        "duration_minutes": 60,
        "challenge_type": "REC",
        "waypoints": [
            {
                "waypoint_sequence": 1,
                "location": {"lat": 51.5074, "long": -0.1278},
                "radius_meters": 50.0,
                "waypoint_clue": "First waypoint",
                "hints": [],
                "image_subject": "Test subject"
            },
            {
                "waypoint_sequence": 3, // Missing sequence 2
                "location": {"lat": 51.5080, "long": -0.1290},
                "radius_meters": 30.0,
                "waypoint_clue": "Third waypoint",
                "hints": [],
                "image_subject": "Test subject"
            }
        ]
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(challenge_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["message"]
        .as_str()
        .unwrap()
        .contains("waypoint sequence"));
}

#[tokio::test]
async fn test_get_challenge_success() {
    let (app, pool) = setup_test_environment().await;

    // Create a challenge directly in the database
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        challenge_id,
        "Test Get Challenge",
        "A challenge for get testing",
        1, // Assume user ID 1 exists
        chrono::Utc::now() + chrono::Duration::hours(1),
        90,
        ChallengeType::Rec as ChallengeType
    )
    .execute(&pool)
    .await
    .expect("Failed to create test challenge");

    // Register a user and get token
    let token =
        register_user_and_get_token(&app, "getter@example.com", vec!["user.verified"]).await;

    let request = Request::builder()
        .method(http::Method::GET)
        .uri(&format!("/challenges/{}", challenge_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["challenge"]["challenge_name"]
            .as_str()
            .unwrap(),
        "Test Get Challenge"
    );
    assert_eq!(
        response_json["challenge"]["challenge_type"]
            .as_str()
            .unwrap(),
        "REC"
    );
}

#[tokio::test]
async fn test_get_challenge_not_found() {
    let (app, _pool) = setup_test_environment().await;

    // Register a user and get token
    let token =
        register_user_and_get_token(&app, "notfound@example.com", vec!["user.verified"]).await;

    let nonexistent_id = Uuid::new_v4();
    let request = Request::builder()
        .method(http::Method::GET)
        .uri(&format!("/challenges/{}", nonexistent_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Challenge not found"
    );
}

#[tokio::test]
async fn test_start_challenge_success() {
    let (app, pool) = setup_test_environment().await;

    // Register a moderator user
    let token = register_user_and_get_token(
        &app,
        "moderator@example.com",
        vec!["challenge.moderator", "user.verified"],
    )
    .await;

    // Get user ID
    let user_id = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1",
        "moderator@example.com"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .user_id;

    // Create a challenge
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        challenge_id,
        "Start Test Challenge",
        "A challenge for start testing",
        user_id,
        chrono::Utc::now() + chrono::Duration::hours(1),
        120,
        ChallengeType::Com as ChallengeType
    )
    .execute(&pool)
    .await
    .expect("Failed to create test challenge");

    // Create waypoints
    sqlx::query!(
        r#"
        INSERT INTO waypoints (challenge_id, waypoint_sequence, location_lat, location_lon,
                             radius_meters, waypoint_clue, hints, waypoint_time_minutes, image_subject)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        challenge_id,
        1,
        51.5074,
        -0.1278,
        50.0,
        "First waypoint",
        &vec!["Hint 1".to_string()],
        15,
        "Test subject"
    )
    .execute(&pool)
    .await
    .expect("Failed to create test waypoint");

    // Add a participant
    sqlx::query!(
        r#"
        INSERT INTO challenge_participants (challenge_id, user_id, participant_nickname)
        VALUES ($1, $2, $3)
        "#,
        challenge_id,
        user_id,
        "TestParticipant"
    )
    .execute(&pool)
    .await
    .expect("Failed to add participant");

    // Start the challenge
    let start_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges/start")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(start_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["challenge-id"].as_str().unwrap(),
        challenge_id.to_string()
    );
    assert!(response_json["actual-start-time"].is_string());
    assert_eq!(response_json["duration"].as_i64().unwrap(), 120);
    assert_eq!(response_json["participants"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_start_challenge_not_moderator() {
    let (app, pool) = setup_test_environment().await;

    // Register a regular user (not moderator)
    let token = register_user_and_get_token(
        &app,
        "regular@example.com",
        vec!["user.verified", "challenge.participant"],
    )
    .await;

    // Create a challenge with a different moderator
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        challenge_id,
        "Not Moderator Challenge",
        "A challenge for testing unauthorized start",
        999, // Different moderator
        chrono::Utc::now() + chrono::Duration::hours(1),
        120,
        ChallengeType::Com as ChallengeType
    )
    .execute(&pool)
    .await
    .expect("Failed to create test challenge");

    // Try to start the challenge
    let start_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges/start")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(start_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "You are not the moderator of this challenge"
    );
}

#[tokio::test]
async fn test_start_challenge_already_started() {
    let (app, pool) = setup_test_environment().await;

    // Register a moderator user
    let token = register_user_and_get_token(
        &app,
        "moderator2@example.com",
        vec!["challenge.moderator", "user.verified"],
    )
    .await;

    // Get user ID
    let user_id = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1",
        "moderator2@example.com"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .user_id;

    // Create a challenge that's already started
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, actual_start_time, 
                              duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        challenge_id,
        "Already Started Challenge",
        "A challenge that's already started",
        user_id,
        chrono::Utc::now() - chrono::Duration::hours(1),
        chrono::Utc::now() - chrono::Duration::minutes(30), // Already started
        120,
        ChallengeType::Com as ChallengeType
    )
    .execute(&pool)
    .await
    .expect("Failed to create test challenge");

    // Try to start the challenge again
    let start_body = json!({
        "challenge-id": challenge_id.to_string()
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri("/challenges/start")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(start_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Challenge has already been started"
    );
}
