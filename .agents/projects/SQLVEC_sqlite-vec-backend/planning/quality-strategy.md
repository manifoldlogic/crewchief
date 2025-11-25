# Quality Strategy: SQLite-Vec Backend

## 1. Test Strategy
Since we are abstracting the DB, we can run the *same* integration tests against both backends.

### Testing Layers
1.  **Unit Tests**: verify SQL generation (if dynamic).
2.  **Integration Tests (`tests/store_compliance.rs`)**:
    - Define a standard test suite: `test_insert_and_retrieve`, `test_vector_search`, `test_fts`.
    - Run suite twice: once with `PostgresStore` (if Docker available), once with `SqliteStore` (in-memory or temp file).

## 2. Performance Benchmarks
- Compare `sqlite-vec` search latency vs `pgvector`.
- Compare indexing throughput (chunks/sec).

## 3. Acceptance Criteria
- [ ] `cargo test --features sqlite` passes.
- [ ] `maproom search --provider sqlite` returns relevant results.
- [ ] `maproom scan` works with SQLite backend without crashing.
- [ ] No regression in Postgres backend.

## 4. CI/CD
- GitHub Actions can easily run SQLite tests (no service container needed).
- Ensure `sqlite-vec` compiles on all target platforms (Linux, macOS, Windows).

