# Ticket: Implement SqliteStore Connection & WAL

**ID:** SQLVEC-2001
**Phase:** 2
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Set up the `SqliteStore` struct with `rusqlite`, connection pooling, and Write-Ahead Logging (WAL) enabled.

## Background
SQLite requires specific configuration for concurrency. WAL mode is essential for the indexer (writer) not to block searchers (readers).

## Acceptance Criteria
- [ ] `SqliteStore` struct defined in `crates/maproom/src/db/sqlite/mod.rs`.
- [ ] Uses `r2d2_sqlite` (or `deadpool-sqlite`) for connection pooling.
- [ ] `initialize()` method sets `PRAGMA journal_mode=WAL` and `PRAGMA synchronous=NORMAL`.
- [ ] `sqlite-vec` extension is loaded on every new connection.

## Technical Requirements
- **Pool Size**: Configurable via env/config.
- **Extension Loading**: Must use `sqlite3_auto_extension` or `load_extension` on connection init.

## Implementation Notes
- Use `deadpool-sqlite` if we want async-friendly pooling, but `rusqlite` itself is synchronous. `tokio-rusqlite` is an option, or just run blocking calls in `spawn_blocking`. Given the trait is async, `tokio-rusqlite` or `spawn_blocking` is required.

## Dependencies
- SQLVEC-1003 (Trait ready)
- SQLVEC-1001 (Extension ready)

## Risks
- `SQLITE_BUSY` errors if WAL setup is incorrect.

