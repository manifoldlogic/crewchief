# Ticket: MAPCLI-1000: Add BackendType Enum and Trait Method

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `BackendType` enum to the VectorStore trait for runtime backend detection. This is a prerequisite for all other MAPCLI tickets as it enables runtime decisions based on which database backend is active.

## Background
The MAPCLI project needs to enable SQLite backend support for the CLI and daemon. To handle backend-specific behavior (e.g., skipping migrations for SQLite, disabling parallel scan), we need a way to detect which backend is active at runtime. The `BackendType` enum and `backend_type()` trait method provide this capability.

This ticket implements **Architecture Decision 0** from the architecture.md - the prerequisite that must be completed before any other work.

**Plan Reference**: Phase 1: Prerequisite (MAPCLI-1000) in plan.md

## Acceptance Criteria
- [ ] `BackendType` enum exists in `crates/maproom/src/db/mod.rs` with variants `PostgreSQL` and `SQLite`
- [ ] `backend_type(&self) -> BackendType` method added to `VectorStore` trait
- [ ] `PostgresStore` implements `backend_type()` returning `BackendType::PostgreSQL`
- [ ] `SqliteStore` implements `backend_type()` returning `BackendType::SQLite`
- [ ] Compilation succeeds without `--features sqlite`: `cargo build --bin crewchief-maproom`
- [ ] Compilation succeeds with `--features sqlite`: `cargo build --features sqlite --bin crewchief-maproom`
- [ ] All existing tests pass: `cargo test`

## Technical Requirements
- `BackendType` enum must derive `Debug, Clone, Copy, PartialEq, Eq`
- The enum should be public and exported from `db` module
- Method signature: `fn backend_type(&self) -> BackendType;`
- SQLite implementation must be behind `#[cfg(feature = "sqlite")]` feature gate
- No breaking changes to existing VectorStore trait methods

## Implementation Notes

### Step 1: Add BackendType enum to db/mod.rs
```rust
/// Backend type for runtime detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    PostgreSQL,
    SQLite,
}
```

### Step 2: Add trait method to VectorStore
```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Returns the backend type for runtime feature detection
    fn backend_type(&self) -> BackendType;

    // ... existing methods unchanged
}
```

### Step 3: Implement in PostgresStore (db/postgres/mod.rs)
```rust
impl VectorStore for PostgresStore {
    fn backend_type(&self) -> BackendType {
        BackendType::PostgreSQL
    }
    // ... existing implementations
}
```

### Step 4: Implement in SqliteStore (db/sqlite/mod.rs)
```rust
#[cfg(feature = "sqlite")]
impl VectorStore for SqliteStore {
    fn backend_type(&self) -> BackendType {
        BackendType::SQLite
    }
    // ... existing implementations
}
```

### Key Considerations
- This is a non-async method (no `async` keyword) since it returns a compile-time constant
- Adding a new method to the trait is a breaking change if there are external implementors, but in this codebase only PostgresStore and SqliteStore implement it
- The enum lives in `db/mod.rs` rather than `db/factory.rs` because it's part of the trait definition

## Dependencies
- None - this is the first ticket and has no dependencies
- VECSTORE project completed (provides the VectorStore trait we're extending)

## Risk Assessment
- **Risk**: Adding a method to an existing trait is a breaking change
  - **Mitigation**: Only internal types implement VectorStore; no external consumers
- **Risk**: Feature flag complexity with SQLite
  - **Mitigation**: Follow existing patterns in db/mod.rs for feature-gated code

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` - Add BackendType enum and trait method
- `crates/maproom/src/db/postgres/mod.rs` - Implement backend_type() for PostgresStore
- `crates/maproom/src/db/sqlite/mod.rs` - Implement backend_type() for SqliteStore (feature-gated)

## Testing
```bash
# Verify compilation without sqlite feature
cargo build --bin crewchief-maproom

# Verify compilation with sqlite feature
cargo build --features sqlite --bin crewchief-maproom

# Run all tests
cargo test

# Run tests with sqlite feature
cargo test --features sqlite
```
