# Ticket: Define VectorStore Trait and Refactor DB Module

**ID:** SQLVEC-1001
**Phase:** 1
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Create the `VectorStore` trait in `crates/maproom/src/db/store.rs` and restructure the `db` module to support multiple backends.

## Background
Currently, database functions are top-level async functions in `src/db/mod.rs` or `queries.rs` that take a `tokio_postgres::Client`. We need to move these into a trait to allow swapping implementations.

## Acceptance Criteria
- [ ] `VectorStore` trait defined with async methods for all DB operations.
- [ ] `db` module structure refactored to `db/mod.rs`, `db/store.rs`, `db/postgres/`.
- [ ] Existing public functions in `db` are marked deprecated or moved to `PostgresStore`.

## Technical Requirements
- **Trait Definition**:
  ```rust
  #[async_trait]
  pub trait VectorStore: Send + Sync {
      // Connection/Lifecycle
      async fn health_check(&self) -> Result<()>;
      
      // Files
      async fn upsert_file(&self, file: FileData) -> Result<i64>;
      async fn get_file_id(&self, repo_id: i64, path: &str) -> Result<Option<i64>>;
      
      // Chunks
      async fn insert_chunk(&self, chunk: ChunkData) -> Result<i64>;
      async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>;
      
      // ... complete list of ops
  }
  ```

## Implementation Notes
- This is a pure refactor. No logic changes, just moving signatures.

## Dependencies
- None

## Risks
- Missing a method that is used in a weird place. Grep usage of `db::*`.

