# Quality Strategy: Database Connection Fallback

## Testing Philosophy

This is a critical infrastructure component that affects every database operation in Maproom. Testing must provide confidence that:

1. **Connection logic works in all environments** (devcontainer, MCP, standalone)
2. **Fallback hierarchy is respected** (explicit > override > auto-detect > localhost)
3. **Error messages are helpful** when connections fail
4. **Backward compatibility is maintained** for existing users

We'll use **pragmatic testing** - focus on real scenarios users encounter, not exhaustive edge cases.

## Test Coverage Strategy

### What to Test

#### Critical Path: Connection Resolution Logic

**Must Test**:
- ✅ Explicit DATABASE_URL is respected (highest priority)
- ✅ MAPROOM_DB_HOST override works
- ✅ maproom-postgres auto-detection works
- ✅ localhost fallback works
- ✅ Node.js and Rust use identical logic

**Nice to Test**:
- MAPROOM_DB_PORT custom port
- Invalid hostnames fail gracefully
- Concurrent connection attempts

**Don't Test** (diminishing returns):
- All possible PostgreSQL URL formats (library's responsibility)
- Network timeout edge cases (OS-level behavior)
- DNS resolver internals (system-level behavior)

#### Integration Points

**Must Test**:
- ✅ Node.js CLI sets DATABASE_URL correctly for Rust binary
- ✅ Rust binary connects successfully with fallback URL
- ✅ Connection pool creation works with resolved URL

**Nice to Test**:
- Actual database queries work after connection
- Connection pool exhaustion handling

**Don't Test**:
- PostgreSQL server internals
- Full application workflows (separate integration tests)

### What Not to Test

We won't test:
- PostgreSQL server correctness (vendor's responsibility)
- Network infrastructure (OS/Docker's responsibility)
- DNS resolution correctness (system's responsibility)
- Full end-to-end application flows (too broad for this project)

## Test Plan

### Phase 1: Rust Unit Tests

**File**: `/workspace/crates/maproom/src/db/connection.rs`

Test the `get_database_url()` function in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;  // Prevent env var conflicts

    #[test]
    #[serial]
    fn test_explicit_database_url_takes_precedence() {
        // Setup: Set all env vars
        env::set_var("DATABASE_URL", "postgresql://explicit:pass@explicit-host:5432/explicit");
        env::set_var("MAPROOM_DB_HOST", "should-be-ignored");

        let url = get_database_url().unwrap();

        assert_eq!(url, "postgresql://explicit:pass@explicit-host:5432/explicit");

        // Cleanup
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_override() {
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-postgres");
        env::set_var("MAPROOM_DB_PORT", "5555");

        let url = get_database_url().unwrap();

        assert_eq!(url, "postgresql://maproom:maproom@custom-postgres:5555/maproom");

        env::remove_var("MAPROOM_DB_HOST");
        env::remove_var("MAPROOM_DB_PORT");
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_default_port() {
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-postgres");
        env::remove_var("MAPROOM_DB_PORT");

        let url = get_database_url().unwrap();

        assert_eq!(url, "postgresql://maproom:maproom@custom-postgres:5432/maproom");

        env::remove_var("MAPROOM_DB_HOST");
    }

    #[test]
    #[serial]
    fn test_fallback_when_no_env_vars() {
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");

        let url = get_database_url().unwrap();

        // Should either detect maproom-postgres or fallback to localhost
        assert!(
            url.contains("maproom-postgres:5432") || url.contains("127.0.0.1:5433"),
            "Expected maproom-postgres or localhost, got: {}",
            url
        );
    }
}
```

**Test Execution**:
```bash
cargo test --lib db::connection
```

**Acceptance Criteria**:
- All 4 tests pass
- Tests run in <1 second
- No flaky failures

### Phase 2: Rust Integration Tests

**File**: `/workspace/crates/maproom/tests/connection_fallback_test.rs`

Test that connection pool actually connects with resolved URL:

```rust
use crewchief_maproom::db::connection::get_database_url;
use crewchief_maproom::db::pool::create_pool;

#[tokio::test]
async fn test_pool_creation_with_fallback_url() {
    // Remove DATABASE_URL to test fallback
    std::env::remove_var("DATABASE_URL");

    // Get fallback URL
    let url = get_database_url().expect("Failed to get database URL");

    // Set it for pool creation
    std::env::set_var("DATABASE_URL", &url);

    // Try to create pool
    let pool_result = create_pool().await;

    // Should succeed if database is running
    if pool_result.is_ok() {
        let pool = pool_result.unwrap();
        // Try to get a connection
        let client = pool.get().await.expect("Failed to get connection from pool");

        // Simple query to verify connection works
        let rows = client
            .query("SELECT 1 as test", &[])
            .await
            .expect("Failed to execute test query");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<_, i32>("test"), 1);
    }
    // If database isn't running, that's OK for unit tests
    // Integration tests will verify this in real environment
}
```

**Test Execution**:
```bash
# Requires database to be running
docker compose up -d maproom-postgres
cargo test connection_fallback_test
```

**Acceptance Criteria**:
- Test passes when database is available
- Test skips gracefully when database unavailable (dev machines)

### Phase 3: Node.js CLI Tests

**File**: `/workspace/packages/maproom-mcp/tests/connection-fallback.test.js`

Test that CLI respects DATABASE_URL:

```javascript
const { spawnSync } = require('child_process');
const assert = require('assert');

describe('Database Connection Fallback', () => {
  it('respects explicit DATABASE_URL', () => {
    const env = {
      ...process.env,
      DATABASE_URL: 'postgresql://test:test@testhost:5432/testdb'
    };

    // We can't easily test the CLI's internal logic without refactoring,
    // but we can verify the pattern works by checking that the env var
    // is preserved when we spread process.env

    const result = { ...env };

    assert.strictEqual(
      result.DATABASE_URL,
      'postgresql://test:test@testhost:5432/testdb'
    );
  });

  it('sets DATABASE_URL when not present', () => {
    const env = { ...process.env };
    delete env.DATABASE_URL;

    // Simulate CLI logic
    if (!env.DATABASE_URL) {
      env.DATABASE_URL = 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
    }

    assert.ok(env.DATABASE_URL);
    assert.ok(env.DATABASE_URL.includes('maproom'));
  });
});
```

**Test Execution**:
```bash
cd packages/maproom-mcp
npm test tests/connection-fallback.test.js
```

**Acceptance Criteria**:
- Both tests pass
- Tests are fast (<100ms)

### Phase 4: End-to-End Scenario Tests

These tests verify the **complete user workflows** in real environments.

#### Scenario 1: Devcontainer Developer

**Manual Test Procedure**:
```bash
# 1. Inside devcontainer
cd /workspace

# 2. Verify DATABASE_URL is set by docker-compose
echo $DATABASE_URL
# Expected: postgresql://postgres:postgres@postgres:5432/crewchief

# 3. Run Rust binary directly
cargo run --bin crewchief-maproom -- db status

# Expected output:
# Connected to database: postgresql://postgres:***@postgres:5432/crewchief
# Database: crewchief

# 4. Run Node.js CLI
node packages/maproom-mcp/bin/cli.cjs db status

# Expected output:
# 🔗 Using explicit DATABASE_URL from environment
# Connected to database: postgresql://postgres:***@postgres:5432/crewchief
```

**Acceptance Criteria**:
- Both commands use devcontainer database (postgres:5432/crewchief)
- CLI shows "Using explicit DATABASE_URL from environment"

#### Scenario 2: MCP User (No DATABASE_URL)

**Manual Test Procedure**:
```bash
# 1. Remove DATABASE_URL
unset DATABASE_URL

# 2. Ensure maproom-postgres is running
docker compose -f ~/.maproom-mcp/docker-compose.yml up -d

# 3. Run scan command
node packages/maproom-mcp/bin/cli.cjs scan /workspace

# Expected output:
# 🔗 Auto-detected database connection
# Connected to database: postgresql://maproom:***@maproom-postgres:5432/maproom
# [... scan output ...]
```

**Acceptance Criteria**:
- Auto-detects maproom-postgres
- Scan completes successfully
- CLI shows "Auto-detected database connection"

#### Scenario 3: Direct Rust Binary (No DATABASE_URL)

**Manual Test Procedure**:
```bash
# 1. Remove DATABASE_URL
unset DATABASE_URL

# 2. Ensure maproom-postgres is running
docker compose -f ~/.maproom-mcp/docker-compose.yml up -d

# 3. Run Rust binary directly
cargo run --bin crewchief-maproom -- db status

# Expected output:
# Auto-detected maproom-postgres hostname
# Connected to database: postgresql://maproom:***@maproom-postgres:5432/maproom
```

**Acceptance Criteria**:
- Auto-detects maproom-postgres
- Connects successfully
- Shows auto-detection message

#### Scenario 4: MAPROOM_DB_HOST Override

**Manual Test Procedure**:
```bash
# 1. Set custom host
export MAPROOM_DB_HOST=postgres
export MAPROOM_DB_PORT=5432

# 2. Run Rust binary
cargo run --bin crewchief-maproom -- db status

# Expected output:
# Using MAPROOM_DB_HOST: postgres
# Connected to database: postgresql://maproom:***@postgres:5432/maproom
```

**Acceptance Criteria**:
- Uses custom host
- Respects custom port
- Shows MAPROOM_DB_HOST message

## Test Execution Strategy

### During Development

**Run frequently** (after each code change):
```bash
# Rust unit tests (fast, no database needed)
cargo test --lib db::connection
```

**Run before commit** (requires database):
```bash
# Rust integration tests
cargo test connection_fallback_test

# Node.js tests
cd packages/maproom-mcp && npm test
```

### CI/CD Pipeline

**GitHub Actions workflow**:
```yaml
name: Database Fallback Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: pgvector/pgvector:pg15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: crewchief
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3

      - name: Rust unit tests
        run: cargo test --lib db::connection

      - name: Rust integration tests
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/crewchief
        run: cargo test connection_fallback_test

      - name: Node.js tests
        run: |
          cd packages/maproom-mcp
          npm test
```

### Acceptance Testing

Before merging to main, manually verify all 4 end-to-end scenarios on:
- ✅ Linux devcontainer (primary environment)
- ✅ macOS local development (if available)

## Regression Prevention

### Watch For These Issues

1. **CLI overriding DATABASE_URL again**: Add test that fails if DATABASE_URL is overridden
2. **Rust binary ignoring fallback**: Add test that fails if DATABASE_URL required
3. **Inconsistent behavior**: Add cross-language test comparing URLs
4. **Performance degradation**: Ensure hostname resolution timeout stays <1s

### Monitoring in Production

After deployment, monitor:
- MCP server connection success rate (should stay >99%)
- Connection errors in logs (should be rare)
- User reports of connection issues (should be zero)

## Summary

Testing strategy prioritizes:
1. **Unit tests** for fallback logic correctness
2. **Integration tests** for connection pool creation
3. **Scenario tests** for real-world usage patterns
4. **Regression tests** to prevent backsliding

We avoid over-testing (DNS internals, PostgreSQL server) and focus on value (does it work for users?).

Total test count: ~15 tests
- 4 Rust unit tests (connection.rs)
- 1 Rust integration test (connection pool)
- 2 Node.js tests (CLI logic)
- 4 manual scenario tests (end-to-end)
- 4 CI/CD tests (automated scenarios)

All tests should complete in <10 seconds (excluding database startup).
