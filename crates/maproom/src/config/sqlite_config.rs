//! SQLite configuration for connection pooling and PRAGMA settings.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during SQLite configuration.
#[derive(Debug, Error)]
pub enum SqliteConfigError {
    #[error("Pool size must be > 0")]
    InvalidPoolSize,

    #[error("Busy timeout should be >= 1000ms (was {0}ms)")]
    BusyTimeoutTooLow(u64),

    #[error("Retry attempts must be > 0")]
    InvalidRetryConfig,

    #[error("Invalid synchronous value: {0} (expected OFF, NORMAL, FULL, or EXTRA)")]
    InvalidSynchronousValue(String),
}

/// SQLite configuration with nested pool, pragma, and retry settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SqliteConfig {
    pub pool: PoolConfig,
    pub pragmas: PragmaConfig,
    pub retry: RetryConfig,
}

/// Connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: u32,
    /// Minimum number of idle connections (None = no minimum)
    pub min_idle: Option<u32>,
    /// Connection acquisition timeout in milliseconds
    pub connection_timeout_ms: u64,
}

/// SQLite PRAGMA configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PragmaConfig {
    /// busy_timeout PRAGMA value in milliseconds
    pub busy_timeout_ms: u64,
    /// wal_autocheckpoint PRAGMA value (pages before checkpoint)
    pub wal_autocheckpoint: u32,
    /// cache_size PRAGMA value in KB (negative value for KB)
    pub cache_size_kb: i32,
    /// mmap_size PRAGMA value in bytes
    pub mmap_size_bytes: u64,
    /// synchronous PRAGMA value (OFF, NORMAL, FULL, EXTRA)
    pub synchronous: String,
}

/// Retry configuration for transient failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Use exponential backoff (true) or constant delay (false)
    pub exponential: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: None,
            connection_timeout_ms: 30000,
        }
    }
}

