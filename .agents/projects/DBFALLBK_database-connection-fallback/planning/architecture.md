# Architecture: Database Connection Fallback

## Design Principles

1. **Explicit Over Implicit**: Respect explicit `DATABASE_URL` configuration first
2. **Consistent Behavior**: Node.js CLI and Rust binary use identical logic
3. **Smart Defaults**: Auto-detect `maproom-postgres` for typical users
4. **Fail Fast**: Clear errors when all fallbacks exhausted
5. **Backward Compatible**: Don't break existing usage patterns

## Proposed Solution

### Unified Fallback Hierarchy

Both Node.js CLI and Rust binary will use this exact priority order:

```
1. DATABASE_URL environment variable (if set)
   → Use as-is, no modification

2. MAPROOM_DB_HOST environment variable (if set)
   → Build connection string: postgresql://maproom:maproom@${MAPROOM_DB_HOST}:${MAPROOM_DB_PORT:-5432}/maproom

3. maproom-postgres hostname resolution (if resolves)
   → Use: postgresql://maproom:maproom@maproom-postgres:5432/maproom

4. Localhost fallback
   → Use: postgresql://maproom:maproom@127.0.0.1:5433/maproom

5. Error
   → If even localhost fails to connect, provide helpful error message
```

### Implementation Details

#### Node.js CLI Changes

**File**: `/workspace/packages/maproom-mcp/bin/cli.cjs`

**Current code** (line 1522-1526):
```javascript
const env = {
  ...process.env,
  DATABASE_URL: getDatabaseConnectionString(),  // Always overrides
  ...providerEnv
};
```

**New code**:
```javascript
const env = {
  ...process.env,
  ...providerEnv
};

// Only set DATABASE_URL if not already set
if (!env.DATABASE_URL) {
  env.DATABASE_URL = getDatabaseConnectionString();
}
```

**Rationale**: This preserves explicit `DATABASE_URL` from environment (e.g., from docker-compose.yml) while still providing auto-detection when not set.

#### Rust Binary Changes

**New Module**: `/workspace/crates/maproom/src/db/connection.rs`

Create a new module to handle connection string resolution:

```rust
use anyhow::{Context, Result};
use std::env;
use std::process::Command;
use tracing::{debug, warn};

/// Get database connection URL with fallback logic.
///
/// Priority order:
/// 1. DATABASE_URL env var (explicit config)
/// 2. MAPROOM_DB_HOST env var (component-based config)
/// 3. maproom-postgres hostname resolution (auto-detect)
/// 4. localhost fallback (development)
pub fn get_database_url() -> Result<String> {
    // 1. Check for explicit DATABASE_URL
    if let Ok(url) = env::var("DATABASE_URL") {
        debug!("Using explicit DATABASE_URL from environment");
        return Ok(url);
    }

    // 2. Check for MAPROOM_DB_HOST component-based config
    if let Ok(host) = env::var("MAPROOM_DB_HOST") {
        let port = env::var("MAPROOM_DB_PORT").unwrap_or_else(|_| "5432".to_string());
        let url = format!("postgresql://maproom:maproom@{}:{}/maproom", host, port);
        debug!("Using MAPROOM_DB_HOST: {}", host);
        return Ok(url);
    }

    // 3. Try to resolve maproom-postgres hostname
    if can_resolve_hostname("maproom-postgres") {
        let url = "postgresql://maproom:maproom@maproom-postgres:5432/maproom".to_string();
        debug!("Auto-detected maproom-postgres hostname");
        return Ok(url);
    }

    // 4. Fallback to localhost
    warn!("maproom-postgres hostname not found, falling back to localhost:5433");
    Ok("postgresql://maproom:maproom@127.0.0.1:5433/maproom".to_string())
}

/// Check if a hostname can be resolved via DNS.
///
/// Uses `getent hosts` on Linux/Unix, `ping` as fallback.
/// Times out after 1 second.
fn can_resolve_hostname(hostname: &str) -> bool {
    // Try getent hosts first (works on Linux)
    let getent_result = Command::new("getent")
        .args(&["hosts", hostname])
        .output();

    if let Ok(output) = getent_result {
        if output.status.success() {
            return true;
        }
    }

    // Fallback: try ping with 1 packet, 1 second timeout
    let ping_result = Command::new("ping")
        .args(&["-c", "1", "-W", "1", hostname])
        .output();

    if let Ok(output) = ping_result {
        return output.status.success();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_database_url() {
        env::set_var("DATABASE_URL", "postgresql://test:test@testhost:5432/testdb");

        let url = get_database_url().unwrap();

        assert_eq!(url, "postgresql://test:test@testhost:5432/testdb");
        env::remove_var("DATABASE_URL");
    }

    #[test]
    fn test_maproom_db_host() {
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-host");
        env::set_var("MAPROOM_DB_PORT", "5555");

        let url = get_database_url().unwrap();

        assert_eq!(url, "postgresql://maproom:maproom@custom-host:5555/maproom");
        env::remove_var("MAPROOM_DB_HOST");
        env::remove_var("MAPROOM_DB_PORT");
    }

    #[test]
    fn test_localhost_fallback() {
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");

        let url = get_database_url().unwrap();

        // Should either detect maproom-postgres or fallback to localhost
        assert!(
            url.contains("maproom-postgres") || url.contains("127.0.0.1:5433"),
            "Expected maproom-postgres or localhost, got: {}",
            url
        );
    }
}
```

