use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard API response structure for successful operations
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Standard error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub success: bool,
    pub error: ErrorDetails,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub details: Option<HashMap<String, String>>,
}

/// Validation error response for field-specific errors
#[derive(Debug, Clone, Serialize)]
pub struct ValidationErrorResponse {
    pub success: bool,
    pub error: ValidationErrorDetails,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationErrorDetails {
    pub code: String,
    pub message: String,
    pub field_errors: HashMap<String, Vec<String>>,
}

/// Response builder for consistent API responses
pub struct ResponseBuilder;

impl ResponseBuilder {
    /// Create a successful response
    #[allow(dead_code)]
    pub fn success<T: Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
        (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                data,
                message: None,
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create a successful response with custom status code
    pub fn success_with_status<T: Serialize>(
        status: StatusCode,
        data: T,
    ) -> (StatusCode, Json<ApiResponse<T>>) {
        (
            status,
            Json(ApiResponse {
                success: true,
                data,
                message: None,
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create a successful response with message
    pub fn success_with_message<T: Serialize>(
        data: T,
        message: String,
    ) -> (StatusCode, Json<ApiResponse<T>>) {
        (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                data,
                message: Some(message),
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create an error response
    #[allow(dead_code)]
    pub fn error(
        status: StatusCode,
        code: String,
        message: String,
    ) -> (StatusCode, Json<ApiErrorResponse>) {
        (
            status,
            Json(ApiErrorResponse {
                success: false,
                error: ErrorDetails {
                    code,
                    message,
                    details: None,
                },
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create an error response with details
    #[allow(dead_code)]
    pub fn error_with_details(
        status: StatusCode,
        code: String,
        message: String,
        details: HashMap<String, String>,
    ) -> (StatusCode, Json<ApiErrorResponse>) {
        (
            status,
            Json(ApiErrorResponse {
                success: false,
                error: ErrorDetails {
                    code,
                    message,
                    details: Some(details),
                },
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create a validation error response
    #[allow(dead_code)]
    pub fn validation_error(
        field_errors: HashMap<String, Vec<String>>,
    ) -> (StatusCode, Json<ValidationErrorResponse>) {
        (
            StatusCode::BAD_REQUEST,
            Json(ValidationErrorResponse {
                success: false,
                error: ValidationErrorDetails {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "Request validation failed".to_string(),
                    field_errors,
                },
                timestamp: chrono::Utc::now(),
            }),
        )
    }

    /// Create a not found error response
    #[allow(dead_code)]
    pub fn not_found(resource: &str) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND".to_string(),
            format!("{resource} not found"),
        )
    }

    /// Create an unauthorized error response
    #[allow(dead_code)]
    pub fn unauthorized(message: Option<String>) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED".to_string(),
            message.unwrap_or_else(|| "Authentication required".to_string()),
        )
    }

    /// Create a forbidden error response
    #[allow(dead_code)]
    pub fn forbidden(message: Option<String>) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(
            StatusCode::FORBIDDEN,
            "FORBIDDEN".to_string(),
            message.unwrap_or_else(|| "Insufficient permissions".to_string()),
        )
    }

    /// Create a conflict error response
    #[allow(dead_code)]
    pub fn conflict(message: String) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(StatusCode::CONFLICT, "CONFLICT".to_string(), message)
    }

    /// Create an internal server error response
    #[allow(dead_code)]
    pub fn internal_server_error(message: Option<String>) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_SERVER_ERROR".to_string(),
            message.unwrap_or_else(|| "An internal server error occurred".to_string()),
        )
    }

    /// Create a bad request error response
    #[allow(dead_code)]
    pub fn bad_request(message: String) -> (StatusCode, Json<ApiErrorResponse>) {
        Self::error(StatusCode::BAD_REQUEST, "BAD_REQUEST".to_string(), message)
    }
}

/// Common error codes used throughout the API
pub struct ErrorCodes;

impl ErrorCodes {
    #[allow(dead_code)]
    pub const VALIDATION_ERROR: &'static str = "VALIDATION_ERROR";
    #[allow(dead_code)]
    pub const AUTHENTICATION_FAILED: &'static str = "AUTHENTICATION_FAILED";
    #[allow(dead_code)]
    pub const AUTHORIZATION_FAILED: &'static str = "AUTHORIZATION_FAILED";
    #[allow(dead_code)]
    pub const RESOURCE_NOT_FOUND: &'static str = "RESOURCE_NOT_FOUND";
    #[allow(dead_code)]
    pub const RESOURCE_CONFLICT: &'static str = "RESOURCE_CONFLICT";
    #[allow(dead_code)]
    pub const INVALID_REQUEST: &'static str = "INVALID_REQUEST";
    #[allow(dead_code)]
    pub const EXTERNAL_SERVICE_ERROR: &'static str = "EXTERNAL_SERVICE_ERROR";
    #[allow(dead_code)]
    pub const DATABASE_ERROR: &'static str = "DATABASE_ERROR";
    #[allow(dead_code)]
    pub const RATE_LIMIT_EXCEEDED: &'static str = "RATE_LIMIT_EXCEEDED";
}

/// Helper functions for common response patterns
pub mod helpers {
    use super::*;

    /// Convert validation errors to field error map
    #[allow(dead_code)]
    pub fn validation_errors_to_field_map(
        errors: Vec<(String, String)>,
    ) -> HashMap<String, Vec<String>> {
        let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

        for (field, message) in errors {
            field_errors.entry(field).or_default().push(message);
        }

        field_errors
    }

    /// Create a standardized pagination response
    #[derive(Debug, Clone, Serialize)]
    pub struct PaginatedResponse<T> {
        pub items: Vec<T>,
        pub total_count: i64,
        pub page: i32,
        pub page_size: i32,
        pub total_pages: i32,
    }

    impl<T> PaginatedResponse<T> {
        #[allow(dead_code)]
        pub fn new(items: Vec<T>, total_count: i64, page: i32, page_size: i32) -> Self {
            let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;

            Self {
                items,
                total_count,
                page,
                page_size,
                total_pages,
            }
        }
    }

    /// Create success response for created resources
    #[allow(dead_code)]
    pub fn created<T: Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
        ResponseBuilder::success_with_status(StatusCode::CREATED, data)
    }

    /// Create success response for updated resources
    #[allow(dead_code)]
    pub fn updated<T: Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
        ResponseBuilder::success_with_message(data, "Resource updated successfully".to_string())
    }

    /// Create success response for deleted resources
    #[allow(dead_code)]
    pub fn deleted() -> (StatusCode, Json<ApiResponse<()>>) {
        ResponseBuilder::success_with_message((), "Resource deleted successfully".to_string())
    }

    /// Create response for no content
    #[allow(dead_code)]
    pub fn no_content() -> StatusCode {
        StatusCode::NO_CONTENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response_structure() {
        let data = "test_data";
        let (status, response) = ResponseBuilder::success(data);

        assert_eq!(status, StatusCode::OK);
        assert!(response.0.success);
        assert_eq!(response.0.data, "test_data");
        assert!(response.0.message.is_none());
    }

    #[test]
    fn test_error_response_structure() {
        let (status, response) = ResponseBuilder::error(
            StatusCode::BAD_REQUEST,
            "TEST_ERROR".to_string(),
            "Test error message".to_string(),
        );

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!response.0.success);
        assert_eq!(response.0.error.code, "TEST_ERROR");
        assert_eq!(response.0.error.message, "Test error message");
    }

    #[test]
    fn test_validation_error_response() {
        let mut field_errors = HashMap::new();
        field_errors.insert(
            "email".to_string(),
            vec!["Invalid email format".to_string()],
        );
        field_errors.insert(
            "password".to_string(),
            vec!["Password too short".to_string()],
        );

        let (status, response) = ResponseBuilder::validation_error(field_errors.clone());

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!response.0.success);
        assert_eq!(response.0.error.code, "VALIDATION_ERROR");
        assert_eq!(response.0.error.field_errors, field_errors);
    }

    #[test]
    fn test_validation_errors_to_field_map() {
        let errors = vec![
            ("email".to_string(), "Invalid format".to_string()),
            ("email".to_string(), "Required field".to_string()),
            ("password".to_string(), "Too short".to_string()),
        ];

        let field_map = helpers::validation_errors_to_field_map(errors);

        assert_eq!(field_map.get("email").unwrap().len(), 2);
        assert_eq!(field_map.get("password").unwrap().len(), 1);
        assert!(field_map
            .get("email")
            .unwrap()
            .contains(&"Invalid format".to_string()));
        assert!(field_map
            .get("email")
            .unwrap()
            .contains(&"Required field".to_string()));
    }

    #[test]
    fn test_paginated_response() {
        let items = vec![1, 2, 3, 4, 5];
        let paginated = helpers::PaginatedResponse::new(items, 25, 1, 5);

        assert_eq!(paginated.total_count, 25);
        assert_eq!(paginated.page, 1);
        assert_eq!(paginated.page_size, 5);
        assert_eq!(paginated.total_pages, 5);
        assert_eq!(paginated.items.len(), 5);
    }
}
