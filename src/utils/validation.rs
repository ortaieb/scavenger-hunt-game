use regex::Regex;
use std::collections::HashMap;

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationErrors>;

/// Collection of validation errors
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn add_error(&mut self, field: &str, message: String) {
        self.errors
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(message);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_field_errors(self) -> HashMap<String, Vec<String>> {
        self.errors
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Validator trait for implementing custom validation logic
pub trait Validator<T> {
    fn validate(&self, value: &T) -> ValidationResult<()>;
}

/// Email validator
pub struct EmailValidator;

impl EmailValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn is_valid_email(email: &str) -> bool {
        // RFC 5322 compliant email regex (simplified)
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).unwrap();

        email.len() <= 254 && email_regex.is_match(email)
    }
}

impl Validator<String> for EmailValidator {
    fn validate(&self, email: &String) -> ValidationResult<()> {
        if email.is_empty() {
            return Err({
                let mut errors = ValidationErrors::new();
                errors.add_error("email", "Email is required".to_string());
                errors
            });
        }

        if !Self::is_valid_email(email) {
            return Err({
                let mut errors = ValidationErrors::new();
                errors.add_error("email", "Invalid email format".to_string());
                errors
            });
        }

        Ok(())
    }
}

/// Password validator
pub struct PasswordValidator {
    min_length: usize,
    require_uppercase: bool,
    require_lowercase: bool,
    require_digit: bool,
    require_special: bool,
}

impl PasswordValidator {
    pub fn new() -> Self {
        Self {
            min_length: 8,
            require_uppercase: false,
            require_lowercase: false,
            require_digit: false,
            require_special: false,
        }
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = length;
        self
    }

    pub fn require_uppercase(mut self) -> Self {
        self.require_uppercase = true;
        self
    }

    pub fn require_lowercase(mut self) -> Self {
        self.require_lowercase = true;
        self
    }

    pub fn require_digit(mut self) -> Self {
        self.require_digit = true;
        self
    }

    pub fn require_special_character(mut self) -> Self {
        self.require_special = true;
        self
    }
}

impl Validator<String> for PasswordValidator {
    fn validate(&self, password: &String) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();

        if password.is_empty() {
            errors.add_error("password", "Password is required".to_string());
            return Err(errors);
        }

