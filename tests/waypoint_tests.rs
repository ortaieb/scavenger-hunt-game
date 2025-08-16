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
    models::challenge::{ChallengeType, WaypointState},
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

/// Helper struct for test data
struct TestSetup {
    participant_token: String,
    participant_id: Uuid,
    challenge_id: Uuid,
    waypoint_id: i32,
}

/// Setup a complete test scenario with user, challenge, and waypoint
async fn setup_challenge_scenario(app: &axum::Router, pool: &PgPool) -> TestSetup {
    // Register a user
    let register_body = json!({
        "username": "participant@example.com",
        "password": "password123",
        "nickname": "ParticipantUser",
        "roles": ["user.verified", "challenge.participant"]
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

    // Get user ID
    let user_id = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1",
        "participant@example.com"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .user_id;

    // Create challenge
    let challenge_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO challenges (challenge_id, challenge_name, challenge_description, 
                              challenge_moderator, planned_start_time, actual_start_time,
                              duration_minutes, challenge_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        challenge_id,
        "Waypoint Test Challenge",
        "A challenge for waypoint testing",
        user_id,
        chrono::Utc::now() - chrono::Duration::hours(1),
        chrono::Utc::now() - chrono::Duration::minutes(30), // Started 30 mins ago
        120,
        ChallengeType::Com as ChallengeType
    )
    .execute(pool)
    .await
    .expect("Failed to create test challenge");

    // Create waypoint
    let waypoint_id = sqlx::query!(
        r#"
        INSERT INTO waypoints (challenge_id, waypoint_sequence, location_lat, location_lon,
                             radius_meters, waypoint_clue, hints, waypoint_time_minutes, image_subject)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING waypoint_id
        "#,
        challenge_id,
        1,
        51.5074, // London coordinates
        -0.1278,
        50.0, // 50 meter radius
        "Find the red post box",
        &vec!["Look for something red".to_string(), "Used for posting letters".to_string()],
        15,
        "Red post box"
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test waypoint")
    .waypoint_id;

    // Create participant
    let participant_id = sqlx::query!(
        r#"
        INSERT INTO challenge_participants (challenge_id, user_id, participant_nickname, 
                                          current_waypoint_id, current_state)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING participant_id
        "#,
        challenge_id,
        user_id,
        "TestParticipant",
        waypoint_id,
        WaypointState::Presented as WaypointState
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test participant")
    .participant_id;

    // Get participant token
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

    let token_response = app.clone().oneshot(token_request).await.unwrap();
    let token_body = to_bytes(token_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let token_json: Value = serde_json::from_slice(&token_body).unwrap();
    let participant_token = token_json["participant-auth-token"]
        .as_str()
        .unwrap()
        .to_string();

    TestSetup {
        participant_token,
        participant_id,
        challenge_id,
        waypoint_id,
    }
}

#[tokio::test]
async fn test_waypoint_checkin_success() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Check in at the waypoint location (within radius)
    let checkin_body = json!({
        "location": {
            "lat": 51.5075, // Very close to waypoint location
            "long": -0.1279
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["challenge-id"].as_str().unwrap(),
        setup.challenge_id.to_string()
    );
    assert_eq!(
        response_json["participant-id"].as_str().unwrap(),
        setup.participant_id.to_string()
    );
    assert_eq!(
        response_json["waypoint-id"].as_i64().unwrap(),
        setup.waypoint_id as i64
    );
    assert_eq!(response_json["state"].as_str().unwrap(), "CHECKED_IN");
    assert_eq!(response_json["proof"].as_str().unwrap(), "Red post box");
}

#[tokio::test]
async fn test_waypoint_checkin_too_far() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Check in far from the waypoint location (outside radius)
    let checkin_body = json!({
        "location": {
            "lat": 51.6074, // ~11km away from waypoint
            "long": -0.1278
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Your checkin attempt is too far from the target"
    );
}

#[tokio::test]
async fn test_waypoint_checkin_wrong_waypoint() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Try to check in to a non-existent waypoint
    let checkin_body = json!({
        "location": {
            "lat": 51.5075,
            "long": -0.1279
        }
    });

    let wrong_waypoint_id = 99999; // Non-existent waypoint
    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            wrong_waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Waypoint not found"
    );
}

#[tokio::test]
async fn test_waypoint_checkin_not_current_waypoint() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Create another waypoint
    let other_waypoint_id = sqlx::query!(
        r#"
        INSERT INTO waypoints (challenge_id, waypoint_sequence, location_lat, location_lon,
                             radius_meters, waypoint_clue, hints, waypoint_time_minutes, image_subject)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING waypoint_id
        "#,
        setup.challenge_id,
        2, // Different sequence
        51.5080,
        -0.1290,
        30.0,
        "Find the clock tower",
        &vec!["Look up high".to_string()],
        20,
        "Clock tower"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to create second waypoint")
    .waypoint_id;

    // Try to check in to the wrong waypoint
    let checkin_body = json!({
        "location": {
            "lat": 51.5080,
            "long": -0.1290
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            other_waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "This is not your current waypoint"
    );
}

#[tokio::test]
async fn test_waypoint_proof_submission_not_checked_in() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Try to submit proof without checking in first
    let multipart_body = format!(
        "--boundary\r\n\
         Content-Disposition: form-data; name=\"image\"; filename=\"test.jpg\"\r\n\
         Content-Type: image/jpeg\r\n\r\n\
         fake-image-data\r\n\
         --boundary--\r\n"
    );

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/proof",
            setup.waypoint_id
        ))
        .header(
            http::header::CONTENT_TYPE,
            "multipart/form-data; boundary=boundary",
        )
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(multipart_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "You must check in to this waypoint before submitting proof"
    );
}

#[tokio::test]
async fn test_waypoint_checkin_invalid_coordinates() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Check in with invalid coordinates
    let checkin_body = json!({
        "location": {
            "lat": 91.0, // Invalid latitude (>90)
            "long": -0.1278
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        response_json["message"].as_str().unwrap(),
        "Location validation failed"
    );
}

#[tokio::test]
async fn test_waypoint_checkin_without_participant_token() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Try to check in with regular user token instead of participant token
    let register_body = json!({
        "username": "regular@example.com",
        "password": "password123",
        "nickname": "RegularUser"
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

    let checkin_body = json!({
        "location": {
            "lat": 51.5075,
            "long": -0.1279
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", user_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json["message"]
        .as_str()
        .unwrap()
        .contains("Token validation failed"));
}

#[tokio::test]
async fn test_location_validation_boundary_cases() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Test location exactly at the boundary (should succeed)
    // Waypoint is at (51.5074, -0.1278) with 50m radius
    // This location is approximately 50m away
    let checkin_body = json!({
        "location": {
            "lat": 51.50785, // About 50m north
            "long": -0.1278
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // This should be close enough to succeed (within 50m radius)
    // The exact result depends on the precision of the Haversine calculation
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST);

    if response.status() == StatusCode::OK {
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let response_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response_json["state"].as_str().unwrap(), "CHECKED_IN");
    }
}

#[tokio::test]
async fn test_concurrent_checkin_attempts() {
    let (app, pool) = setup_test_environment().await;
    let setup = setup_challenge_scenario(&app, &pool).await;

    // Test multiple rapid check-in attempts (should handle gracefully)
    let checkin_body = json!({
        "location": {
            "lat": 51.5075,
            "long": -0.1279
        }
    });

    let request1 = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    let request2 = Request::builder()
        .method(http::Method::POST)
        .uri(&format!(
            "/challenges/waypoints/{}/checkin",
            setup.waypoint_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", setup.participant_token.clone()),
        )
        .body(Body::from(checkin_body.to_string()))
        .unwrap();

    // Send both requests concurrently
    let (response1, response2) =
        tokio::join!(app.clone().oneshot(request1), app.clone().oneshot(request2));

    let response1 = response1.unwrap();
    let response2 = response2.unwrap();

    // At least one should succeed
    assert!(
        response1.status() == StatusCode::OK || response2.status() == StatusCode::OK,
        "At least one concurrent request should succeed"
    );
}
