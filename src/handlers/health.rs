use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::db::health_check;
use crate::routes::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub database: String,
}

/// Health check endpoint
/// GET /health
pub async fn health_check_handler(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let timestamp = chrono::Utc::now();

    // Check database health
    let database_status = match health_check(&state.pool).await {
        Ok(_) => "healthy".to_string(),
        Err(_) => "unhealthy".to_string(),
    };

    let overall_status = if database_status == "healthy" {
        "healthy"
    } else {
        "unhealthy"
    };

    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp,
        database: database_status,
    };

    if overall_status == "healthy" {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            database: "healthy".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("timestamp"));
        assert!(json.contains("database"));
        assert!(json.contains("healthy"));
    }
}
