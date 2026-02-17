use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port to bind to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Log level for the application
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Docker socket path (usually auto-detected)
    #[serde(default)]
    pub docker_socket: Option<String>,

    /// Metrics update interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval_seconds: u64,

    /// Maximum number of metrics history points to keep
    #[serde(default = "default_metrics_history")]
    pub metrics_history_limit: usize,

    /// Maximum number of containers to show in charts (for performance and readability)
    #[serde(default = "default_max_chart_containers")]
    pub max_chart_containers: usize,

    /// Graceful shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    #[allow(dead_code)]
    pub shutdown_timeout_seconds: u64,

    /// Authentication settings
    /// Enable/disable authentication (default: true for security)
    #[serde(default = "default_auth_enabled")]
    pub auth_enabled: bool,

    /// Username for basic authentication (default: admin)
    #[serde(default = "default_auth_username")]
    pub auth_username: String,

    /// Password for basic authentication (default: randomly generated on first run)
    #[serde(default)]
    pub auth_password: Option<String>,

    /// Hashed password (internal use, generated from password)
    #[serde(default)]
    pub auth_password_hash: Option<String>,

    /// Session timeout in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_session_timeout")]
    pub session_timeout_seconds: u64,
}

impl Config {
    /// Load configuration from environment variables with .env file support
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Load .env file if present (ignored if not found)
        let _ = dotenvy::dotenv();

        // Use envy to deserialize from environment variables with SDM_ prefix
        let mut config = envy::prefixed("SDM_").from_env::<Config>()?;

        // Handle authentication setup
        if config.auth_enabled {
            config.setup_authentication().map_err(|e| e.to_string())?;
        } else {
            tracing::warn!("🚨 ================================");
            tracing::warn!("🚨 SECURITY WARNING: AUTHENTICATION DISABLED");
            tracing::warn!("🚨 This application has FULL Docker access!");
            tracing::warn!("🚨 Anyone can access and control containers!");
            tracing::warn!("🚨 Set SDM_AUTH_ENABLED=true for security");
            tracing::warn!("🚨 ================================");
        }

        tracing::info!("Configuration loaded: {:#?}", config);
        Ok(config)
    }

    /// Set up authentication by handling password and hashing
    fn setup_authentication(&mut self) -> Result<(), String> {
        match (&self.auth_password, &self.auth_password_hash) {
            (Some(password), None) => {
                // Hash the provided password
                self.auth_password_hash = Some(Self::hash_password(password)?);
                tracing::info!("🔐 Password hashed for user: {}", self.auth_username);
            }
            (None, None) => {
                // Try to load password from file first
                if let Some(saved_password) = Self::load_password_from_file() {
                    let password_file = Self::get_password_file_path();
                    tracing::info!("🔐 Loaded saved password from {}", password_file);
                    self.auth_password_hash = Some(Self::hash_password(&saved_password)?);
                    tracing::info!("🔐 ================================");
                    tracing::info!("🔐 AUTHENTICATION ENABLED");
                    tracing::info!("🔐 Using saved password for user '{}'", self.auth_username);
                    tracing::info!("🔐 Password available in: {}", password_file);
                    tracing::info!("🔐 ================================");
                } else {
                    // Generate a new password and save it
                    let generated_password = Self::generate_password();
                    self.auth_password_hash = Some(Self::hash_password(&generated_password)?);

                    let password_file = Self::get_password_file_path();

                    // Save password to file
                    if let Err(e) = Self::save_password_to_file(&generated_password) {
                        tracing::error!("Failed to save password to file: {}", e);
                    }

                    tracing::warn!("🔐 ================================");
                    tracing::warn!("🔐 AUTHENTICATION ENABLED");
                    tracing::warn!(
                        "🔐 Generated secure password for user '{}':",
                        self.auth_username
                    );
                    tracing::warn!("🔐 PASSWORD: {}", generated_password);
                    tracing::warn!("🔐 Password saved to: {}", password_file);
                    tracing::warn!("🔐 ================================");
                    tracing::warn!(
                        "💡 TIP: You can view the password anytime with: cat {}",
                        password_file
                    );
                    tracing::warn!("💡 TIP: Set SDM_AUTH_PASSWORD environment variable to use a custom password.");
                    tracing::warn!("💡 TIP: Set SDM_PASSWORD_FILE environment variable to use a custom file location.");
                }
            }
            (Some(_), Some(_)) => {
                tracing::info!(
                    "🔐 Using provided password hash for user: {}",
                    self.auth_username
                );
            }
            (None, Some(_)) => {
                tracing::info!(
                    "🔐 Using provided password hash for user: {}",
                    self.auth_username
                );
            }
        }
        Ok(())
    }

    /// Load password from the password file
    fn load_password_from_file() -> Option<String> {
        let password_file = Self::get_password_file_path();
        if Path::new(&password_file).exists() {
            match fs::read_to_string(&password_file) {
                Ok(content) => {
                    let password = content.trim().to_string();
                    if !password.is_empty() {
                        return Some(password);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read password file {}: {}", password_file, e);
                }
            }
        }
        None
    }

    /// Save password to the password file
    fn save_password_to_file(password: &str) -> Result<(), std::io::Error> {
        let password_file = Self::get_password_file_path();

        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&password_file).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&password_file, password)?;

        // Set restrictive permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&password_file)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(&password_file, perms)?;
        }

        tracing::info!(
            "Password saved to {} with secure permissions",
            password_file
        );
        Ok(())
    }

    /// Get the password file path, preferring container-friendly locations
    fn get_password_file_path() -> String {
        // Check for custom path via environment variable
        if let Ok(custom_path) = std::env::var("SDM_PASSWORD_FILE") {
            return custom_path;
        }

        // Container-friendly paths (in order of preference)
        let container_paths = [
            "/data/sdm_password",     // Common data volume mount
            "/config/sdm_password",   // Common config volume mount
            "/app/data/sdm_password", // App-specific data directory
            "/var/lib/sdm/password",  // System-style location
        ];

        // Check if we're likely in a container (common indicators)
        let likely_container = std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
            || std::env::var("DOCKER_CONTAINER").is_ok()
            || Path::new("/.dockerenv").exists()
            || std::env::var("container").is_ok();

        if likely_container {
            // In container: try to use a writable container path
            for path in &container_paths {
                if let Some(parent) = Path::new(path).parent() {
                    // Check if parent directory exists or can be created
                    if parent.exists() || fs::create_dir_all(parent).is_ok() {
                        return path.to_string();
                    }
                }
            }
        }

        // Fallback to current directory (development/local use)
        ".sdm_password".to_string()
    }

    /// Generate a secure random password
    fn generate_password() -> String {
        // Generate a longer, more secure password using two UUIDs
        let uuid1 = uuid::Uuid::new_v4().to_string().replace("-", "");
        let uuid2 = uuid::Uuid::new_v4().to_string().replace("-", "");
        format!("{}{}", &uuid1[..12], &uuid2[..12]) // 24 character password
    }

    /// Hash a password using Argon2
    fn hash_password(password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?;
        Ok(password_hash.to_string())
    }

    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> Result<bool, String> {
        if let Some(hash) = &self.auth_password_hash {
            let parsed_hash = PasswordHash::new(hash)
                .map_err(|e| format!("Failed to parse password hash: {}", e))?;
            let argon2 = Argon2::default();
            Ok(argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok())
        } else {
            Ok(false)
        }
    }

    /// Get the full bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// Default values following 12-Factor principles
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_metrics_interval() -> u64 {
    5
}

