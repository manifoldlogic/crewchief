# Ticket: Refactor Postgres Implementation to VectorStore

**ID:** SQLVEC-1003
**Phase:** 1
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Move the existing `tokio-postgres` logic into a `PostgresStore` struct that implements the `VectorStore` trait.

## Background
We need to preserve the existing functionality while putting it behind the new abstraction.

## Acceptance Criteria
- [ ] `PostgresStore` struct created in `crates/maproom/src/db/postgres/mod.rs`.
- [ ] Implements `VectorStore`.
- [ ] Existing `src/db/queries.rs` logic moved/adapted to the struct methods.
- [ ] Application code (indexer/searcher) updated to use `Arc<dyn VectorStore>` instead of `Client`.
- [ ] All existing tests pass.

## Technical Requirements
- **Refactor**: This is a large refactor. Use the compiler to guide you.
- **Performance**: Ensure no regressions in connection pooling (use `deadpool-postgres` as is).

## Implementation Notes
- This is the biggest ticket in Phase 1. Take care with the `Arc<dyn VectorStore>` passing.

## Dependencies
- SQLVEC-1002

## Risks
- Breaking existing functionality. Rely on existing tests.

