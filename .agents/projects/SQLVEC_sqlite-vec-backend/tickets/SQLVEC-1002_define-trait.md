# Ticket: Define VectorStore Trait Abstraction

**ID:** SQLVEC-1002
**Phase:** 1
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Refactor the database layer to introduce a `VectorStore` trait, decoupling business logic from `tokio-postgres`.

## Background
Currently, database logic is direct `client.query()` calls. We need an interface that both Postgres and SQLite can implement.

## Acceptance Criteria
- [ ] `VectorStore` trait defined in `crates/maproom/src/db/mod.rs`.
- [ ] Trait covers all necessary operations: `upsert_file`, `upsert_chunks`, `search`, `delete_repo`, `get_stats`.
- [ ] Data structures (`FileRecord`, `ChunkRecord`, `SearchResult`) decoupled from postgres-specific types (e.g., `tokio_postgres::Row`).
- [ ] No implementation changes yet, just the interface definition.

## Technical Requirements
- **Async**: Must be `#[async_trait]`.
- **Send + Sync**: Must be thread-safe.
- **Error Handling**: Use a generic `anyhow::Result` or custom `DbError`.

## Implementation Notes
- Look at `src/db/queries.rs` to inventory all required methods.
- Don't forget transaction management (`begin()`).

## Dependencies
- None

## Risks
- Trait might become "least common denominator", limiting Postgres features. Keep it high-level.

