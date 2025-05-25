use serde::Deserialize;

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
    
    /// Graceful shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_seconds: u64,
}

impl Config {
    /// Load configuration from environment variables with .env file support
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Load .env file if present (ignored if not found)
        let _ = dotenvy::dotenv();
        
        // Use envy to deserialize from environment variables with SDM_ prefix
        let config = envy::prefixed("SDM_").from_env::<Config>()?;
        
        tracing::info!("Configuration loaded: {:#?}", config);
        Ok(config)
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

fn default_shutdown_timeout() -> u64 {
    30
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
            shutdown_timeout_seconds: default_shutdown_timeout(),
        }
    }
} 