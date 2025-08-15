name: "Scavenger Hunt Game Server - Phase 1 Implementation"
description: |

## Purpose
Complete implementation of a Rust-based scavenger hunt game server with REST API endpoints, JWT authentication, database integration, and image validation capabilities for Phase 1.

## Core Principles
1. **Context is King**: Include ALL necessary documentation, examples, and caveats
2. **Validation Loops**: Provide executable tests/lints the AI can run and fix
3. **Information Dense**: Use keywords and patterns from the codebase
4. **Progressive Success**: Start simple, validate, then enhance
5. **Global rules**: Be sure to follow all rules in CLAUDE.md

---

## Goal
Build a complete Rust-based scavenger hunt game server that supports user authentication, challenge management, waypoint check-ins with geolocation validation, and image proof verification through external API integration. The server must support REST-based polling communication for Phase 1 implementation.

## Why
- **Business value**: Enables recreational and competitive scavenger hunt gameplay with real-time location tracking and proof validation
- **Integration**: Foundation for mobile app client integration and future real-time features
- **Problems solved**: Provides secure, scalable backend for location-based gaming with audit trails and competitive scoring

## What
A REST API server with the following user-visible behavior:
- User registration and JWT-based authentication 
- Challenge creation, management, and participant invitation
- Real-time waypoint check-ins with GPS validation
- Image-based proof submission with AI validation
- Role-based permissions (admin, moderator, participant)
- Audit logging and disaster recovery capabilities

### Success Criteria
- [ ] All REST endpoints defined in examples/ are functional
- [ ] JWT authentication working for both user and participant tokens
- [ ] Database integration with environment variable configuration
- [ ] Image validation through external image-checker service
- [ ] Geolocation validation within specified radius
- [ ] All tests pass and code follows Rust best practices
- [ ] Complete audit trail for all game events

## All Needed Context

### Documentation & References (list all context needed to implement the feature)
```yaml
# MUST READ - Include these in your context window
- url: https://docs.rs/axum/latest/axum/
  why: Primary web framework - routing, middleware, extractors

- url: https://docs.rs/sqlx/latest/sqlx/
  why: Database operations, migrations, connection pooling

- url: https://codevoweb.com/jwt-authentication-in-rust-using-axum-framework/
  why: Complete JWT authentication implementation example

- url: https://sheroz.com/pages/blog/rust-axum-rest-api-postgres-redis-jwt-docker.html
  why: Full stack Axum + PostgreSQL + JWT implementation

- url: https://github.com/ortaieb/image-checker
  why: External image validation service API specification

- file: /docs/gameplay.md
  why: Complete game mechanics and user roles definition

- file: /docs/model/challenge.md
  why: Data models, challenge structure, waypoint definitions

- file: /examples/events/authentication.md
  why: Authentication endpoints specification

- file: /examples/events/waypoint_check-in.md
  why: Geolocation validation and check-in logic

- file: /examples/events/waypoint_proof.md
  why: Image proof submission and validation workflow

- docfile: /docs/user-management.md
  why: User roles, permissions, and security requirements
```

### Current Codebase tree
```bash
/Users/ortaieb/dev/sandbox/scavenger-hunt/game-server/
├── CLAUDE.md                    # Project guidelines and conventions
├── TASKS.md                     # Task tracking
├── docs/                        # Project documentation
│   ├── gameplay.md             # Game mechanics and definitions
│   ├── model/
│   │   └── challenge.md        # Data model specifications
│   └── user-management.md      # User roles and security
├── examples/                    # REST API specifications
│   └── events/
│       ├── authentication.md
│       ├── start_challenge.md
│       ├── waypoint_check-in.md
│       ├── waypoint_proof.md
│       └── [other endpoints]
└── prp/                        # Planning documents
    ├── features/
    └── plans/
```

