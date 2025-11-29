# Ticket: SQLFIX-1001: Fix SQLite Compilation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Verification is `cargo check --features sqlite` passing
- No unit tests yet (those come in SQLFIX-1004)

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix all compile-time errors in the SQLite backend, including Cargo.toml dependencies, module exports, and move semantics errors. Also add security hardening (busy_timeout, file permissions).

## Background
The SQLite backend was left in a non-compiling state by the SQLVEC project. Running `cargo check --features sqlite` produces exactly 4 errors (verified 2025-11-25):

```
error[E0432]: unresolved import `crate::db::sqlite::schema`
  --> crates/maproom/src/db/sqlite/mod.rs:10:24

error[E0277]: the trait bound `DateTime<Utc>: rusqlite::ToSql` is not satisfied
   --> crates/maproom/src/db/sqlite/mod.rs:149:21

error[E0277]: the trait bound `DateTime<Utc>: rusqlite::ToSql` is not satisfied
   --> crates/maproom/src/db/sqlite/mod.rs:172:17

error[E0382]: use of moved value
   --> crates/maproom/src/db/sqlite/mod.rs:562:47
```

This ticket also incorporates security recommendations from the security review.

**Plan Reference**: Phase 1 - Compile Fixes + CI (Ticket 1001)

## Acceptance Criteria
- [ ] `cargo check --features sqlite` passes with zero errors
- [ ] `cargo check --features postgres` still passes (no regression)
- [ ] `cargo check` (default features) still passes
- [ ] `pub mod schema;` declaration added to `sqlite/mod.rs`
- [ ] `rusqlite` has `chrono` feature enabled in Cargo.toml
- [ ] `find_chunk_by_symbol` compiles without move errors
- [ ] `busy_timeout` PRAGMA is set in connection initialization
- [ ] Database file permissions set to 0600 on Unix systems

## Technical Requirements

### 1. Cargo.toml Dependencies (Line 89)
Update `crates/maproom/Cargo.toml`:
```toml
# Change from:
rusqlite = { version = "0.29.0", features = ["bundled"], optional = true }

# To:
rusqlite = { version = "0.29.0", features = ["bundled", "chrono"], optional = true }
```

### 2. Module Export (Line 10)
The error occurs because `schema.rs` exists but there's no module declaration.

Add at the **top** of `crates/maproom/src/db/sqlite/mod.rs` (before line 1):
```rust
pub mod schema;
```

Then the import on line 10 will work:
```rust
use crate::db::sqlite::schema::init_schema;
```

### 3. Move Semantics Fix (Lines 534-574)
The issue is in `find_chunk_by_symbol`. The `relpath` variable is consumed in a pattern match, then reused.

**Current code (line 534):**
```rust
let sql = if let Some(path) = relpath {  // relpath moved here
```

**Then at line 562:**
```rust
let id: Option<i64> = if let Some(path) = relpath {  // ERROR: already moved
```

**Fix**: Clone relpath at the start of the closure (after line 531):
```rust
let symbol_name = symbol_name.to_string();
let relpath = relpath.map(|s| s.to_string());  // This line already exists!
self.run(move |conn| {
    let relpath_ref = relpath.as_deref();  // ADD: Create reference for reuse
    // Similar to Postgres logic
    let sql = if let Some(path) = relpath_ref {  // CHANGE: use relpath_ref
        // ... rest of code uses relpath_ref instead of relpath
```

**Alternative simpler fix** - use `relpath.as_ref()` in pattern matching:
```rust
let sql = if relpath.is_some() {
    if worktree_id.is_some() {
        // query with path
    } else {
        // query with path, no worktree
    }
} else {
    if worktree_id.is_some() {
        // query without path
    } else {
        // query without path or worktree
    }
};
// Then use relpath.as_deref() in the params
```

### 4. Connection Initialization (Lines 43-51)
Add `busy_timeout` to existing PRAGMA block in `SqliteStore::connect()`:
```rust
.with_init(|conn| {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        "#,
    )?;
    Ok(())
});
```

### 5. File Permissions Security (After line 60)
Add after pool creation in `connect()`:
```rust
let pool = r2d2::Pool::builder()
    .max_size(10)
    .build(manager)
    .context("Failed to create SQLite connection pool")?;

// ADD: Set secure file permissions on database file
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let db_path = std::path::Path::new(path);
    if db_path.exists() && !path.contains(":memory:") {
        std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600))
            .context("Failed to set database file permissions")?;
    }
}

Ok(Self { pool })
```

## Implementation Notes

### Pre-flight Verification
Before making changes, run:
```bash
cargo check --features sqlite 2>&1 | grep "^error"
```
This should show exactly 4 errors as documented above.

### Verification Steps
```bash
# Primary verification - must pass
cargo check --features sqlite

# Regression checks - must pass
cargo check --features postgres
cargo check  # default features

# Full verification
cargo check --features sqlite && cargo check --features postgres && cargo check
```

### Code Locations Summary
| Fix | File | Line(s) |
|-----|------|---------|
| Module export | `sqlite/mod.rs` | Add before line 1 |
| Chrono feature | `Cargo.toml` | Line 89 |
| Move semantics | `sqlite/mod.rs` | Lines 534-574 |
| busy_timeout | `sqlite/mod.rs` | Lines 43-51 |
| File permissions | `sqlite/mod.rs` | After line 60 |

## Dependencies
- **SQLFIX-1000**: Baseline fixes must be committed first

## Risk Assessment
- **Risk**: chrono feature compatibility
  - **Mitigation**: Verified rusqlite 0.29 supports chrono 0.4.x via the chrono feature flag
- **Risk**: find_chunk_by_symbol logic change
  - **Mitigation**: Logic preserved, only ownership fixed; runtime testing in SQLFIX-1003
- **Risk**: File permissions on non-Unix systems
  - **Mitigation**: `#[cfg(unix)]` guard ensures code only runs on Unix; `:memory:` check prevents errors for in-memory databases

## Files/Packages Affected
- `crates/maproom/Cargo.toml`
- `crates/maproom/src/db/sqlite/mod.rs`
