# Ticket: Vendor sqlite-vec Source Code

**ID:** SQLVEC-1001
**Phase:** 1
**Status:** Pending
**Assigned To:** Build Engineer

## Summary
Vendor the `sqlite-vec` C source code into the repository and configure the build system to compile it.

## Background
To ensure reproducible, zero-dependency builds, we will include the C extension source directly in our tree rather than relying on system libraries or external package managers.

## Acceptance Criteria
- [ ] `sqlite-vec` source files (.c, .h) added to `crates/maproom/vendor/sqlite-vec/`.
- [ ] `crates/maproom/build.rs` updated to compile the vendored source using `cc`.
- [ ] `rusqlite` configured to bundle sqlite (feature `bundled`).
- [ ] Build succeeds with `cargo build -p crewchief-maproom`.

## Technical Requirements
- **Path**: `crates/maproom/vendor/sqlite-vec/`
- **Build Script**: Must handle platform-specific flags if necessary (discovered in Ticket 0).
- **License**: Ensure license compatibility (MIT/Apache) and include license file.

## Implementation Notes
- Apply learnings from SQLVEC-0001.

## Dependencies
- SQLVEC-0001 (Prototype success)

## Risks
- Bloating the repo size (unlikely for C source).