### Desired Codebase tree with files to be added and responsibility of file
```bash
/Users/ortaieb/dev/sandbox/scavenger-hunt/game-server/
├── Cargo.toml                   # Dependencies and project config
├── .env.example                 # Environment variables template
├── migrations/                  # Database schema migrations
│   └── 001_initial.sql
├── src/
│   ├── main.rs                 # Application entry point and server setup
│   ├── lib.rs                  # Library exports and common imports
│   ├── config.rs               # Environment variable configuration
│   ├── auth/                   # Authentication and authorization
│   │   ├── mod.rs
│   │   ├── jwt.rs              # JWT token handling
│   │   └── middleware.rs       # Auth middleware
│   ├── handlers/               # HTTP request handlers
│   │   ├── mod.rs
│   │   ├── auth.rs             # Login, register, participant tokens
│   │   ├── challenges.rs       # Challenge CRUD operations
│   │   ├── waypoints.rs        # Check-in and proof endpoints
│   │   └── health.rs           # Health check endpoint
│   ├── models/                 # Database models and business logic
│   │   ├── mod.rs
│   │   ├── user.rs             # User and role models
│   │   ├── challenge.rs        # Challenge and waypoint models
│   │   └── audit_log.rs        # Event logging models
│   ├── services/               # Business logic services
│   │   ├── mod.rs
│   │   ├── auth_service.rs     # Authentication business logic
│   │   ├── challenge_service.rs # Challenge management
│   │   ├── location_service.rs  # Geolocation validation
│   │   └── image_service.rs     # External image validation
│   ├── db/                     # Database operations
│   │   ├── mod.rs
│   │   └── connection.rs       # Database connection pool
│   ├── utils/                  # Utility functions
│   │   ├── mod.rs
│   │   ├── validation.rs       # Input validation helpers
│   │   └── responses.rs        # Standardized API responses
│   └── routes/                 # Route definitions
│       ├── mod.rs
│       └── api.rs              # API route configuration
├── tests/                      # Integration tests
│   ├── auth_tests.rs
│   ├── challenge_tests.rs
│   └── waypoint_tests.rs
└── [existing files...]
```

### Known Gotchas of our codebase & Library Quirks
```rust
// CRITICAL: Axum requires Tower services for middleware
// Example: Use tower::ServiceBuilder for middleware composition

// CRITICAL: SQLx requires compile-time query checking
// Example: Use sqlx::query_as! macro for type-safe queries
// Must set DATABASE_URL environment variable for sqlx-cli

// CRITICAL: JWT tokens need proper expiration handling
// User tokens: 2 hours, Participant tokens: challenge duration + 1 hour

// CRITICAL: Image paths must use full URI with configurable base directory
// Environment variable IMAGE_BASE_DIR for local/cloud storage flexibility

// CRITICAL: Geolocation validation uses Haversine formula
// Earth radius = 6371000 meters for distance calculations

// CRITICAL: External image-checker service is async
// Use /validate POST -> /status GET -> /results GET pattern

// CRITICAL: Database transactions required for challenge start
// Participant creation must be atomic operation

// CRITICAL: Role-based authorization uses custom extractors
// Must validate both user authentication and role permissions
```

## Implementation Blueprint

### Data models and structure

Create the core data models to ensure type safety and consistency.
```rust
// Database models using SQLx
// - User model with roles and authentication
// - Challenge model with waypoints and participants  
// - AuditLog model for event tracking
// - Location model for GPS coordinates

// API request/response models using Serde
// - Authentication requests and JWT responses
// - Challenge creation and management DTOs
// - Waypoint check-in and proof submission models
// - Error response standardization

// Service layer models for business logic
// - Challenge state management
// - Participant progress tracking
// - Image validation integration
```

### List of tasks to be completed to fulfill the PRP in the order they should be completed

