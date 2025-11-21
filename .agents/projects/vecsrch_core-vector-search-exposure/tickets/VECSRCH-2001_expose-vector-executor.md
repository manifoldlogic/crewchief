# Ticket: Expose VectorExecutor Types

**ID:** VECSRCH-2001
**Phase:** Implementation
**Status:** Open

## Title & Summary
Expose `VectorExecutor` types to the CLI module.

## Background
The `VectorExecutor` struct is currently defined in `crates/maproom/src/search/vector.rs`. To use it from the CLI (which is likely in `src/main.rs` or a `cli` module), we need to ensure it and its necessary public methods/types are accessible.

## Acceptance Criteria
1.  `VectorExecutor` is accessible from `src/main.rs`.
2.  Any helper types returned by `VectorExecutor` (e.g., search results) are also public/accessible.
3.  The project compiles without visibility errors.

## Technical Requirements
- Modify `crates/maproom/src/lib.rs` to re-export `search` module or `VectorExecutor` specifically.
- Ensure `pub` visibility is set correctly on the struct and its methods.

## Implementation Notes
- Check `crates/maproom/src/lib.rs`.
- Ensure `mod search;` is present and public if needed.

## Dependencies
None.

## Risks
- Exposing internal types might leak implementation details, but `VectorExecutor` seems designed for this purpose.

## Files/Packages
- `crates/maproom/src/lib.rs`
- `crates/maproom/src/search/mod.rs` (if exists)
- `crates/maproom/src/search/vector.rs`

## Agent Assignments
- **Primary:** Rust Developer
