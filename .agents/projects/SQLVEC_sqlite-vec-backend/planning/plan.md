# Execution Plan: SQLite-Vec Backend

## Phase 1: Abstraction (Tickets 1-3)
1.  **Define Trait**: Create `VectorStore` trait and move existing `tokio-postgres` calls into `PostgresStore`.
2.  **Refactor Consumers**: Update `Indexer`, `Searcher`, `Context` to use `Arc<dyn VectorStore>`.
3.  **Verify Postgres**: Ensure existing tests pass with the refactor.

## Phase 2: SQLite Implementation (Tickets 4-6)
4.  **Build Setup**: Configure `Cargo.toml` and `build.rs` to include `rusqlite` and `sqlite-vec`.
5.  **Schema Migration**: Translate Postgres schema to SQLite (tables + FTS5 + vec0).
6.  **Impl Store**: Implement `SqliteStore` methods (insert, search, etc.).

## Phase 3: Integration & Config (Tickets 7-9)
7.  **Configuration**: Add `provider` switch to config and `main.rs`.
8.  **Unified Tests**: Create the compliance test suite and enable it for SQLite.
9.  **Benchmarks**: Measure performance and tune pragmas (WAL, page size).

## Agent Assignments
- **Rust Engineer**: All implementation tickets.
- **Performance Engineer**: Benchmarking.

## Success Definition
- User can run `crewchief maproom serve` without Docker running.
- Semantic search quality is comparable between backends.

