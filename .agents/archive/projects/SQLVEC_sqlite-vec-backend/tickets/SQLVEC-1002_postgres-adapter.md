# Ticket: Implement PostgresStore Adapter

**ID:** SQLVEC-1002
**Phase:** 1
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Move the existing `tokio-postgres` logic into a struct `PostgresStore` that implements the `VectorStore` trait.

## Background
We need to preserve the existing functionality while putting it behind the new interface.

## Acceptance Criteria
- [ ] `PostgresStore` struct defined in `src/db/postgres/store.rs`.
- [ ] Implements all methods of `VectorStore`.
- [ ] Uses existing `src/db/queries.rs` logic (which can be moved to `src/db/postgres/queries.rs`).
- [ ] Connects to Postgres using the existing logic.

## Technical Requirements
- The struct will hold the `deadpool_postgres::Pool` or `tokio_postgres::Client` (likely the Pool for the store instance).

## Implementation Notes
- You might need to wrap the existing functions in the trait implementation.

## Dependencies
- SQLVEC-1001

## Risks
- Lifetime issues with async traits and database clients. Use `Arc` where needed.