        if password.len() < self.min_length {
            errors.add_error(
                "password",
                format!(
                    "Password must be at least {} characters long",
                    self.min_length
                ),
            );
        }

        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.add_error(
                "password",
                "Password must contain at least one uppercase letter".to_string(),
            );
        }

        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.add_error(
                "password",
                "Password must contain at least one lowercase letter".to_string(),
            );
        }

        if self.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.add_error(
                "password",
                "Password must contain at least one digit".to_string(),
            );
        }

        if self.require_special {
            let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
            if !password.chars().any(|c| special_chars.contains(c)) {
                errors.add_error(
                    "password",
                    "Password must contain at least one special character".to_string(),
                );
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// GPS coordinate validator
pub struct GpsCoordinateValidator;

impl GpsCoordinateValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn is_valid_latitude(lat: f64) -> bool {
        lat >= -90.0 && lat <= 90.0
    }

    pub fn is_valid_longitude(lon: f64) -> bool {
        lon >= -180.0 && lon <= 180.0
    }

    pub fn validate_coordinates(lat: f64, lon: f64) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();

        if !Self::is_valid_latitude(lat) {
            errors.add_error(
                "latitude",
                format!("Latitude must be between -90 and 90 degrees, got {}", lat),
            );
        }

        if !Self::is_valid_longitude(lon) {
            errors.add_error(
                "longitude",
                format!(
                    "Longitude must be between -180 and 180 degrees, got {}",
                    lon
                ),
            );
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// String length validator
pub struct StringLengthValidator {
    min_length: Option<usize>,
    max_length: Option<usize>,
}

impl StringLengthValidator {
    pub fn new() -> Self {
        Self {
            min_length: None,
            max_length: None,
        }
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = Some(length);
        self
    }

    pub fn max_length(mut self, length: usize) -> Self {
        self.max_length = Some(length);
        self
    }
}

impl Validator<String> for StringLengthValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();

        if let Some(min_len) = self.min_length {
            if value.len() < min_len {
                errors.add_error(
                    "length",
                    format!("Must be at least {} characters long", min_len),
                );
            }
        }

        if let Some(max_len) = self.max_length {
            if value.len() > max_len {
                errors.add_error(
                    "length",
                    format!("Must be no more than {} characters long", max_len),
                );
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// Required field validator
pub struct RequiredValidator;

impl RequiredValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Validator<Option<String>> for RequiredValidator {
    fn validate(&self, value: &Option<String>) -> ValidationResult<()> {
        match value {
            Some(s) if !s.trim().is_empty() => Ok(()),
            _ => {
                let mut errors = ValidationErrors::new();
                errors.add_error("required", "This field is required".to_string());
                Err(errors)
            }
        }
    }
}

impl Validator<String> for RequiredValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        if value.trim().is_empty() {
            let mut errors = ValidationErrors::new();
            errors.add_error("required", "This field is required".to_string());
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// Numeric range validator
pub struct NumericRangeValidator<T> {
    min: Option<T>,
    max: Option<T>,
}

impl<T> NumericRangeValidator<T>
where
    T: PartialOrd + Copy,
{
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    pub fn min(mut self, min_val: T) -> Self {
        self.min = Some(min_val);
        self
    }

    pub fn max(mut self, max_val: T) -> Self {
        self.max = Some(max_val);
        self
    }
}

impl<T> Validator<T> for NumericRangeValidator<T>
where
    T: PartialOrd + Copy + std::fmt::Display,
{
    fn validate(&self, value: &T) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();

        if let Some(min_val) = self.min {
            if *value < min_val {
                errors.add_error("range", format!("Value must be at least {}", min_val));
            }
        }

        if let Some(max_val) = self.max {
            if *value > max_val {
                errors.add_error("range", format!("Value must be no more than {}", max_val));
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// Collection of common validation functions
pub mod validators {
    use super::*;

    /// Validate user registration data
    pub fn validate_user_registration(
        email: &str,
        password: &str,
        nickname: Option<&str>,
    ) -> ValidationResult<()> {
        let mut all_errors = ValidationErrors::new();

        // Validate email
        if let Err(errors) = EmailValidator::new().validate(&email.to_string()) {
            for (field, messages) in errors.errors {
                for message in messages {
                    all_errors.add_error(&format!("email.{}", field), message);
                }
            }
        }

        // Validate password
        let password_validator = PasswordValidator::new()
            .min_length(8)
            .require_lowercase()
            .require_digit();

        if let Err(errors) = password_validator.validate(&password.to_string()) {
            for (field, messages) in errors.errors {
                for message in messages {
                    all_errors.add_error(&format!("password.{}", field), message);
                }
            }
        }

        // Validate nickname if provided
        if let Some(nick) = nickname {
            let nickname_validator = StringLengthValidator::new().min_length(2).max_length(50);

            if let Err(errors) = nickname_validator.validate(&nick.to_string()) {
                for (field, messages) in errors.errors {
                    for message in messages {
                        all_errors.add_error(&format!("nickname.{}", field), message);
                    }
                }
            }
        }

        if all_errors.has_errors() {
            Err(all_errors)
        } else {
            Ok(())
        }
    }

    /// Validate challenge data
    pub fn validate_challenge_data(
        name: &str,
        description: Option<&str>,
        duration_minutes: i32,
    ) -> ValidationResult<()> {
        let mut all_errors = ValidationErrors::new();

        // Validate name
        let name_validator = StringLengthValidator::new().min_length(3).max_length(100);

        if let Err(errors) = name_validator.validate(&name.to_string()) {
            for (field, messages) in errors.errors {
                for message in messages {
                    all_errors.add_error(&format!("name.{}", field), message);
                }
            }
        }

        // Validate description if provided
        if let Some(desc) = description {
            let desc_validator = StringLengthValidator::new().max_length(1000);

            if let Err(errors) = desc_validator.validate(&desc.to_string()) {
                for (field, messages) in errors.errors {
                    for message in messages {
                        all_errors.add_error(&format!("description.{}", field), message);
                    }
                }
            }
        }

        // Validate duration
        let duration_validator = NumericRangeValidator::new().min(1).max(1440); // Max 24 hours

        if let Err(errors) = duration_validator.validate(&duration_minutes) {
            for (field, messages) in errors.errors {
                for message in messages {
                    all_errors.add_error(&format!("duration.{}", field), message);
                }
            }
        }

        if all_errors.has_errors() {
            Err(all_errors)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let validator = EmailValidator::new();

        // Valid emails
        assert!(validator.validate(&"test@example.com".to_string()).is_ok());
        assert!(validator
            .validate(&"user.name@domain.co.uk".to_string())
            .is_ok());

        // Invalid emails
        assert!(validator.validate(&"invalid_email".to_string()).is_err());
        assert!(validator.validate(&"@domain.com".to_string()).is_err());
        assert!(validator.validate(&"user@".to_string()).is_err());
        assert!(validator.validate(&"".to_string()).is_err());
    }

    #[test]
    fn test_password_validation() {
        let validator = PasswordValidator::new()
            .min_length(8)
            .require_lowercase()
            .require_digit();

        // Valid passwords
        assert!(validator.validate(&"password123".to_string()).is_ok());
        assert!(validator.validate(&"mypassword1".to_string()).is_ok());

        // Invalid passwords
        assert!(validator.validate(&"short".to_string()).is_err());
        assert!(validator.validate(&"PASSWORD123".to_string()).is_err()); // No lowercase
        assert!(validator.validate(&"passwordonly".to_string()).is_err()); // No digit
        assert!(validator.validate(&"".to_string()).is_err());
    }

    #[test]
    fn test_gps_coordinate_validation() {
        // Valid coordinates
        assert!(GpsCoordinateValidator::validate_coordinates(51.5074, -0.1278).is_ok());
        assert!(GpsCoordinateValidator::validate_coordinates(0.0, 0.0).is_ok());
        assert!(GpsCoordinateValidator::validate_coordinates(90.0, 180.0).is_ok());
        assert!(GpsCoordinateValidator::validate_coordinates(-90.0, -180.0).is_ok());

        // Invalid coordinates
        assert!(GpsCoordinateValidator::validate_coordinates(91.0, 0.0).is_err());
        assert!(GpsCoordinateValidator::validate_coordinates(-91.0, 0.0).is_err());
        assert!(GpsCoordinateValidator::validate_coordinates(0.0, 181.0).is_err());
        assert!(GpsCoordinateValidator::validate_coordinates(0.0, -181.0).is_err());
    }

    #[test]
    fn test_string_length_validation() {
        let validator = StringLengthValidator::new().min_length(3).max_length(10);

        // Valid strings
        assert!(validator.validate(&"test".to_string()).is_ok());
        assert!(validator.validate(&"hello".to_string()).is_ok());

        // Invalid strings
        assert!(validator.validate(&"hi".to_string()).is_err()); // Too short
        assert!(validator.validate(&"this is too long".to_string()).is_err()); // Too long
    }

    #[test]
    fn test_required_validation() {
        let validator = RequiredValidator::new();

        // Valid values
        assert!(validator.validate(&"test".to_string()).is_ok());
        assert!(validator.validate(&Some("test".to_string())).is_ok());

        // Invalid values
        assert!(validator.validate(&"".to_string()).is_err());
        assert!(validator.validate(&"   ".to_string()).is_err()); // Only whitespace
        assert!(validator.validate(&None).is_err());
    }

    #[test]
    fn test_numeric_range_validation() {
        let validator = NumericRangeValidator::new().min(1).max(100);

        // Valid values
        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&1).is_ok());
        assert!(validator.validate(&100).is_ok());

        // Invalid values
        assert!(validator.validate(&0).is_err()); // Below min
        assert!(validator.validate(&101).is_err()); // Above max
    }

    #[test]
    fn test_user_registration_validation() {
        // Valid registration
        assert!(validators::validate_user_registration(
            "test@example.com",
            "password123",
            Some("TestUser")
        )
        .is_ok());

        // Invalid registration
        assert!(
            validators::validate_user_registration("invalid_email", "short", Some("A")).is_err()
        );
    }
}
