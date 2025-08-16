use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::services::location_service::GeoLocation;

#[derive(Debug, Clone, Serialize)]
pub struct ImageValidationRequest {
    #[serde(rename = "processing-id")]
    pub processing_id: String,
    #[serde(rename = "image-path")]
    pub image_path: String,
    #[serde(rename = "analysis-request")]
    pub analysis_request: AnalysisRequest,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisRequest {
    pub content: String,
    pub location: Option<LocationConstraint>,
    pub datetime: Option<DateTimeConstraint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationConstraint {
    pub lat: f64,
    #[serde(rename = "long")]
    pub long: f64,
    pub max_distance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DateTimeConstraint {
    pub start: DateTime<Utc>,
    pub duration: i64, // Duration in minutes
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidationResult {
    pub resolution: String, // "accepted" or "rejected"
    pub reasons: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Image validation failed")]
    ValidationFailed,
    #[error("Processing timeout")]
    Timeout,
    #[error("Invalid image path: {0}")]
    InvalidImagePath(String),
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),
}

pub struct ImageService {
    client: Client,
    base_url: String,
    image_base_dir: String,
}

impl ImageService {
    pub fn new(base_url: String, image_base_dir: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            image_base_dir,
        }
    }

    /// Validate an image with the external image-checker service
    pub async fn validate_image(
        &self,
        image_path: &str,
        expected_content: &str,
        location: Option<&GeoLocation>,
        max_distance: Option<f64>,
        datetime_constraint: Option<DateTimeConstraint>,
    ) -> Result<ValidationResult, ImageError> {
        let processing_id = Uuid::new_v4().to_string();

        // Build full image path
        let full_image_path = self.build_image_path(image_path)?;

        // Create location constraint if provided
        let location_constraint = location.map(|loc| LocationConstraint {
            lat: loc.lat,
            long: loc.lon,
            max_distance: max_distance.unwrap_or(50.0),
        });

        // Submit validation request
        let request = ImageValidationRequest {
            processing_id: processing_id.clone(),
            image_path: full_image_path,
            analysis_request: AnalysisRequest {
                content: expected_content.to_string(),
                location: location_constraint,
                datetime: datetime_constraint,
            },
        };

        self.submit_validation(&request).await?;

        // Poll for completion
        self.poll_for_completion(&processing_id).await?;

        // Get results
        self.get_results(&processing_id).await
    }

    /// Submit validation request to the image-checker service
    async fn submit_validation(&self, request: &ImageValidationRequest) -> Result<(), ImageError> {
        let url = format!("{}/validate", self.base_url);

        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            return Err(ImageError::ServiceUnavailable);
        }

        Ok(())
    }

    /// Poll for processing completion with exponential backoff
    async fn poll_for_completion(&self, processing_id: &str) -> Result<(), ImageError> {
        let mut attempts = 0;
        let max_attempts = 30;
        let base_delay = 1000; // 1 second base delay

        loop {
            let status = self.check_status(processing_id).await?;

            match status.status.as_str() {
                "completed" => return Ok(()),
                "failed" => return Err(ImageError::ValidationFailed),
                "in_progress" | "accepted" => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(ImageError::Timeout);
                    }

                    // Exponential backoff with jitter
                    let delay = base_delay + (attempts * 100);
                    sleep(Duration::from_millis(delay)).await;
                }
                _ => {
                    return Err(ImageError::UnexpectedResponse(format!(
                        "Unknown status: {}",
                        status.status
                    )));
                }
            }
        }
    }

    /// Check processing status
    async fn check_status(&self, processing_id: &str) -> Result<StatusResponse, ImageError> {
        let url = format!("{}/status/{}", self.base_url, processing_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ImageError::ServiceUnavailable);
        }

        let status = response.json::<StatusResponse>().await?;
        Ok(status)
    }

    /// Get validation results
    async fn get_results(&self, processing_id: &str) -> Result<ValidationResult, ImageError> {
        let url = format!("{}/results/{}", self.base_url, processing_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ImageError::ServiceUnavailable);
        }

        let result = response.json::<ValidationResult>().await?;
        Ok(result)
    }

    /// Build full image path from relative path
    fn build_image_path(&self, relative_path: &str) -> Result<String, ImageError> {
        // Validate path doesn't contain dangerous characters
        if relative_path.contains("..") || relative_path.starts_with('/') {
            return Err(ImageError::InvalidImagePath(
                "Path contains invalid characters".to_string(),
            ));
        }

        // Support both local and cloud storage paths
        if self.image_base_dir.starts_with("http://") || self.image_base_dir.starts_with("https://")
        {
            // Cloud storage URL
            Ok(format!(
                "{}/{}",
                self.image_base_dir.trim_end_matches('/'),
                relative_path
            ))
        } else {
            // Local file path
            Ok(format!(
                "file://{}/{}",
                self.image_base_dir.trim_end_matches('/'),
                relative_path
            ))
        }
    }

    /// Validate image format and size (basic checks)
    pub fn validate_image_format(&self, filename: &str) -> Result<(), ImageError> {
        let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp"];

        let extension = filename
            .split('.')
            .next_back()
            .ok_or_else(|| ImageError::InvalidImagePath("No file extension".to_string()))?
            .to_lowercase();

        if !allowed_extensions.contains(&extension.as_str()) {
            return Err(ImageError::InvalidImagePath(format!(
                "Unsupported file format: {extension}"
            )));
        }

        Ok(())
    }

    /// Create datetime constraint for current time window
    pub fn create_current_time_constraint(duration_minutes: i64) -> DateTimeConstraint {
        let now = Utc::now();
        DateTimeConstraint {
            start: now - chrono::Duration::minutes(duration_minutes),
            duration: duration_minutes * 2, // Allow past and future window
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_image_path_local() {
        let service = ImageService::new(
            "http://localhost:8080".to_string(),
            "/var/images".to_string(),
        );

        let path = service.build_image_path("test/image.jpg").unwrap();
        assert_eq!(path, "file:///var/images/test/image.jpg");
    }

    #[test]
    fn test_build_image_path_cloud() {
        let service = ImageService::new(
            "http://localhost:8080".to_string(),
            "https://storage.googleapis.com/my-bucket".to_string(),
        );

        let path = service.build_image_path("test/image.jpg").unwrap();
        assert_eq!(
            path,
            "https://storage.googleapis.com/my-bucket/test/image.jpg"
        );
    }

    #[test]
    fn test_invalid_image_path() {
        let service = ImageService::new(
            "http://localhost:8080".to_string(),
            "/var/images".to_string(),
        );

        // Test path traversal attack
        let result = service.build_image_path("../../../etc/passwd");
        assert!(result.is_err());

        // Test absolute path
        let result = service.build_image_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_image_format() {
        let service = ImageService::new(
            "http://localhost:8080".to_string(),
            "/var/images".to_string(),
        );

        // Valid formats
        assert!(service.validate_image_format("test.jpg").is_ok());
        assert!(service.validate_image_format("test.PNG").is_ok());
        assert!(service.validate_image_format("test.jpeg").is_ok());

        // Invalid formats
        assert!(service.validate_image_format("test.txt").is_err());
        assert!(service.validate_image_format("test.exe").is_err());
        assert!(service.validate_image_format("test").is_err());
    }

    #[test]
    fn test_create_time_constraint() {
        let constraint = ImageService::create_current_time_constraint(10);
        let now = Utc::now();

        // Start time should be 10 minutes ago
        let expected_start = now - chrono::Duration::minutes(10);
        let time_diff = (constraint.start - expected_start).num_seconds().abs();

        // Allow for small timing differences (less than 1 second)
        assert!(time_diff < 1);

        // Duration should be 20 minutes (10 past + 10 future)
        assert_eq!(constraint.duration, 20);
    }

    #[tokio::test]
    async fn test_image_validation_request_serialization() {
        let location = LocationConstraint {
            lat: 51.5074,
            long: -0.1278,
            max_distance: 100.0,
        };

        let request = ImageValidationRequest {
            processing_id: "test-id".to_string(),
            image_path: "test/image.jpg".to_string(),
            analysis_request: AnalysisRequest {
                content: "A red bicycle".to_string(),
                location: Some(location),
                datetime: None,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("processing-id"));
        assert!(json.contains("image-path"));
        assert!(json.contains("analysis-request"));
        assert!(json.contains("A red bicycle"));
    }
}
