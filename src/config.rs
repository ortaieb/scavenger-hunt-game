use std::env;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    pub image_checker_url: String,
    pub image_base_dir: String,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingEnvironmentVariable(String),
    InvalidValue(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingEnvironmentVariable(var) => {
                write!(f, "Missing required environment variable: {var}")
            }
            ConfigError::InvalidValue(msg) => write!(f, "Invalid configuration value: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if it exists
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvironmentVariable("DATABASE_URL".to_string()))?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingEnvironmentVariable("JWT_SECRET".to_string()))?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::InvalidValue(
                "JWT_SECRET must be at least 32 characters long".to_string(),
            ));
        }

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidValue("PORT must be a valid number".to_string()))?;

        let image_checker_url = env::var("IMAGE_CHECKER_URL").map_err(|_| {
            ConfigError::MissingEnvironmentVariable("IMAGE_CHECKER_URL".to_string())
        })?;

        let image_base_dir = env::var("IMAGE_BASE_DIR")
            .map_err(|_| ConfigError::MissingEnvironmentVariable("IMAGE_BASE_DIR".to_string()))?;

        Ok(Config {
            database_url,
            jwt_secret,
            host,
            port,
            image_checker_url,
            image_base_dir,
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_from_env_invalid_jwt_secret() {
        // Store original JWT secret
        let original_jwt_secret = env::var("JWT_SECRET").ok();
        
        // Set an invalid JWT secret (too short)
        env::set_var("JWT_SECRET", "short");
        
        let result = Config::from_env();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidValue(_)
        ));
        
        // Restore original value
        if let Some(jwt_secret) = original_jwt_secret {
            env::set_var("JWT_SECRET", jwt_secret);
        } else {
            env::remove_var("JWT_SECRET");
        }
    }

    #[test]
    fn test_server_address() {
        let config = Config {
            database_url: "test".to_string(),
            jwt_secret: "test-secret-key-that-is-long-enough-32chars".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            image_checker_url: "http://localhost:8080".to_string(),
            image_base_dir: "/tmp".to_string(),
        };
        assert_eq!(config.server_address(), "localhost:8080");
    }
}
