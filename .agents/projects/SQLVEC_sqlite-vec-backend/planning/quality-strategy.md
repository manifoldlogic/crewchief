# Quality Strategy: SQLite-Vec Backend

## 1. Test Strategy
We need to ensure **Functional Parity** between Postgres and SQLite backends.

### Levels of Testing
1.  **Unit Tests (`src/db/sqlite/*.rs`)**:
    - Verify schema creation.
    - Verify basic CRUD operations.
2.  **Integration Tests (`tests/store_compat.rs`)**:
    - Run the *same* test suite against both backends.
    - `test_search_quality`: Index a known codebase, search for a term, assert results overlap.
3.  **Performance Benchmarks (`benches/store_bench.rs`)**:
    - Compare indexing speed (100 files).
    - Compare search latency (vector vs hybrid).

## 2. Critical Paths
- **Vector Search Accuracy**: Does `sqlite-vec` return the same top-K results as `pgvector`?
- **Concurrency**: Does SQLite panic or lock under load from the indexer (rayon threads)? -> *Mitigation: Use WAL mode and a connection pool (r2d2_sqlite).*

## 3. Acceptance Criteria
- [ ] `cargo test` passes for both backends.
- [ ] `maproom search` returns valid results with `MAPROOM_DB_URL=sqlite://test.db`.
- [ ] Binary size increase is < 2MB (sqlite-vec is tiny).
- [ ] No runtime requirement for `libsqlite3` (static linking preferred).

## 4. Risk Mitigation
- **Dual-Run Mode**: For testing, we could implement a mode that writes to both and logs discrepancies, but that might be overkill for MVP. Using shared integration tests is better.