```yaml
Task 1: Project Setup and Dependencies
CREATE Cargo.toml:
  - ADD axum framework with features ["macros", "tokio"]
  - ADD sqlx with features ["runtime-tokio-rustls", "postgres", "chrono", "uuid"]
  - ADD tokio with features ["full"]
  - ADD serde with features ["derive"]
  - ADD jsonwebtoken, argon2, chrono, uuid, dotenv
  - ADD reqwest for external API calls
  - ADD geo-types for location handling

Task 2: Environment Configuration
CREATE .env.example:
  - DEFINE database connection URL template
  - DEFINE JWT secret key template
  - DEFINE image service base URL and auth
  - DEFINE server host and port configuration

CREATE src/config.rs:
  - IMPLEMENT environment variable loading with dotenv
  - DEFINE configuration struct with validation
  - HANDLE missing environment variables gracefully

Task 3: Database Schema and Connection
CREATE migrations/001_initial.sql:
  - DEFINE users table with authentication fields
  - DEFINE user_roles table for role-based permissions
  - DEFINE challenges table with moderator and timing
  - DEFINE waypoints table linked to challenges
  - DEFINE challenge_participants table
  - DEFINE audit_log table for event tracking
  - DEFINE indexes for performance optimization

CREATE src/db/connection.rs:
  - IMPLEMENT SQLx connection pool setup
  - CONFIGURE connection limits and timeouts
  - HANDLE database connection errors

Task 4: Authentication System
CREATE src/auth/jwt.rs:
  - IMPLEMENT JWT token creation and validation
  - HANDLE user tokens (2-hour expiration)
  - HANDLE participant tokens (challenge duration)
  - INCLUDE proper claims structure

CREATE src/auth/middleware.rs:
  - IMPLEMENT authentication middleware
  - IMPLEMENT role-based authorization
  - EXTRACT user and participant information

CREATE src/models/user.rs:
  - DEFINE User struct with SQLx derives
  - DEFINE UserRole enum with database mapping
  - IMPLEMENT password hashing with argon2

Task 5: Core Services
CREATE src/services/auth_service.rs:
  - IMPLEMENT user registration and login
  - IMPLEMENT participant token generation
  - HANDLE authentication business logic
  - VALIDATE user permissions for challenges

CREATE src/services/location_service.rs:
  - IMPLEMENT Haversine distance calculation
  - VALIDATE GPS coordinates within radius
  - HANDLE location data validation

CREATE src/services/image_service.rs:
  - IMPLEMENT image-checker API integration
  - HANDLE async image validation workflow
  - MANAGE processing IDs and status checking
  - HANDLE validation results parsing

Task 6: Business Models
CREATE src/models/challenge.rs:
  - DEFINE Challenge struct with waypoints
  - DEFINE Waypoint struct with validation rules
  - DEFINE ChallengeParticipant with state tracking
  - IMPLEMENT challenge state management

CREATE src/models/audit_log.rs:
  - DEFINE AuditEvent enum for all game events
  - IMPLEMENT event logging with timestamps
  - HANDLE audit trail creation

Task 7: API Handlers
CREATE src/handlers/auth.rs:
  - IMPLEMENT POST /authentication/login
  - IMPLEMENT POST /authentication/register
  - IMPLEMENT POST /challenge/authentication
  - HANDLE authentication error responses

CREATE src/handlers/challenges.rs:
  - IMPLEMENT challenge CRUD operations
  - IMPLEMENT POST /challenges/start
  - HANDLE challenge state transitions
  - VALIDATE moderator permissions

CREATE src/handlers/waypoints.rs:
  - IMPLEMENT POST /challenges/waypoints/{id}/checkin
  - IMPLEMENT POST /challenges/waypoints/{id}/proof
  - INTEGRATE location and image validation
  - HANDLE waypoint state progression

Task 8: API Routes and Server Setup
CREATE src/routes/api.rs:
  - CONFIGURE all API route definitions
  - APPLY authentication middleware selectively
  - HANDLE CORS and common middleware

CREATE src/main.rs:
  - INITIALIZE database connection pool
  - CONFIGURE Axum server with routes
  - HANDLE graceful shutdown
  - START server with environment configuration

Task 9: Error Handling and Utilities
CREATE src/utils/responses.rs:
  - STANDARDIZE API response formats
  - IMPLEMENT error response consistency
  - HANDLE success response patterns

CREATE src/utils/validation.rs:
  - IMPLEMENT input validation helpers
  - VALIDATE email addresses and passwords
  - HANDLE GPS coordinate validation

Task 10: Integration Tests
CREATE tests/auth_tests.rs:
  - TEST user registration and login flows
  - TEST JWT token validation
  - TEST participant token generation

CREATE tests/challenge_tests.rs:
  - TEST challenge creation and management
  - TEST challenge start workflow
  - TEST participant invitation system

CREATE tests/waypoint_tests.rs:
  - TEST waypoint check-in validation
  - TEST image proof submission
  - TEST location validation logic
```

### Per task pseudocode as needed added to each task

