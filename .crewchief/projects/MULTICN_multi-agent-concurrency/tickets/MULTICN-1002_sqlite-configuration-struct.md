# Ticket: MULTICN-1002: SQLite Configuration Struct

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a nested `SqliteConfig` struct following the existing configuration pattern (SearchConfig/EmbeddingConfig). Support environment variable overrides for all SQLite settings including pool size, PRAGMA values, and retry configuration.

## Background

Hard-coded SQLite settings from MULTICN-1001 need to be configurable for different environments and use cases. Following the established pattern from `SearchConfig` and `EmbeddingConfig`, we'll create a nested configuration structure with environment variable support.

This enables tuning for different workloads (development vs production, large repos vs small) without code changes.

Reference: [architecture.md](../planning/architecture.md) - Configurable SQLite Settings section

## Acceptance Criteria

- [ ] `SqliteConfig` struct created with nested `PoolConfig`, `PragmaConfig`, `RetryConfig`
- [ ] All config fields support environment variable overrides with `MAPROOM_SQLITE_*` prefix
- [ ] `Default` trait implemented with sensible defaults matching MULTICN-1001 values
- [ ] `from_env()` method parses environment variables correctly
- [ ] `validate()` method rejects invalid values (pool size 0, timeout < 1000ms, etc.)
- [ ] Unit tests verify env var parsing and validation
- [ ] Daemon startup logs show applied configuration values

## Technical Requirements

Create `crates/maproom/src/config/sqlite_config.rs` following existing config pattern.

### Configuration Structure

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub pool: PoolConfig,
    pub pragmas: PragmaConfig,
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_size: u32,              // Default: 10
    pub min_idle: Option<u32>,      // Default: None
    pub connection_timeout_ms: u64, // Default: 30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PragmaConfig {
    pub busy_timeout_ms: u64,       // Default: 30000
    pub wal_autocheckpoint: u32,    // Default: 10000
    pub cache_size_kb: i32,         // Default: 65536 (negative = KB)
    pub mmap_size_bytes: u64,       // Default: 268435456
    pub synchronous: String,        // Default: "NORMAL"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,      // Default: 5
    pub base_delay_ms: u64,     // Default: 50
    pub max_delay_ms: u64,      // Default: 5000
    pub exponential: bool,      // Default: true
}
```

### Environment Variable Mapping

| Environment Variable | Config Field | Default |
|---------------------|--------------|---------|
| `MAPROOM_SQLITE_POOL_SIZE` | pool.max_size | 10 |
| `MAPROOM_SQLITE_MIN_IDLE` | pool.min_idle | None |
| `MAPROOM_SQLITE_TIMEOUT_MS` | pool.connection_timeout_ms | 30000 |
| `MAPROOM_SQLITE_BUSY_TIMEOUT_MS` | pragmas.busy_timeout_ms | 30000 |
| `MAPROOM_SQLITE_WAL_CHECKPOINT` | pragmas.wal_autocheckpoint | 10000 |
| `MAPROOM_SQLITE_CACHE_SIZE_KB` | pragmas.cache_size_kb | 65536 |
| `MAPROOM_SQLITE_MMAP_SIZE` | pragmas.mmap_size_bytes | 268435456 |
| `MAPROOM_SQLITE_SYNCHRONOUS` | pragmas.synchronous | "NORMAL" |
| `MAPROOM_SQLITE_RETRY_ATTEMPTS` | retry.max_attempts | 5 |
| `MAPROOM_SQLITE_RETRY_BASE_MS` | retry.base_delay_ms | 50 |
| `MAPROOM_SQLITE_RETRY_MAX_MS` | retry.max_delay_ms | 5000 |
| `MAPROOM_SQLITE_RETRY_EXPONENTIAL` | retry.exponential | true |

### Default Implementation

```rust
impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            pragmas: PragmaConfig::default(),
            retry: RetryConfig::default(),
        }
    }
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
            cache_size_kb: 65536,
            mmap_size_bytes: 268435456,
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
```

### Environment Variable Parsing

```rust
impl SqliteConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
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

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.pool.max_size == 0 {
            return Err(ConfigError::InvalidPoolSize);
        }
        if self.pragmas.busy_timeout_ms < 1000 {
            return Err(ConfigError::BusyTimeoutTooLow);
        }
        if self.retry.max_attempts == 0 {
            return Err(ConfigError::InvalidRetryConfig);
        }
        // Validate synchronous value
        if !["OFF", "NORMAL", "FULL", "EXTRA"].contains(&self.pragmas.synchronous.as_str()) {
            return Err(ConfigError::InvalidSynchronousValue);
        }
        Ok(())
    }
}
```

### Error Types

```rust
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Pool size must be > 0")]
    InvalidPoolSize,
    #[error("Busy timeout should be >= 1000ms (was {0}ms)")]
    BusyTimeoutTooLow(u64),
    #[error("Retry attempts must be > 0")]
    InvalidRetryConfig,
    #[error("Invalid synchronous value: {0} (expected OFF, NORMAL, FULL, or EXTRA)")]
    InvalidSynchronousValue(String),
}
```

### Helper Functions

```rust
fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_opt<T: FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}
```

## Implementation Notes

### Integration with Connection Pool

Update `crates/maproom/src/db/sqlite/mod.rs` to use `SqliteConfig`:

```rust
use crate::config::sqlite_config::SqliteConfig;

