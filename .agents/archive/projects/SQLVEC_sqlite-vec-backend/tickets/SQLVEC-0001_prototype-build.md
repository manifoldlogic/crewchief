# Ticket: Prototype Build: Static Linking & 1536-Dim Verification

**ID:** SQLVEC-0001
**Phase:** 0
**Status:** Pending
**Assigned To:** Build Engineer

## Summary
Create a standalone proof-of-concept Rust binary that statically links `sqlite-vec` using the `cc` crate and verifies support for 1536-dimensional vectors (OpenAI compatible). This is a critical go/no-go gate.

## Background
Before refactoring the entire codebase, we must confirm that we can successfully compile `sqlite-vec` into a Rust binary across our target platforms and that it supports the required vector dimensions without runtime errors.

## Acceptance Criteria
- [ ] Standalone Rust project (in `crates/maproom/examples/prototype-sqlite-vec` or temporary location) compiles successfully.
- [ ] `sqlite-vec.c` is compiled via `build.rs` and linked.
- [ ] Binary successfully creates a `vec0` table with 1536 dimensions.
- [ ] Binary successfully inserts a 1536-float array and queries it.
- [ ] No shared library dependency on `libsqlite3` (verify with `ldd` or `otool`).

## Technical Requirements
- **Dependencies**: `rusqlite`, `cc`.
- **Source**: Download latest release of `sqlite-vec.c` / `.h`.
- **Verification**:
  ```rust
  // Pseudocode
  conn.execute("CREATE VIRTUAL TABLE vec_test USING vec0(e float[1536])", [])?;
  let vector = vec![0.1; 1536];
  // Insert and query...
  ```

## Implementation Notes
- This ticket mitigates the highest technical risks identified in the review.
- If this fails, the project must be paused or rescoped.

## Dependencies
- None

## Risks
- Cross-compilation complexity.
- `sqlite-vec` might default to lower dimensions (e.g., 1024) and require compile-time flags.

