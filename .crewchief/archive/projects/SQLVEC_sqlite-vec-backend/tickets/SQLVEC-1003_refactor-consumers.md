# Ticket: Refactor Consumers to Use VectorStore

**ID:** SQLVEC-1003
**Phase:** 1
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Update `Indexer`, `Searcher`, and other high-level components to accept `Arc<dyn VectorStore>` instead of `pg::Pool` or `Client`.

## Background
The application logic currently calls specific DB functions. It must now call trait methods.

## Acceptance Criteria
- [ ] `Indexer` struct updated to use `Arc<dyn VectorStore>`.
- [ ] `Searcher` struct updated.
- [ ] `main.rs` instantiates `PostgresStore` and passes it down.
- [ ] All `cargo test` run pass with the refactor.

## Technical Requirements
- Use `Arc<dyn VectorStore>` for shared ownership across async tasks.

## Implementation Notes
- This effectively completes the "Strategy Pattern" refactor for the DB.

## Dependencies
- SQLVEC-1002

## Risks
- Performance regression if `dyn dispatch` is in a hot loop (unlikely for DB IO).

