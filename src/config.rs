//! Centralized configuration management for fast10k

use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Result, Context};

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the SQLite database file
    pub database_path: PathBuf,
    /// Directory for downloaded documents
    pub download_dir: PathBuf,
    /// EDINET API key (optional)
    pub edinet_api_key: Option<String>,
    /// Rate limiting configuration
    pub rate_limits: RateLimits,
    /// HTTP client configuration
    pub http: HttpConfig,
}

/// Rate limiting configuration for different APIs
#[derive(Debug, Clone)]
pub struct RateLimits {
    /// Delay between EDINET API calls (milliseconds)
    pub edinet_api_delay_ms: u64,
    /// Delay between EDINET document downloads (milliseconds)
    pub edinet_download_delay_ms: u64,
    /// Delay between EDGAR API calls (milliseconds)
    pub edgar_api_delay_ms: u64,
}

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// User agent string
    pub user_agent: String,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            edinet_api_delay_ms: 100,
            edinet_download_delay_ms: 200,
            edgar_api_delay_ms: 100,
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            user_agent: "fast10k/0.1.0".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables and defaults
    pub fn from_env() -> Result<Self> {
        let database_path = std::env::var("FAST10K_DB_PATH")
            .unwrap_or_else(|_| "./fast10k.db".to_string())
            .into();

        let download_dir = std::env::var("FAST10K_DOWNLOAD_DIR")
            .unwrap_or_else(|_| "./downloads".to_string())
            .into();

        let edinet_api_key = std::env::var("EDINET_API_KEY").ok();

        let rate_limits = RateLimits {
            edinet_api_delay_ms: parse_env_var("FAST10K_EDINET_API_DELAY_MS")?.unwrap_or(100),
            edinet_download_delay_ms: parse_env_var("FAST10K_EDINET_DOWNLOAD_DELAY_MS")?.unwrap_or(200),
            edgar_api_delay_ms: parse_env_var("FAST10K_EDGAR_API_DELAY_MS")?.unwrap_or(100),
        };

        let http = HttpConfig {
            timeout_seconds: parse_env_var("FAST10K_HTTP_TIMEOUT_SECONDS")?.unwrap_or(30),
            user_agent: std::env::var("FAST10K_USER_AGENT")
                .unwrap_or_else(|_| "fast10k/0.1.0".to_string()),
        };

        Ok(Config {
            database_path,
            download_dir,
            edinet_api_key,
            rate_limits,
            http,
        })
    }

    /// Get database path as string
    pub fn database_path_str(&self) -> &str {
        self.database_path.to_str().unwrap_or("./fast10k.db")
    }

    /// Get download directory as string
    pub fn download_dir_str(&self) -> &str {
        self.download_dir.to_str().unwrap_or("./downloads")
    }

    /// Get EDINET API delay as Duration
    pub fn edinet_api_delay(&self) -> Duration {
        Duration::from_millis(self.rate_limits.edinet_api_delay_ms)
    }

    /// Get EDINET download delay as Duration
    pub fn edinet_download_delay(&self) -> Duration {
        Duration::from_millis(self.rate_limits.edinet_download_delay_ms)
    }

    /// Get HTTP timeout as Duration
    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http.timeout_seconds)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check if parent directory of database exists
        if let Some(parent) = self.database_path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Database parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        // Check if download directory can be created
        std::fs::create_dir_all(&self.download_dir)
            .with_context(|| format!("Cannot create download directory: {}", self.download_dir.display()))?;

        Ok(())
    }
}

/// Helper function to parse environment variable as a specific type
fn parse_env_var<T>(var_name: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display + Send + Sync + std::error::Error + 'static,
{
    match std::env::var(var_name) {
        Ok(val) => val.parse().map(Some).with_context(|| {
            format!("Failed to parse environment variable {} = '{}'", var_name, val)
        }),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::from_env().unwrap();
        assert_eq!(config.database_path_str(), "./fast10k.db");
        assert_eq!(config.download_dir_str(), "./downloads");
        assert_eq!(config.rate_limits.edinet_api_delay_ms, 100);
        assert_eq!(config.http.timeout_seconds, 30);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::from_env().unwrap();
        // Should not fail for default paths
        config.validate().unwrap();
    }
}