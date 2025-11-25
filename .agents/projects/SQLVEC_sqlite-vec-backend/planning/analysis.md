# Analysis: SQLite-Vec Backend

## 1. Problem Definition
The current Maproom daemon requires a running PostgreSQL instance with `pgvector`. This introduces a heavy external dependency (Docker) that complicates local development and "zero-setup" usage.
- **Current**: `crewchief` -> `maproom daemon` -> `postgres:5432` (Docker)
- **Goal**: `crewchief` -> `maproom daemon` (embedded `sqlite` + `sqlite-vec`) -> `maproom.db`

## 2. Technical Context
- **Existing Code**: `crates/maproom/src/db/` contains direct `tokio-postgres` usage. Queries are often raw SQL strings in `queries.rs`.
- **Dependencies**: `tokio-postgres`, `pgvector`.
- **Target**: `rusqlite`, `sqlite-vec` (crate or static link).

## 3. Requirements
1.  **Abstraction**: Introduce a `VectorStore` trait that covers all DB operations (insert chunk, search, upsert file, etc.).
2.  **Postgres Implementation**: Move existing logic into `PostgresStore` struct implementing `VectorStore`.
3.  **SQLite Implementation**: Create `SqliteStore` implementing `VectorStore`.
4.  **Vector Search**: Use `sqlite-vec` for cosine similarity search.
5.  **FTS**: Use SQLite FTS5 for full-text search.
6.  **Config**: Add `db.provider` (postgres|sqlite) to `maproom.config.yaml`.

## 4. Risks
- **SQL Dialect**: Postgres and SQLite SQL syntax differs significantly (e.g., `RETURNING`, `ON CONFLICT`, types). Queries cannot be shared easily.
- **Concurrency**: SQLite writes are serialized. Performance for concurrent indexing might degrade compared to Postgres.
- **Migration Maintenance**: We must maintain schema migrations for *both* DBs.

## 5. Performance Considerations
- `sqlite-vec` is fast but runs in-process.
- Batch inserts in SQLite should use a single transaction.
- WAL mode must be enabled for concurrency.

