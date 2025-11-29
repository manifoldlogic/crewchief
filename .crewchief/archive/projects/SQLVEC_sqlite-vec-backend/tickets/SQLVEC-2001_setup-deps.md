# Ticket: Setup SQLite and sqlite-vec Dependencies

**ID:** SQLVEC-2001
**Phase:** 2
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Add `rusqlite` and `sqlite-vec` dependencies to `Cargo.toml` and configure build.

## Background
We need the SQLite driver and the vector search extension available in the project.

## Acceptance Criteria
- [ ] `rusqlite` added with `bundled` feature (to ensure we control the SQLite version).
- [ ] `sqlite-vec` support added (either via crate or `cc` build in `build.rs`).
- [ ] Simple test confirms `SELECT vec_version()` works.

## Technical Requirements
- **Dependencies**:
  ```toml
  rusqlite = { version = "0.31", features = ["bundled"] }
  ```
- **Vector Extension**: If `sqlite-vec` isn't on crates.io easily, use `zerocopy` or similar if needed, or build the C extension. (Check `sqlite-vec` Rust bindings availability).

## Implementation Notes
- `sqlite-vec` is often distributed as a loadable extension. Static linking requires some `build.rs` magic.

## Dependencies
- None (can start in parallel)

## Risks
- Cross-compilation issues with C extensions.

