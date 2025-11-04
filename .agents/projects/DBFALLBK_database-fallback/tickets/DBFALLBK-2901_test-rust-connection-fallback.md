# Ticket: DBFALLBK-2901: Test Rust Connection Fallback Logic

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Write comprehensive unit and integration tests for the Rust connection fallback logic implemented in DBFALLBK-2001 to verify all fallback scenarios work correctly.

## Background
After implementing the connection fallback logic in DBFALLBK-2001, we need comprehensive tests to ensure:
- All fallback scenarios work correctly (explicit DATABASE_URL, MAPROOM_DB_HOST, hostname auto-detect, localhost fallback)
- Connection pool creation works with resolved URLs
- No regressions in existing database functionality

This implements the testing strategy from planning/quality-strategy.md Phase 1 (Rust Unit Tests) and Phase 2 (Rust Integration Tests).

## Acceptance Criteria
- [x] 4 unit tests in connection.rs all pass
- [x] 1 integration test in connection_fallback_test.rs passes
- [x] All existing database tests still pass (no regressions)
- [x] Tests complete in less than 10 seconds total
- [x] `cargo test --lib db::connection` succeeds
- [x] `cargo test connection_fallback_test` succeeds

## Technical Requirements
Write 5 tests total:

**Unit tests in crates/maproom/src/db/connection.rs:**
1. `test_explicit_database_url_takes_precedence` - Verify DATABASE_URL overrides MAPROOM_DB_HOST
2. `test_maproom_db_host_override` - Verify MAPROOM_DB_HOST and MAPROOM_DB_PORT work
3. `test_maproom_db_host_default_port` - Verify default port 5432 when MAPROOM_DB_PORT not set
4. `test_fallback_when_no_env_vars` - Verify maproom-postgres or localhost fallback

**Integration test in crates/maproom/tests/connection_fallback_test.rs:**
5. `test_pool_creation_with_fallback_url` - Verify connection pool works with fallback URL

Use `serial_test` crate to prevent environment variable conflicts between tests running in parallel.

## Implementation Notes

### Unit Tests Module Structure

Add to `crates/maproom/src/db/connection.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_explicit_database_url_takes_precedence() {
        // Set both DATABASE_URL and MAPROOM_DB_HOST
        env::set_var("DATABASE_URL", "postgresql://explicit:explicit@explicit:5432/explicit");
        env::set_var("MAPROOM_DB_HOST", "should-not-use-this");

        let result = get_database_url();

        // Clean up
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "postgresql://explicit:explicit@explicit:5432/explicit"
        );
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_override() {
        // Remove DATABASE_URL, set MAPROOM_DB_HOST and MAPROOM_DB_PORT
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-host");
        env::set_var("MAPROOM_DB_PORT", "5555");

        let result = get_database_url();

        // Clean up
        env::remove_var("MAPROOM_DB_HOST");
        env::remove_var("MAPROOM_DB_PORT");

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "postgresql://maproom:maproom@custom-host:5555/maproom"
        );
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_default_port() {
        // Remove DATABASE_URL and MAPROOM_DB_PORT, set only MAPROOM_DB_HOST
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_PORT");
        env::set_var("MAPROOM_DB_HOST", "custom-host");

        let result = get_database_url();

        // Clean up
        env::remove_var("MAPROOM_DB_HOST");

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "postgresql://maproom:maproom@custom-host:5432/maproom"
        );
    }

    #[test]
    #[serial]
    fn test_fallback_when_no_env_vars() {
        // Remove all environment variables
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");
        env::remove_var("MAPROOM_DB_PORT");

        let result = get_database_url();

        assert!(result.is_ok());
        let url = result.unwrap();

        // Should fall back to either maproom-postgres or localhost
        assert!(
            url == "postgresql://maproom:maproom@maproom-postgres:5432/maproom"
            || url == "postgresql://maproom:maproom@127.0.0.1:5433/maproom",
            "Expected maproom-postgres or localhost fallback, got: {}",
            url
        );
    }
}
```

### Integration Test Structure

Create `crates/maproom/tests/connection_fallback_test.rs`:

```rust
use maproom::db::connection::get_database_url;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_pool_creation_with_fallback_url() {
    // Get the database URL using the fallback logic
    let database_url = get_database_url()
        .expect("Should be able to determine database URL");

    // Try to create a connection pool with the fallback URL
    let pool_result = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await;

    // If database is running, connection should succeed
    // If not running, we just verify the URL is valid format
    match pool_result {
        Ok(pool) => {
            // Verify we can execute a simple query
            let result = sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&pool)
                .await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1);
        }
        Err(e) => {
            // Database not running - just verify URL format is valid
            assert!(
                database_url.starts_with("postgresql://"),
                "Database URL should start with postgresql://, got: {}",
                database_url
            );

            println!("Note: Database not running, skipping connection test");
            println!("Error was: {}", e);
        }
    }
}
```

### Cargo.toml Updates

Ensure `serial_test` is added to dev-dependencies in `crates/maproom/Cargo.toml`:

```toml
[dev-dependencies]
serial_test = "3.0"
```

### Testing Commands

```bash
# Run unit tests only
cargo test --lib db::connection

# Run integration test only
cargo test connection_fallback_test

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Testing Strategy
1. Unit tests verify each fallback scenario in isolation
2. Integration test verifies connection pool creation works with resolved URL
3. Use `serial_test::serial` attribute to prevent parallel execution conflicts with environment variables
4. Integration test gracefully skips connection verification if database is unavailable
5. All existing database tests should still pass to ensure no regressions

## Dependencies
- DBFALLBK-2001 must be complete (connection.rs module must exist)

## Risk Assessment
- **Risk**: Tests might be flaky due to environment variable state between test runs
  - **Mitigation**: Use `serial_test` crate to run tests sequentially, clean up environment variables after each test

- **Risk**: Integration test might fail if database isn't running
  - **Mitigation**: Test gracefully skips actual connection verification when database is unavailable, only validates URL format

- **Risk**: Hostname resolution tests might behave differently on different systems
  - **Mitigation**: `test_fallback_when_no_env_vars` accepts either maproom-postgres or localhost as valid results

- **Risk**: Tests might interfere with developer's environment variables
  - **Mitigation**: Tests explicitly remove environment variables after execution in cleanup code

## Files/Packages Affected
- `/workspace/crates/maproom/src/db/connection.rs` - Add `#[cfg(test)] mod tests` section
- `/workspace/crates/maproom/tests/connection_fallback_test.rs` - Create new integration test file
- `/workspace/crates/maproom/Cargo.toml` - Add `serial_test` to dev-dependencies (if not already present)