pub fn create_pool(database_path: &Path, config: &SqliteConfig) -> Result<Pool> {
    let pragmas = format!(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = {};
        PRAGMA busy_timeout = {};
        PRAGMA wal_autocheckpoint = {};
        PRAGMA cache_size = {};
        PRAGMA mmap_size = {};
        PRAGMA foreign_keys = ON;
        "#,
        config.pragmas.synchronous,
        config.pragmas.busy_timeout_ms,
        config.pragmas.wal_autocheckpoint,
        -config.pragmas.cache_size_kb, // Negative for KB
        config.pragmas.mmap_size_bytes
    );

    let manager = SqliteConnectionManager::file(database_path)
        .with_init(move |conn| {
            conn.execute_batch(&pragmas)?;
            Ok(())
        });

    let pool = Pool::builder()
        .max_size(config.pool.max_size)
        .min_idle(config.pool.min_idle)
        .connection_timeout(Duration::from_millis(config.pool.connection_timeout_ms))
        .build(manager)?;

    tracing::info!(?config, "SQLite connection pool created");
    Ok(pool)
}
```

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SqliteConfig::default();
        assert_eq!(config.pool.max_size, 10);
        assert_eq!(config.pragmas.busy_timeout_ms, 30000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_from_env() {
        std::env::set_var("MAPROOM_SQLITE_POOL_SIZE", "20");
        std::env::set_var("MAPROOM_SQLITE_BUSY_TIMEOUT_MS", "60000");

        let config = SqliteConfig::from_env().unwrap();
        assert_eq!(config.pool.max_size, 20);
        assert_eq!(config.pragmas.busy_timeout_ms, 60000);
    }

    #[test]
    fn test_validation_rejects_invalid_pool_size() {
        let mut config = SqliteConfig::default();
        config.pool.max_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_rejects_low_timeout() {
        let mut config = SqliteConfig::default();
        config.pragmas.busy_timeout_ms = 500;
        assert!(config.validate().is_err());
    }
}
```

## Dependencies

- MULTICN-1001 (Enhanced PRAGMA Configuration) - builds on those settings

## Risk Assessment

- **Risk**: Breaking changes to existing connection initialization
  - **Mitigation**: Defaults match MULTICN-1001 values exactly. Zero behavior change if no env vars set.

- **Risk**: Environment variable parsing errors
  - **Mitigation**: Comprehensive unit tests. Fallback to defaults on parse failure.

