# Ticket: Add Configuration for Database Provider

**ID:** SQLVEC-3001
**Phase:** 3
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Update the configuration system to allow choosing between `postgres` and `sqlite`.

## Background
Users need to control which backend is used.

## Acceptance Criteria
- [ ] `maproom.config.yaml` supports `database.provider: "sqlite" | "postgres"`.
- [ ] `maproom.config.yaml` supports `database.sqlite_path` (default `~/.config/crewchief/maproom.db`).
- [ ] `main.rs` reads this config and initializes the correct store.

## Technical Requirements
- Update config structs.

## Implementation Notes
- Default to `postgres` for now to avoid breaking changes, or switch to `sqlite` if we are ready for the "zero-dependency" release.

## Dependencies
- SQLVEC-2003

## Risks
- None.