```rust
// Task 4: Authentication System - JWT Implementation
impl JwtService {
    // PATTERN: Always include proper claims validation
    pub fn create_user_token(user: &User) -> Result<String, AuthError> {
        let claims = Claims {
            iss: "scavenger-hunt-game".to_string(),
            upn: user.username.clone(),
            groups: user.roles.clone(),
            exp: Utc::now() + Duration::hours(2), // CRITICAL: 2-hour window
        };
        
        // GOTCHA: Use proper JWT secret from environment
        let secret = config.jwt_secret.as_bytes();
        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
            .map_err(AuthError::from)
    }
    
    // PATTERN: Participant tokens include challenge context
    pub fn create_participant_token(user: &User, challenge: &Challenge, participant_id: Uuid) -> Result<String, AuthError> {
        let claims = ParticipantClaims {
            iss: "scavenger-hunt-challenge".to_string(),
            upn: participant_id.to_string(),
            groups: user.roles.clone(),
            clg: challenge.challenge_id,  // CRITICAL: Challenge ID claim
            usr: user.user_id,           // CRITICAL: User ID claim
            exp: challenge.end_time + Duration::hours(1), // CRITICAL: Challenge window + 1 hour
        };
        
        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
            .map_err(AuthError::from)
    }
}

// Task 5: Location Service - Geolocation Validation
impl LocationService {
    // PATTERN: Use Haversine formula for accurate distance calculation
    pub fn validate_location(target: &GeoLocation, current: &GeoLocation, radius_meters: f64) -> bool {
        const EARTH_RADIUS: f64 = 6371000.0; // CRITICAL: Earth radius in meters
        
        let lat1_rad = target.lat.to_radians();
        let lat2_rad = current.lat.to_radians();
        let delta_lat = (current.lat - target.lat).to_radians();
        let delta_lon = (current.lon - target.lon).to_radians();
        
        let a = (delta_lat / 2.0).sin().powi(2) +
                lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let distance = EARTH_RADIUS * c;
        
        distance <= radius_meters
    }
}

// Task 5: Image Service - External API Integration
impl ImageService {
    // PATTERN: Async workflow with proper error handling
    pub async fn validate_image(
        &self,
        image_path: &str,
        expected_content: &str,
        location: &GeoLocation,
        max_distance: f64,
    ) -> Result<ValidationResult, ImageError> {
        let processing_id = Uuid::new_v4().to_string();
        
        // CRITICAL: Submit validation request
        let request = ImageValidationRequest {
            processing_id: processing_id.clone(),
            image_path: format!("{}/{}", self.base_url, image_path),
            analysis_request: AnalysisRequest {
                content: expected_content.to_string(),
                location: LocationConstraint {
                    lat: location.lat,
                    long: location.lon,
                    max_distance,
                },
                datetime: None, // TODO: Add time constraints
            },
        };
        
        // GOTCHA: Must handle async processing workflow
        self.submit_validation(&request).await?;
        
        // PATTERN: Poll for completion with exponential backoff
        let mut attempts = 0;
        loop {
            let status = self.check_status(&processing_id).await?;
            match status.as_str() {
                "completed" => break,
                "failed" => return Err(ImageError::ValidationFailed),
                _ => {
                    if attempts > 30 { // CRITICAL: Prevent infinite polling
                        return Err(ImageError::Timeout);
                    }
                    tokio::time::sleep(Duration::from_millis(1000 + attempts * 100)).await;
                    attempts += 1;
                }
            }
        }
        
        self.get_results(&processing_id).await
    }
}
```

### Integration Points
```yaml
DATABASE:
  - migration: "Run sqlx migrate run to create initial schema"
  - indexes: "CREATE INDEX idx_challenge_participants ON challenge_participants(challenge_id, user_id)"
  - connection: "Configure DATABASE_URL environment variable"

CONFIG:
  - add to: .env
  - pattern: "JWT_SECRET=your-secret-key-here"
  - pattern: "IMAGE_CHECKER_URL=http://localhost:8080"
  - pattern: "IMAGE_BASE_DIR=/var/images"

EXTERNAL_SERVICES:
  - image-checker: "HTTP client integration for async image validation"
  - endpoints: "/validate, /status/{id}, /results/{id}"

MIDDLEWARE:
  - auth: "Apply JWT validation to protected endpoints"
  - logging: "Audit trail for all game events"
  - cors: "Enable cross-origin requests for web clients"
```

## Validation Loop

### Level 1: Syntax & Style
```bash
# Run these FIRST - fix any errors before proceeding
cargo fmt --check                    # Code formatting
cargo clippy -- -D warnings         # Linting and best practices
cargo check                         # Compilation check

# Expected: No errors. If errors, READ the error and fix.
```