**File**: `/workspace/crates/maproom/src/db/pool.rs`

**Current code** (lines 118-120):
```rust
pub async fn create_pool() -> anyhow::Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required (tip: use a .env file)")?;
```

**New code**:
```rust
use crate::db::connection::get_database_url;

pub async fn create_pool() -> anyhow::Result<PgPool> {
    let database_url = get_database_url()
        .context("Failed to determine database connection URL")?;
```

**File**: `/workspace/crates/maproom/src/db/queries.rs`

**Current code** (lines 6-7):
```rust
pub async fn connect() -> anyhow::Result<Client> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required (tip: use a .env file)")?;
```

**New code**:
```rust
use crate::db::connection::get_database_url;

pub async fn connect() -> anyhow::Result<Client> {
    let database_url = get_database_url()
        .context("Failed to determine database connection URL")?;
```

**File**: `/workspace/crates/maproom/src/db/mod.rs`

Add new module export:
```rust
pub mod connection;
pub mod pool;
pub mod queries;
// ... existing exports
```

## Error Handling

### When All Fallbacks Fail

If even the localhost fallback fails to connect, provide a comprehensive error message:

```rust
Error: Failed to connect to database

Attempted connection strategies:
  1. DATABASE_URL env var: not set
  2. MAPROOM_DB_HOST env var: not set
  3. maproom-postgres hostname: not found
  4. localhost:5433: connection refused

Troubleshooting:
  - Set DATABASE_URL explicitly: export DATABASE_URL="postgresql://user:pass@host:port/db"
  - Start maproom-postgres container: docker compose up -d maproom-postgres
  - Check PostgreSQL is running: docker ps | grep postgres
  - Verify network connectivity: ping maproom-postgres

For development, see: docs/architecture/DATABASE_ARCHITECTURE.md
```

### Connection Pool Errors

The existing error handling in `pool.rs` (lines 170-194) already provides good context. We'll enhance it to mention the fallback logic:

```rust
let client = pool.get().await.map_err(|e| {
    let sanitized_url = sanitize_database_url(&database_url);

    let mut error_msg = format!(
        "Failed to connect to database\n  Resolved URL: {}\n  Error: {}",
        sanitized_url, e
    );

    error_msg.push_str("\n\n  Connection was determined via fallback logic:");
    error_msg.push_str("\n  - DATABASE_URL env var");
    error_msg.push_str("\n  - MAPROOM_DB_HOST env var");
    error_msg.push_str("\n  - maproom-postgres hostname auto-detection");
    error_msg.push_str("\n  - localhost fallback");

    // ... rest of existing error message

    anyhow::anyhow!(error_msg)
})?;
```