## Files/Packages Affected

- `crates/maproom/src/config/sqlite_config.rs` (NEW)
- `crates/maproom/src/config/mod.rs` (MODIFY - add module export)
- `crates/maproom/src/db/sqlite/mod.rs` (MODIFY - use SqliteConfig)

## Implementation Notes

Implementation completed successfully with the following deliverables:

### Files Created
- `/workspace/crates/maproom/src/config/sqlite_config.rs` - Complete SqliteConfig implementation with nested structs (PoolConfig, PragmaConfig, RetryConfig)

### Files Modified
- `/workspace/crates/maproom/src/config/mod.rs` - Added sqlite_config module and re-exported types
- `/workspace/crates/maproom/src/db/sqlite/mod.rs` - Updated SqliteStore::connect() to use SqliteConfig, added connect_with_config() method

### Configuration Details

**Default Values (matching MULTICN-1001)**:
- Pool: max_size=10, min_idle=None, connection_timeout_ms=30000
- Pragmas: busy_timeout_ms=30000, wal_autocheckpoint=10000, cache_size_kb=65536, mmap_size_bytes=268435456, synchronous=NORMAL
- Retry: max_attempts=5, base_delay_ms=50, max_delay_ms=5000, exponential=true

**Environment Variables** (all with MAPROOM_SQLITE_* prefix):
- MAPROOM_SQLITE_POOL_SIZE
- MAPROOM_SQLITE_MIN_IDLE
- MAPROOM_SQLITE_TIMEOUT_MS
- MAPROOM_SQLITE_BUSY_TIMEOUT_MS
- MAPROOM_SQLITE_WAL_CHECKPOINT
- MAPROOM_SQLITE_CACHE_SIZE_KB
- MAPROOM_SQLITE_MMAP_SIZE
- MAPROOM_SQLITE_SYNCHRONOUS
- MAPROOM_SQLITE_RETRY_ATTEMPTS
- MAPROOM_SQLITE_RETRY_BASE_MS
- MAPROOM_SQLITE_RETRY_MAX_MS
- MAPROOM_SQLITE_RETRY_EXPONENTIAL

**Validation Rules**:
- Pool size must be > 0
- Busy timeout must be >= 1000ms
- Retry attempts must be > 0
- Synchronous must be one of: OFF, NORMAL, FULL, EXTRA (case-insensitive)

**Logging**:
Daemon startup logs all configuration values using structured logging (tracing::info! with fields):
- pool_size, min_idle, connection_timeout_ms
- busy_timeout_ms, wal_autocheckpoint, cache_size_kb, mmap_size_bytes, synchronous
- retry_attempts, retry_base_ms, retry_max_ms, retry_exponential

### Test Coverage

10 unit tests implemented covering:
1. Default configuration validation
2. Environment variable parsing (MAPROOM_SQLITE_*)
3. Validation rejection of invalid pool size
4. Validation rejection of low busy timeout
5. Validation rejection of invalid synchronous values
6. Validation acceptance of all valid synchronous values
7. env_or helper function with defaults and parsing
8. env_opt helper function for optional values
9. min_idle optional field handling
10. Zero retry attempts validation

All tests pass (10/10 passed, 0 failed).

### Integration

The SqliteStore::connect() method now:
1. Loads configuration from environment using SqliteConfig::from_env()
2. Falls back to defaults if env vars not set
3. Validates all configuration before use
4. Applies configuration to pool builder and PRAGMA statements
5. Logs all applied configuration values at startup

Backward compatibility maintained - existing code continues to work with default values if no environment variables are set.

### Build Status

- Compiles cleanly with `cargo build --release -p crewchief-maproom`
- All tests pass with `cargo test -p crewchief-maproom --lib config::sqlite_config`
- No clippy warnings introduced
- Zero-config deployment still works (all defaults sensible)