impl Default for PragmaConfig {
    fn default() -> Self {
        Self {
            busy_timeout_ms: 30000,
            wal_autocheckpoint: 10000,
            cache_size_kb: 65536,       // 64MB
            mmap_size_bytes: 268435456, // 256MB
            synchronous: "NORMAL".to_string(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 50,
            max_delay_ms: 5000,
            exponential: true,
        }
    }
}

impl SqliteConfig {
    /// Load configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Result<Self, SqliteConfigError> {
        let config = Self {
            pool: PoolConfig {
                max_size: env_or("MAPROOM_SQLITE_POOL_SIZE", 10),
                min_idle: env_opt("MAPROOM_SQLITE_MIN_IDLE"),
                connection_timeout_ms: env_or("MAPROOM_SQLITE_TIMEOUT_MS", 30000),
            },
            pragmas: PragmaConfig {
                busy_timeout_ms: env_or("MAPROOM_SQLITE_BUSY_TIMEOUT_MS", 30000),
                wal_autocheckpoint: env_or("MAPROOM_SQLITE_WAL_CHECKPOINT", 10000),
                cache_size_kb: env_or("MAPROOM_SQLITE_CACHE_SIZE_KB", 65536),
                mmap_size_bytes: env_or("MAPROOM_SQLITE_MMAP_SIZE", 268435456),
                synchronous: env_or("MAPROOM_SQLITE_SYNCHRONOUS", "NORMAL".to_string()),
            },
            retry: RetryConfig {
                max_attempts: env_or("MAPROOM_SQLITE_RETRY_ATTEMPTS", 5),
                base_delay_ms: env_or("MAPROOM_SQLITE_RETRY_BASE_MS", 50),
                max_delay_ms: env_or("MAPROOM_SQLITE_RETRY_MAX_MS", 5000),
                exponential: env_or("MAPROOM_SQLITE_RETRY_EXPONENTIAL", true),
            },
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), SqliteConfigError> {
        // Validate pool configuration
        if self.pool.max_size == 0 {
            return Err(SqliteConfigError::InvalidPoolSize);
        }

        // Validate pragma configuration
        if self.pragmas.busy_timeout_ms < 1000 {
            return Err(SqliteConfigError::BusyTimeoutTooLow(
                self.pragmas.busy_timeout_ms,
            ));
        }

        // Validate synchronous value
        if !["OFF", "NORMAL", "FULL", "EXTRA"]
            .contains(&self.pragmas.synchronous.to_uppercase().as_str())
        {
            return Err(SqliteConfigError::InvalidSynchronousValue(
                self.pragmas.synchronous.clone(),
            ));
        }

        // Validate retry configuration
        if self.retry.max_attempts == 0 {
            return Err(SqliteConfigError::InvalidRetryConfig);
        }

        Ok(())
    }
}

/// Parse environment variable or return default value.
fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Parse optional environment variable.
fn env_opt<T: FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SqliteConfig::default();
        assert_eq!(config.pool.max_size, 10);
        assert_eq!(config.pool.min_idle, None);
        assert_eq!(config.pool.connection_timeout_ms, 30000);
        assert_eq!(config.pragmas.busy_timeout_ms, 30000);
        assert_eq!(config.pragmas.wal_autocheckpoint, 10000);
        assert_eq!(config.pragmas.cache_size_kb, 65536);
        assert_eq!(config.pragmas.mmap_size_bytes, 268435456);
        assert_eq!(config.pragmas.synchronous, "NORMAL");
        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.retry.base_delay_ms, 50);
        assert_eq!(config.retry.max_delay_ms, 5000);
        assert!(config.retry.exponential);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_from_env() {
        // Save original env vars to restore later
        let original_pool_size = std::env::var("MAPROOM_SQLITE_POOL_SIZE").ok();
        let original_busy_timeout = std::env::var("MAPROOM_SQLITE_BUSY_TIMEOUT_MS").ok();
        let original_synchronous = std::env::var("MAPROOM_SQLITE_SYNCHRONOUS").ok();

        // Set test values
        std::env::set_var("MAPROOM_SQLITE_POOL_SIZE", "20");
        std::env::set_var("MAPROOM_SQLITE_BUSY_TIMEOUT_MS", "60000");
        std::env::set_var("MAPROOM_SQLITE_SYNCHRONOUS", "FULL");

        let config = SqliteConfig::from_env().unwrap();
        assert_eq!(config.pool.max_size, 20);
        assert_eq!(config.pragmas.busy_timeout_ms, 60000);
        assert_eq!(config.pragmas.synchronous, "FULL");

        // Restore original env vars
        if let Some(val) = original_pool_size {
            std::env::set_var("MAPROOM_SQLITE_POOL_SIZE", val);
        } else {
            std::env::remove_var("MAPROOM_SQLITE_POOL_SIZE");
        }
        if let Some(val) = original_busy_timeout {
            std::env::set_var("MAPROOM_SQLITE_BUSY_TIMEOUT_MS", val);
        } else {
            std::env::remove_var("MAPROOM_SQLITE_BUSY_TIMEOUT_MS");
        }
        if let Some(val) = original_synchronous {
            std::env::set_var("MAPROOM_SQLITE_SYNCHRONOUS", val);
        } else {
            std::env::remove_var("MAPROOM_SQLITE_SYNCHRONOUS");
        }
    }

    #[test]
    fn test_validation_rejects_invalid_pool_size() {
        let mut config = SqliteConfig::default();
        config.pool.max_size = 0;
        assert!(matches!(
            config.validate(),
            Err(SqliteConfigError::InvalidPoolSize)
        ));
    }

    #[test]
    fn test_validation_rejects_low_timeout() {
        let mut config = SqliteConfig::default();
        config.pragmas.busy_timeout_ms = 500;
        let result = config.validate();
        assert!(matches!(
            result,
            Err(SqliteConfigError::BusyTimeoutTooLow(500))
        ));
    }

    #[test]
    fn test_validation_rejects_invalid_synchronous() {
        let mut config = SqliteConfig::default();
        config.pragmas.synchronous = "INVALID".to_string();
        let result = config.validate();
        assert!(matches!(
            result,
            Err(SqliteConfigError::InvalidSynchronousValue(_))
        ));
    }

    #[test]
    fn test_validation_accepts_valid_synchronous_values() {
        for value in &["OFF", "NORMAL", "FULL", "EXTRA", "normal", "full"] {
            let mut config = SqliteConfig::default();
            config.pragmas.synchronous = value.to_string();
            assert!(
                config.validate().is_ok(),
                "Failed to validate synchronous value: {}",
                value
            );
        }
    }

    #[test]
    fn test_validation_rejects_zero_retry_attempts() {
        let mut config = SqliteConfig::default();
        config.retry.max_attempts = 0;
        assert!(matches!(
            config.validate(),
            Err(SqliteConfigError::InvalidRetryConfig)
        ));
    }

    #[test]
    fn test_env_or_parsing() {
        // Test with unset variable (should use default)
        std::env::remove_var("TEST_UNSET_VAR");
        assert_eq!(env_or("TEST_UNSET_VAR", 42), 42);

        // Test with set variable
        std::env::set_var("TEST_SET_VAR", "100");
        assert_eq!(env_or("TEST_SET_VAR", 42), 100);
        std::env::remove_var("TEST_SET_VAR");

        // Test with invalid value (should fall back to default)
        std::env::set_var("TEST_INVALID_VAR", "not_a_number");
        assert_eq!(env_or::<i32>("TEST_INVALID_VAR", 42), 42);
        std::env::remove_var("TEST_INVALID_VAR");
    }

    #[test]
    fn test_env_opt_parsing() {
        // Test with unset variable
        std::env::remove_var("TEST_OPT_UNSET");
        assert_eq!(env_opt::<u32>("TEST_OPT_UNSET"), None);

        // Test with set variable
        std::env::set_var("TEST_OPT_SET", "100");
        assert_eq!(env_opt::<u32>("TEST_OPT_SET"), Some(100));
        std::env::remove_var("TEST_OPT_SET");

        // Test with invalid value
        std::env::set_var("TEST_OPT_INVALID", "not_a_number");
        assert_eq!(env_opt::<u32>("TEST_OPT_INVALID"), None);
        std::env::remove_var("TEST_OPT_INVALID");
    }

    #[test]
    fn test_min_idle_optional() {
        let config = SqliteConfig::default();
        assert_eq!(config.pool.min_idle, None);

        // Test env var parsing
        let original = std::env::var("MAPROOM_SQLITE_MIN_IDLE").ok();
        std::env::set_var("MAPROOM_SQLITE_MIN_IDLE", "5");
        let config = SqliteConfig::from_env().unwrap();
        assert_eq!(config.pool.min_idle, Some(5));

        // Restore
        if let Some(val) = original {
            std::env::set_var("MAPROOM_SQLITE_MIN_IDLE", val);
        } else {
            std::env::remove_var("MAPROOM_SQLITE_MIN_IDLE");
        }
    }
}