### Level 2: Unit Tests each new feature/file/function use existing test patterns
```rust
// CREATE tests for each module following Rust conventions
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_token_creation() {
        let user = User::new("test@example.com", "password");
        let token = JwtService::create_user_token(&user);
        assert!(token.is_ok());
    }
    
    #[test] 
    fn test_location_validation_within_radius() {
        let target = GeoLocation { lat: 51.5074, lon: -0.1278 };
        let current = GeoLocation { lat: 51.5075, lon: -0.1279 };
        assert!(LocationService::validate_location(&target, &current, 100.0));
    }
    
    #[test]
    fn test_location_validation_outside_radius() {
        let target = GeoLocation { lat: 51.5074, lon: -0.1278 };
        let current = GeoLocation { lat: 51.6074, lon: -0.1278 }; // ~11km away
        assert!(!LocationService::validate_location(&target, &current, 100.0));
    }
    
    #[tokio::test]
    async fn test_image_validation_success() {
        let service = ImageService::new("http://localhost:8080");
        // GOTCHA: Requires running image-checker service for integration test
        // Use mock service for unit tests
    }
}
```

```bash
# Run and iterate until passing:
cargo test --all-features
# If failing: Read error, understand root cause, fix code, re-run
```

### Level 3: Integration Test
```bash
# Set up test database and environment
export DATABASE_URL="postgresql://test:test@localhost/scavenger_test"
export JWT_SECRET="test-secret-key"
export IMAGE_CHECKER_URL="http://localhost:8080"
export IMAGE_BASE_DIR="/tmp/test-images"

# Run database migrations
sqlx migrate run

# Start the service
cargo run --release

# Test authentication endpoint
curl -X POST http://localhost:3000/authentication/login \
  -H "Content-Type: application/json" \
  -d '{"username": "test@example.com", "password": "Password1"}'

# Expected: {"user-auth-token": "...", "expires_in": 7200, "token_type": "Bearer"}

# Test waypoint check-in
curl -X POST http://localhost:3000/challenges/waypoints/1/checkin \
  -H "Authorization: Bearer <participant-token>" \
  -H "Content-Type: application/json" \
  -d '{"location": {"lat": 51.5074, "lon": -0.1278}}'

# Expected: {"challenge-id": "...", "state": "CHECKED_IN", "proof": "..."}
```

## Final validation Checklist
- [ ] All tests pass: `cargo test --all-features`
- [ ] No linting errors: `cargo fmt --check && cargo clippy -- -D warnings`
- [ ] Manual test successful: Authentication and waypoint flows
- [ ] Database migrations apply successfully
- [ ] External image-checker integration working
- [ ] Environment variable configuration functional
- [ ] JWT token expiration handling correct
- [ ] Geolocation validation accurate within specified radius
- [ ] Audit logging capturing all events
- [ ] Error cases handled gracefully with proper HTTP status codes
- [ ] API responses match specification in examples/

---

## Anti-Patterns to Avoid
- ❌ Don't use unwrap() in production code - use proper error handling
- ❌ Don't hardcode secrets - use environment variables
- ❌ Don't skip database transactions for multi-step operations
- ❌ Don't ignore JWT token expiration validation
- ❌ Don't trust client-provided location data without validation
- ❌ Don't block async functions with synchronous operations
- ❌ Don't create new patterns when existing Rust idioms work
- ❌ Don't skip input validation on API endpoints
- ❌ Don't log sensitive information (passwords, tokens)
- ❌ Don't use String when &str would suffice for performance

## PRP Confidence Score: 9/10

This PRP provides comprehensive context for one-pass implementation success:
- ✅ Complete API specification with examples
- ✅ Modern Rust framework recommendations (Axum + SQLx)
- ✅ External service integration details (image-checker)
- ✅ Database schema and migration strategy
- ✅ JWT authentication patterns with proper claims
- ✅ Geolocation validation implementation
- ✅ Comprehensive error handling strategy
- ✅ Testing approach with specific test cases
- ✅ Environment configuration and deployment considerations
- ✅ Clear task ordering and dependencies

The -1 point reflects the complexity of external service integration and potential for async workflow timing issues, but all necessary context is provided for resolution.