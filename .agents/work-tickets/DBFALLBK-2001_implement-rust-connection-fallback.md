# Ticket: DBFALLBK-2001: Implement Rust Database Connection Fallback Logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create a new connection.rs module that implements the database URL fallback hierarchy, and update pool.rs and queries.rs to use it instead of requiring explicit DATABASE_URL.

## Background
Currently, the Rust binary (`crewchief-maproom`) requires DATABASE_URL to be explicitly set or it fails with an error. The Node.js CLI has auto-detection logic that checks for maproom-postgres hostname and falls back appropriately.

This creates inconsistent behavior:
- Node.js CLI: Auto-detects maproom-postgres (but ignores explicit DATABASE_URL - will be fixed in Phase 3)
- Rust binary: Requires explicit DATABASE_URL, no auto-detection

This ticket implements Phase 2 from planning/plan.md: adding the same fallback logic to the Rust binary so it behaves consistently with the Node.js CLI.

## Acceptance Criteria
- [ ] New module `crates/maproom/src/db/connection.rs` created with `get_database_url()` function
- [ ] Fallback hierarchy implemented: DATABASE_URL → MAPROOM_DB_HOST → maproom-postgres hostname → localhost:5433
- [ ] `pool.rs` updated to use `get_database_url()` instead of `std::env::var("DATABASE_URL")`
- [ ] `queries.rs` updated to use `get_database_url()` instead of `std::env::var("DATABASE_URL")`
- [ ] Module exported in `db/mod.rs`
- [ ] Debug logging added to show which connection method was used

## Technical Requirements
- Implement `get_database_url() -> Result<String>` function with this fallback logic:
  1. If DATABASE_URL env var is set → return it (respect explicit config)
  2. If MAPROOM_DB_HOST env var is set → build connection string from MAPROOM_DB_HOST and MAPROOM_DB_PORT
  3. If maproom-postgres hostname resolves → return `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
  4. Otherwise → return `postgresql://maproom:maproom@127.0.0.1:5433/maproom` (localhost fallback)

- Hostname resolution check using `can_resolve_hostname()` helper:
  - Try `getent hosts <hostname>` first (Linux)
  - Fallback to `ping -c 1 -W 1 <hostname>` (cross-platform)
  - 1-second timeout to avoid hanging

- Use `tracing::debug!()` to log which method was used
- Update error messages to mention fallback logic

## Implementation Notes
The key module structure:

```rust
// crates/maproom/src/db/connection.rs
use anyhow::{Context, Result};
use std::env;
use std::process::Command;
use tracing::debug;

pub fn get_database_url() -> Result<String> {
    // 1. Explicit DATABASE_URL
    if let Ok(url) = env::var("DATABASE_URL") {
        debug!("Using explicit DATABASE_URL from environment");
        return Ok(url);
    }

    // 2. MAPROOM_DB_HOST components
    if let Ok(host) = env::var("MAPROOM_DB_HOST") {
        let port = env::var("MAPROOM_DB_PORT").unwrap_or_else(|_| "5432".to_string());
        let url = format!("postgresql://maproom:maproom@{}:{}/maproom", host, port);
        debug!("Using MAPROOM_DB_HOST: {}", host);
        return Ok(url);
    }

    // 3. Auto-detect maproom-postgres
    if can_resolve_hostname("maproom-postgres") {
        debug!("Auto-detected maproom-postgres hostname");
        return Ok("postgresql://maproom:maproom@maproom-postgres:5432/maproom".to_string());
    }

    // 4. Localhost fallback
    debug!("Falling back to localhost:5433");
    Ok("postgresql://maproom:maproom@127.0.0.1:5433/maproom".to_string())
}

fn can_resolve_hostname(hostname: &str) -> bool {
    // Try getent hosts first (Linux)
    if let Ok(status) = Command::new("getent")
        .args(&["hosts", hostname])
        .status()
    {
        if status.success() {
            return true;
        }
    }

    // Fallback to ping (cross-platform)
    if let Ok(status) = Command::new("ping")
        .args(&["-c", "1", "-W", "1", hostname])
        .status()
    {
        return status.success();
    }

    false
}
```

### Changes Required

Update `pool.rs` (around line 119):
```rust
use crate::db::connection::get_database_url;

pub async fn create_pool() -> anyhow::Result<PgPool> {
    let database_url = get_database_url()
        .context("Failed to determine database connection URL")?;

    // ... rest of function remains the same
}
```

Update `queries.rs` (around line 6):
```rust
use crate::db::connection::get_database_url;

// Replace env::var("DATABASE_URL") calls with get_database_url()
```

Add to `db/mod.rs`:
```rust
pub mod connection;
```

### Testing Strategy
- Test with explicit DATABASE_URL set
- Test with MAPROOM_DB_HOST set
- Test with maproom-postgres hostname available (Docker)
- Test with localhost fallback (no hostname resolution)
- Verify debug logging appears in each scenario

## Dependencies
- DBFALLBK-1001 (devcontainer postgres removal) should be complete first, but not strictly required

## Risk Assessment
- **Risk**: Hostname resolution might hang on some systems
  - **Mitigation**: 1-second timeout on all resolution attempts using `-W 1` flag

- **Risk**: Different behavior on Windows vs Linux
  - **Mitigation**: Fallback from getent to ping covers both platforms; getent is Linux-specific but ping is cross-platform

- **Risk**: Breaking existing code that expects DATABASE_URL requirement
  - **Mitigation**: This is an improvement - code that sets DATABASE_URL still works (it's checked first in the fallback hierarchy)

- **Risk**: getent or ping commands might not be available on all systems
  - **Mitigation**: If both fail, we fall back to localhost:5433 which is a safe default

## Files/Packages Affected
- `/workspace/crates/maproom/src/db/connection.rs` - New module (create)
- `/workspace/crates/maproom/src/db/pool.rs` - Update to use get_database_url() (around line 119)
- `/workspace/crates/maproom/src/db/queries.rs` - Update to use get_database_url() (around line 6)
- `/workspace/crates/maproom/src/db/mod.rs` - Export connection module
