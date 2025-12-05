# Ticket: MULTICN-1003: Write Retry Logic

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

Implement `write_with_retry()` wrapper that automatically retries write operations on SQLITE_BUSY errors with exponential backoff. This provides application-level retry logic as a defensive measure against transient lock contention.

## Background

Even with enhanced PRAGMAs (MULTICN-1001) and configuration (MULTICN-1002), SQLITE_BUSY errors can still occur under heavy concurrent load. Automatic retry with exponential backoff is a battle-tested pattern for handling transient database lock contention.

This retry logic will be useful in both current (per-agent daemon) and future (shared daemon) architectures as a defense-in-depth measure.

Reference: [architecture.md](../planning/architecture.md) - Retry Logic for Writes section

## Acceptance Criteria

- [ ] `write_with_retry()` method exists in SqliteStore or connection pool wrapper
- [ ] Implements exponential backoff: 50ms, 100ms, 200ms, 400ms, 800ms (5 attempts)
- [ ] Catches SQLITE_BUSY and rusqlite DatabaseLocked errors
- [ ] Logs warning on each retry with attempt number and delay
- [ ] Returns error after max attempts (doesn't retry forever)
- [ ] All write operations use `write_with_retry()` instead of direct writes
- [ ] Unit tests verify retry behavior and exponential backoff timing
- [ ] Integration test shows successful retry on simulated SQLITE_BUSY

## Technical Requirements

Implement retry logic in `crates/maproom/src/db/sqlite/mod.rs`.

### Retry Configuration

Use `RetryConfig` from MULTICN-1002:

```rust
pub struct RetryConfig {
    pub max_attempts: u32,      // Default: 5
    pub base_delay_ms: u64,     // Default: 50
    pub max_delay_ms: u64,      // Default: 5000
    pub exponential: bool,      // Default: true
}
```

### Write Retry Wrapper

```rust
use rusqlite::{Error as RusqliteError, ErrorCode};
use tokio::time::{sleep, Duration};

impl SqliteStore {
    /// Execute a write operation with automatic retry on SQLITE_BUSY errors.
    ///
    /// Uses exponential backoff: 50ms → 100ms → 200ms → 400ms → 800ms
    /// Logs warnings for observability of contention issues.
    pub async fn write_with_retry<F, T>(&self, mut op: F) -> Result<T, StoreError>
    where
        F: FnMut(&mut rusqlite::Connection) -> Result<T, RusqliteError>,
    {
        let config = &self.config.retry;
        let mut delay_ms = config.base_delay_ms;

        for attempt in 1..=config.max_attempts {
            match self.execute_write(&mut op) {
                Ok(result) => {
                    if attempt > 1 {
                        tracing::info!(
                            attempt,
                            "Write succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(e) if is_busy_error(&e) && attempt < config.max_attempts => {
                    tracing::warn!(
                        attempt,
                        total_attempts = config.max_attempts,
                        delay_ms,
                        error = %e,
                        "SQLITE_BUSY error, retrying"
                    );

                    sleep(Duration::from_millis(delay_ms)).await;

                    // Exponential backoff
                    if config.exponential {
                        delay_ms = (delay_ms * 2).min(config.max_delay_ms);
                    }
                }
                Err(e) => {
                    if is_busy_error(&e) {
                        tracing::error!(
                            max_attempts = config.max_attempts,
                            "Write failed after all retry attempts"
                        );
                    }
                    return Err(e.into());
                }
            }
        }

        unreachable!("Loop should always return or error")
    }

    fn execute_write<F, T>(&self, op: F) -> Result<T, RusqliteError>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<T, RusqliteError>,
    {
        let mut conn = self.pool.get()
            .map_err(|e| RusqliteError::SqliteFailure(
                rusqlite::ffi::Error::new(ErrorCode::CannotOpen),
                Some(format!("Failed to get connection from pool: {}", e))
            ))?;

        op(&mut conn)
    }
}

/// Check if error is a transient lock contention error that should be retried
fn is_busy_error(error: &RusqliteError) -> bool {
    match error {
        RusqliteError::SqliteFailure(err, _) => {
            matches!(
                err.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            )
        }
        _ => false,
    }
}
```

### Applying to Write Operations

Update all write operations to use retry wrapper:

```rust
// Before
pub fn insert_chunk(&self, chunk: &Chunk) -> Result<()> {
    let mut conn = self.pool.get()?;
    conn.execute(
        "INSERT INTO chunks (...) VALUES (...)",
        params![...],
    )?;
    Ok(())
}

// After
pub async fn insert_chunk(&self, chunk: &Chunk) -> Result<()> {
    self.write_with_retry(|conn| {
        conn.execute(
            "INSERT INTO chunks (...) VALUES (...)",
            params![...],
        )?;
        Ok(())
    }).await
}
```

### Identifying Write Operations

Search for write operations to convert:
- `INSERT` statements
- `UPDATE` statements
- `DELETE` statements
- Transaction commits
- Schema migrations

Read operations do NOT need retry (SQLITE_BUSY only occurs on writes).

## Implementation Notes

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_write_retry_with_eventual_success() {
        let store = create_test_store();
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = store.write_with_retry(|_conn| {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;

            // Fail first 2 attempts, succeed on 3rd
            if *count < 3 {
                Err(RusqliteError::SqliteFailure(
                    rusqlite::ffi::Error::new(ErrorCode::DatabaseBusy),
                    Some("simulated busy".into())
                ))
            } else {
                Ok(42)
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_write_retry_fails_after_max_attempts() {
        let store = create_test_store();
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = store.write_with_retry(|_conn| {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;

            // Always fail
            Err(RusqliteError::SqliteFailure(
                rusqlite::ffi::Error::new(ErrorCode::DatabaseBusy),
                Some("always busy".into())
            ))
        }).await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 5); // Max attempts
    }

    #[tokio::test]
    async fn test_exponential_backoff_timing() {
        let store = create_test_store();
        let start = std::time::Instant::now();

        let _ = store.write_with_retry(|_conn| {
            Err(RusqliteError::SqliteFailure(
                rusqlite::ffi::Error::new(ErrorCode::DatabaseBusy),
                None
            ))
        }).await;

        let elapsed = start.elapsed();
        // 50 + 100 + 200 + 400 = 750ms minimum
        assert!(elapsed >= Duration::from_millis(750));
        // Allow some overhead
        assert!(elapsed < Duration::from_millis(1500));
    }

    #[test]
    fn test_is_busy_error_detection() {
        let busy = RusqliteError::SqliteFailure(
            rusqlite::ffi::Error::new(ErrorCode::DatabaseBusy),
            None
        );
        assert!(is_busy_error(&busy));

        let locked = RusqliteError::SqliteFailure(
            rusqlite::ffi::Error::new(ErrorCode::DatabaseLocked),
            None
        );
        assert!(is_busy_error(&locked));

        let other = RusqliteError::SqliteFailure(
            rusqlite::ffi::Error::new(ErrorCode::ConstraintViolation),
            None
        );
        assert!(!is_busy_error(&other));
    }
}
```

### Integration Test

```rust
// tests/integration/sqlite_retry.rs

#[tokio::test]
async fn test_concurrent_writes_with_retry() {
    let db_path = create_temp_database();
    let store = SqliteStore::new(&db_path, SqliteConfig::default()).unwrap();

    // Spawn 5 concurrent writers
    let mut handles = vec![];
    for i in 0..5 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            for j in 0..20 {
                let chunk_id = format!("chunk-{}-{}", i, j);
                store.insert_test_chunk(&chunk_id).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all writes succeeded
    let count = store.count_test_chunks().await.unwrap();
    assert_eq!(count, 100); // 5 writers * 20 chunks
}
```

## Dependencies

- MULTICN-1002 (SQLite Configuration Struct) - uses RetryConfig

## Risk Assessment

- **Risk**: Exponential backoff could delay writes significantly under heavy load
  - **Mitigation**: Total delay for 5 attempts is ~1.5s, which is reasonable. Enhanced PRAGMAs (MULTICN-1001) should prevent most BUSY errors.

- **Risk**: Converting synchronous writes to async changes API signatures
  - **Mitigation**: Acceptable for maproom daemon which already uses tokio. Client code may need minor adjustments.

- **Risk**: Retry logic masks underlying lock contention issues
  - **Mitigation**: Logging at WARN level ensures contention is visible. Metrics should track retry frequency.

## Files/Packages Affected

- `crates/maproom/src/db/sqlite/mod.rs` (MODIFY - add retry logic)
- `crates/maproom/src/db/sqlite/vector.rs` (MODIFY - update write methods)
- `crates/maproom/src/db/sqlite/chunks.rs` (MODIFY - update write methods)
- `tests/integration/sqlite_retry.rs` (NEW - integration tests)
