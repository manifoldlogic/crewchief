# Ticket: MAPCLI-1003: Add SQLite Backend Detection and Configuration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement auto-detection of SQLite database and configuration options, enabling zero-configuration operation for new users while preserving PostgreSQL behavior for existing users.

## Background
Currently, the CLI requires `MAPROOM_DATABASE_URL` to be set to a PostgreSQL connection string. For zero-configuration operation, we want new users to be able to run maproom commands without any database setup - defaulting to a local SQLite database at `~/.maproom/maproom.db`.

This ticket updates the database URL detection logic in the factory to support SQLite-first defaults while maintaining backward compatibility with PostgreSQL.

**Plan Reference**: Phase 1: Backend Detection & Configuration (MAPCLI-1003) in plan.md

## Acceptance Criteria
- [ ] `MAPROOM_DATABASE_URL=sqlite://...` correctly selects SQLite backend
- [ ] `MAPROOM_DATABASE_URL=postgresql://...` correctly selects PostgreSQL backend
- [ ] Auto-detection finds existing SQLite database at `~/.maproom/maproom.db`
- [ ] SQLite database is created automatically if no configuration exists and no database found
- [ ] Parent directory `~/.maproom/` is created if needed when creating SQLite database
- [ ] Helpful error message when configuration is invalid
- [ ] PostgreSQL behavior unchanged for existing users with MAPROOM_DATABASE_URL set

## Technical Requirements
- Update `get_database_url()` in `db/connection.rs` or `db/factory.rs`
- Implement detection order: env var → existing SQLite file → default SQLite
- Use `dirs` or `home` crate for cross-platform home directory resolution
- Create parent directories with `std::fs::create_dir_all`
- SQLite URL format: `sqlite:///absolute/path/to/file.db`

## Implementation Notes

### Step 1: Define detection order
```rust
/// Detection order for database URL:
/// 1. MAPROOM_DATABASE_URL environment variable (explicit config)
/// 2. ~/.maproom/maproom.db if exists (existing SQLite database)
/// 3. Default to sqlite://~/.maproom/maproom.db (auto-create)
pub fn get_database_url() -> anyhow::Result<String> {
    // Check environment variable first (preserves existing behavior)
    if let Ok(url) = std::env::var("MAPROOM_DATABASE_URL") {
        return Ok(url);
    }

    // Check for existing SQLite database
    let default_sqlite_path = get_default_sqlite_path()?;
    if default_sqlite_path.exists() {
        return Ok(format!("sqlite://{}", default_sqlite_path.display()));
    }

    // Default to SQLite (will be created on first connection)
    Ok(format!("sqlite://{}", default_sqlite_path.display()))
}
```

### Step 2: Helper for default SQLite path
```rust
fn get_default_sqlite_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".maproom").join("maproom.db"))
}
```

### Step 3: Create parent directory on SQLite connection
In `SqliteStore::connect()` or the factory:
```rust
pub async fn connect(url: &str) -> anyhow::Result<Self> {
    let path = url.strip_prefix("sqlite://")
        .ok_or_else(|| anyhow::anyhow!("Invalid SQLite URL"))?;

    // Create parent directory if needed
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Continue with connection...
}
```

### Step 4: Validate URL format
```rust
fn validate_database_url(url: &str) -> anyhow::Result<()> {
    if url.starts_with("postgresql://") || url.starts_with("postgres://") {
        return Ok(());
    }
    if url.starts_with("sqlite://") {
        return Ok(());
    }
    anyhow::bail!(
        "Invalid database URL format. Expected:\n\
         - PostgreSQL: postgresql://user:pass@host/db\n\
         - SQLite: sqlite:///path/to/database.db\n\
         Got: {}", url
    );
}
```

### Step 5: Helpful error messages
```rust
// When no database configured and SQLite feature not enabled
#[cfg(not(feature = "sqlite"))]
fn get_store_without_sqlite() -> anyhow::Result<Arc<dyn VectorStore>> {
    anyhow::bail!(
        "No database configured.\n\
         Set MAPROOM_DATABASE_URL environment variable to a PostgreSQL connection string.\n\
         Example: MAPROOM_DATABASE_URL=postgresql://user:pass@localhost/maproom\n\
         \n\
         For SQLite support, rebuild with: cargo build --features sqlite"
    );
}
```

### Detection Scenarios

| Scenario | MAPROOM_DATABASE_URL | ~/.maproom/maproom.db | Result |
|----------|---------------------|----------------------|--------|
| Existing PostgreSQL user | `postgresql://...` | - | PostgreSQL |
| Existing SQLite user | `sqlite://...` | - | SQLite |
| New user, existing DB | Not set | Exists | SQLite (auto-detect) |
| New user, no DB | Not set | Does not exist | SQLite (auto-create) |
| SQLite feature disabled | Not set | - | Error with instructions |

## Dependencies
- **MAPCLI-1000**: BackendType enum for validation
- Can be developed in parallel with MAPCLI-1002 (Daemon)
- `dirs` crate for home directory (or `home` crate)

## Risk Assessment
- **Risk**: Breaking existing PostgreSQL users
  - **Mitigation**: Environment variable takes precedence; no change if MAPROOM_DATABASE_URL is set
- **Risk**: Permission issues creating ~/.maproom directory
  - **Mitigation**: Clear error message explaining the issue and workaround
- **Risk**: Cross-platform path handling issues
  - **Mitigation**: Use `dirs` crate for portable home directory resolution

## Files/Packages Affected
- `crates/maproom/src/db/connection.rs` - Update `get_database_url()` function
- `crates/maproom/src/db/factory.rs` - Possibly add URL validation
- `crates/maproom/src/db/sqlite/mod.rs` - Add parent directory creation to connect()
- `crates/maproom/Cargo.toml` - Add `dirs` dependency if not present

## Testing
```bash
# Test with explicit PostgreSQL URL
MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost/maproom" cargo run --bin crewchief-maproom -- status

# Test with explicit SQLite URL
MAPROOM_DATABASE_URL="sqlite:///tmp/test-maproom.db" cargo run --features sqlite --bin crewchief-maproom -- status

# Test auto-detection with existing database
rm -rf ~/.maproom  # Clean slate
mkdir -p ~/.maproom
cp test-fixtures/sample.db ~/.maproom/maproom.db
unset MAPROOM_DATABASE_URL
cargo run --features sqlite --bin crewchief-maproom -- status  # Should find ~/.maproom/maproom.db

# Test auto-create
rm -rf ~/.maproom
unset MAPROOM_DATABASE_URL
cargo run --features sqlite --bin crewchief-maproom -- status  # Should create ~/.maproom/maproom.db

# Test error without sqlite feature
rm -rf ~/.maproom
unset MAPROOM_DATABASE_URL
cargo run --bin crewchief-maproom -- status  # Should show error with instructions

# Run all tests
cargo test
cargo test --features sqlite
```