fn default_metrics_history() -> usize {
    20
}

fn default_max_chart_containers() -> usize {
    5
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_auth_enabled() -> bool {
    true
}

fn default_auth_username() -> String {
    "admin".to_string()
}

fn default_session_timeout() -> u64 {
    3600
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            docker_socket: None,
            metrics_interval_seconds: default_metrics_interval(),
            metrics_history_limit: default_metrics_history(),
            max_chart_containers: default_max_chart_containers(),
            shutdown_timeout_seconds: default_shutdown_timeout(),
            auth_enabled: default_auth_enabled(),
            auth_username: default_auth_username(),
            auth_password: None,
            auth_password_hash: None,
            session_timeout_seconds: default_session_timeout(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.metrics_interval_seconds, 5);
        assert_eq!(config.metrics_history_limit, 20);
        assert_eq!(config.max_chart_containers, 5);
        assert_eq!(config.shutdown_timeout_seconds, 30);
        assert!(config.auth_enabled);
        assert_eq!(config.auth_username, "admin");
        assert_eq!(config.session_timeout_seconds, 3600);
    }

    #[test]
    fn test_bind_address() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ..Default::default()
        };
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = Config::hash_password(password).expect("Should hash password");

        // Hash should start with Argon2 prefix
        assert!(hash.starts_with("$argon2"));

        // Hash should be different each time due to random salt
        let hash2 = Config::hash_password(password).expect("Should hash password");
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_password_verification() {
        let password = "secure_password_456";
        let hash = Config::hash_password(password).expect("Should hash password");

        let mut config = Config::default();
        config.auth_password_hash = Some(hash);

        // Correct password should verify
        assert!(config.verify_password(password).unwrap());

        // Incorrect password should not verify
        assert!(!config.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_password_verification_without_hash() {
        let config = Config::default();

        // Should return false if no hash is set
        assert!(!config.verify_password("any_password").unwrap());
    }

    #[test]
    fn test_generate_password_length() {
        let password = Config::generate_password();

        // Generated password should be 24 characters
        assert_eq!(password.len(), 24);

        // Should be alphanumeric (hex characters from UUID)
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_password_uniqueness() {
        let password1 = Config::generate_password();
        let password2 = Config::generate_password();

        // Each generated password should be unique
        assert_ne!(password1, password2);
    }

    #[test]
    fn test_get_password_file_path() {
        // Test that the function returns a path
        // (specific path depends on environment, so we just check it's not empty)
        let path = Config::get_password_file_path();
        assert!(!path.is_empty());

        // Should contain "password" in the path
        assert!(path.contains("password") || path.contains("sdm"));
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_host(), "0.0.0.0");
        assert_eq!(default_port(), 3000);
        assert_eq!(default_log_level(), "info");
        assert_eq!(default_metrics_interval(), 5);
        assert_eq!(default_metrics_history(), 20);
        assert_eq!(default_max_chart_containers(), 5);
        assert_eq!(default_shutdown_timeout(), 30);
        assert!(default_auth_enabled());
        assert_eq!(default_auth_username(), "admin");
        assert_eq!(default_session_timeout(), 3600);
    }
}