## Logging Strategy

Add debug logging to track which connection method was used:

**Rust**:
```rust
use tracing::{debug, info};

debug!("Database connection method: {}", method);
info!("Connected to database: {}", sanitized_url);
```

**Node.js**:
```javascript
if (!env.DATABASE_URL) {
  env.DATABASE_URL = getDatabaseConnectionString();
  console.error('🔗 Auto-detected database connection');
} else {
  console.error('🔗 Using explicit DATABASE_URL from environment');
}
```

## Backward Compatibility

### Existing Behavior Preserved

**Scenario 1**: User runs MCP CLI (typical case)
- Before: CLI auto-detects maproom-postgres
- After: ✅ Same (DATABASE_URL not set, auto-detects)

**Scenario 2**: Developer in devcontainer with docker-compose.yml DATABASE_URL
- Before: CLI overrides with maproom-postgres (WRONG)
- After: ✅ Respects devcontainer DATABASE_URL (FIXED)

**Scenario 3**: Direct Rust binary usage
- Before: Requires explicit DATABASE_URL
- After: ✅ Auto-detects maproom-postgres (IMPROVED)

**Scenario 4**: Explicit DATABASE_URL override
- Before: CLI ignores it
- After: ✅ Respects it (IMPROVED)

### Migration Path

No migration needed - changes are pure improvements:
- Users with no DATABASE_URL: behavior unchanged
- Users with explicit DATABASE_URL: now works correctly (was broken)
- Direct Rust binary users: now works (was failing)

## Performance Impact

Minimal performance impact:

1. **Hostname resolution check**: 1-second timeout, only when DATABASE_URL not set
2. **Rust**: Happens once at startup during pool creation
3. **Node.js**: Happens once per CLI invocation
4. **Caching**: Not needed - resolution is fast enough

## Security Considerations

No new security concerns:

1. Connection strings still sanitized in logs (existing `sanitize_database_url()`)
2. No plaintext passwords in error messages
3. Hostname resolution doesn't expose credentials
4. Localhost fallback uses standard port (5433) not privileged ports

## Alternative Approaches Considered

### Alternative A: Use Individual Component Env Vars (REJECTED)

Use `CREWCHIEF_DB_HOST`, `CREWCHIEF_DB_PORT`, etc. from docker-compose.yml

**Why Rejected**:
- Industry standard is DATABASE_URL for connection strings
- Individual components add complexity
- Existing docker-compose.yml already sets DATABASE_URL
- 12-factor app pattern uses single URL

### Alternative B: Configuration File (REJECTED)

Read from `~/.maproom-mcp/config.yml` or similar

**Why Rejected**:
- Adds file I/O overhead
- Environment variables are simpler
- Config files complicate Docker/container deployments
- Need to sync config between Node.js and Rust

### Alternative C: No Fallback, Require Explicit Config (REJECTED)

Remove auto-detection entirely, require DATABASE_URL always

**Why Rejected**:
- Poor user experience for typical MCP users
- Forces every user to set environment variables
- Breaks "just works" principle for Docker Compose setups
- User asked specifically for auto-detection to maproom-postgres

## Summary

The architecture uses a simple, consistent fallback hierarchy:

1. Respect explicit `DATABASE_URL` (devcontainer compatibility)
2. Allow `MAPROOM_DB_HOST` override (flexibility)
3. Auto-detect `maproom-postgres` (typical users)
4. Fallback to localhost (development)

Both Node.js and Rust use identical logic for predictable behavior. The solution is backward compatible, performant, and follows industry best practices.
