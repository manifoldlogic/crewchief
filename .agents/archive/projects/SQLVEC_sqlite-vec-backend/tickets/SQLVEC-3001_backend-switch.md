# Ticket: Implement Backend Switching & Config

**ID:** SQLVEC-3001
**Phase:** 3
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Update the application configuration and startup logic to choose between Postgres and SQLite backends.

## Background
The app currently assumes Postgres. We need to inspect `MAPROOM_DB_URL` or config to decide.

## Acceptance Criteria
- [ ] `crates/maproom/src/main.rs` parses config.
- [ ] If URL starts with `postgres://`, instantiate `PostgresStore`.
- [ ] If URL starts with `sqlite://` or `file:`, instantiate `SqliteStore`.
- [ ] Default to `sqlite://maproom.db` if no config provided (Zero-config goal).

## Technical Requirements
- **Factory**: Simple factory pattern in `src/db/mod.rs`.

## Implementation Notes
- Ensure graceful error if the wrong URL type is provided.

## Dependencies
- SQLVEC-1003
- SQLVEC-2001

## Risks
- None.

