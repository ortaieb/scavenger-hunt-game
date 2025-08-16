use geo_types::Point;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub lat: f64,
    #[serde(rename = "long")]
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationValidationRequest {
    pub location: GeoLocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationValidationResult {
    pub is_valid: bool,
    pub distance_meters: f64,
    pub max_distance_meters: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Invalid coordinates: lat={lat}, lon={lon}")]
    InvalidCoordinates { lat: f64, lon: f64 },
    #[error("Waypoint not found")]
    WaypointNotFound,
    #[error("Location outside allowed radius")]
    LocationOutsideRadius { distance: f64, max_distance: f64 },
}

pub struct LocationService {
    pool: PgPool,
}

impl LocationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Validate if a location is within the allowed radius of a waypoint
    pub async fn validate_waypoint_location(
        &self,
        waypoint_id: i32,
        user_location: &GeoLocation,
    ) -> Result<LocationValidationResult, LocationError> {
        // Get waypoint details
        let waypoint = sqlx::query!(
            r#"
            SELECT location_lat, location_lon, radius_meters
            FROM waypoints
            WHERE waypoint_id = $1
            "#,
            waypoint_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(LocationError::WaypointNotFound)?;

        let target_location = GeoLocation {
            lat: waypoint.location_lat,
            lon: waypoint.location_lon,
        };

        let distance = self.calculate_distance(&target_location, user_location)?;
        let is_valid = distance <= waypoint.radius_meters;

        if !is_valid {
            return Ok(LocationValidationResult {
                is_valid: false,
                distance_meters: distance,
                max_distance_meters: waypoint.radius_meters,
            });
        }

        Ok(LocationValidationResult {
            is_valid: true,
            distance_meters: distance,
            max_distance_meters: waypoint.radius_meters,
        })
    }

    /// Calculate distance between two GPS coordinates using Haversine formula
    pub fn calculate_distance(
        &self,
        target: &GeoLocation,
        current: &GeoLocation,
    ) -> Result<f64, LocationError> {
        // Validate coordinates
        self.validate_coordinates(target)?;
        self.validate_coordinates(current)?;

        const EARTH_RADIUS: f64 = 6371000.0; // Earth radius in meters

        let lat1_rad = target.lat.to_radians();
        let lat2_rad = current.lat.to_radians();
        let delta_lat = (current.lat - target.lat).to_radians();
        let delta_lon = (current.lon - target.lon).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let distance = EARTH_RADIUS * c;

        Ok(distance)
    }

    /// Validate GPS coordinates are within valid ranges
    pub fn validate_coordinates(&self, location: &GeoLocation) -> Result<(), LocationError> {
        if location.lat < -90.0 || location.lat > 90.0 {
            return Err(LocationError::InvalidCoordinates {
                lat: location.lat,
                lon: location.lon,
            });
        }

        if location.lon < -180.0 || location.lon > 180.0 {
            return Err(LocationError::InvalidCoordinates {
                lat: location.lat,
                lon: location.lon,
            });
        }

        Ok(())
    }

    /// Log participant location for tracking and auditing
    pub async fn log_participant_location(
        &self,
        participant_id: Uuid,
        location: &GeoLocation,
        accuracy_meters: Option<f64>,
    ) -> Result<(), LocationError> {
        self.validate_coordinates(location)?;

        sqlx::query!(
            r#"
            INSERT INTO geolocation_log (participant_id, location_lat, location_lon, accuracy_meters)
            VALUES ($1, $2, $3, $4)
            "#,
            participant_id,
            location.lat,
            location.lon,
            accuracy_meters
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent location history for a participant
    pub async fn get_participant_location_history(
        &self,
        participant_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<GeoLocation>, LocationError> {
        let limit = limit.unwrap_or(50);

        let locations = sqlx::query!(
            r#"
            SELECT location_lat, location_lon
            FROM geolocation_log
            WHERE participant_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            participant_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(locations
            .into_iter()
            .map(|row| GeoLocation {
                lat: row.location_lat,
                lon: row.location_lon,
            })
            .collect())
    }

    /// Check if a location is within a simple bounding box (fast pre-check)
    pub fn is_within_bounding_box(
        &self,
        target: &GeoLocation,
        current: &GeoLocation,
        radius_meters: f64,
    ) -> bool {
        // Rough approximation: 1 degree ≈ 111,320 meters
        let degree_tolerance = radius_meters / 111320.0;

        let lat_diff = (current.lat - target.lat).abs();
        let lon_diff = (current.lon - target.lon).abs();

        lat_diff <= degree_tolerance && lon_diff <= degree_tolerance
    }

    /// Convert GeoLocation to geo-types Point for spatial operations
    pub fn to_point(location: &GeoLocation) -> Point<f64> {
        Point::new(location.lon, location.lat)
    }

    /// Convert geo-types Point back to GeoLocation
    pub fn from_point(point: Point<f64>) -> GeoLocation {
        GeoLocation {
            lat: point.y(),
            lon: point.x(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distance_calculation() {
        let service = LocationService::new(
            PgPool::connect("postgresql://test:test@localhost/test")
                .await
                .expect("Failed to connect to test database"),
        );

        // Test known distance: London to New York (approximately 5585 km)
        let london = GeoLocation {
            lat: 51.5074,
            lon: -0.1278,
        };
        let new_york = GeoLocation {
            lat: 40.7128,
            lon: -74.0060,
        };

        let distance = service.calculate_distance(&london, &new_york).unwrap();

        // Distance should be approximately 5585 km (5,585,000 meters)
        // Allow for some variance due to Earth's curvature approximation
        assert!(distance > 5_500_000.0 && distance < 5_600_000.0);
    }

    #[tokio::test]
    async fn test_short_distance_calculation() {
        let service = LocationService::new(
            PgPool::connect("postgresql://test:test@localhost/test")
                .await
                .expect("Failed to connect to test database"),
        );

        // Test short distance: two points very close together
        let point1 = GeoLocation {
            lat: 51.5074,
            lon: -0.1278,
        };
        let point2 = GeoLocation {
            lat: 51.5075, // About 11 meters north
            lon: -0.1278,
        };

        let distance = service.calculate_distance(&point1, &point2).unwrap();

        // Should be approximately 11 meters
        assert!(distance > 10.0 && distance < 15.0);
    }

    #[tokio::test]
    async fn test_coordinate_validation() {
        let service = LocationService::new(
            PgPool::connect("postgresql://test:test@localhost/test")
                .await
                .expect("Failed to connect to test database"),
        );

        // Valid coordinates
        let valid_location = GeoLocation {
            lat: 51.5074,
            lon: -0.1278,
        };
        assert!(service.validate_coordinates(&valid_location).is_ok());

        // Invalid latitude (too high)
        let invalid_lat = GeoLocation {
            lat: 91.0,
            lon: 0.0,
        };
        assert!(service.validate_coordinates(&invalid_lat).is_err());

        // Invalid longitude (too low)
        let invalid_lon = GeoLocation {
            lat: 0.0,
            lon: -181.0,
        };
        assert!(service.validate_coordinates(&invalid_lon).is_err());
    }

    #[tokio::test]
    async fn test_bounding_box_check() {
        let service = LocationService::new(
            PgPool::connect("postgresql://test:test@localhost/test")
                .await
                .expect("Failed to connect to test database"),
        );

        let target = GeoLocation {
            lat: 51.5074,
            lon: -0.1278,
        };

        // Point within 100 meters
        let nearby = GeoLocation {
            lat: 51.5075,
            lon: -0.1279,
        };
        assert!(service.is_within_bounding_box(&target, &nearby, 100.0));

        // Point far away
        let far_away = GeoLocation {
            lat: 52.0,
            lon: 0.0,
        };
        assert!(!service.is_within_bounding_box(&target, &far_away, 100.0));
    }

    #[test]
    fn test_point_conversion() {
        let location = GeoLocation {
            lat: 51.5074,
            lon: -0.1278,
        };

        let point = LocationService::to_point(&location);
        let converted_back = LocationService::from_point(point);

        assert_eq!(location.lat, converted_back.lat);
        assert_eq!(location.lon, converted_back.lon);
    }
}
