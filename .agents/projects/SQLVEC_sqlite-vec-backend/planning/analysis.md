# Analysis: SQLite-Vec Backend

## 1. Problem Definition
The current `crewchief-maproom` daemon requires a running PostgreSQL instance with `pgvector`.
- **Dependency Heavy**: Users must install Docker and run `docker-compose up`.
- **Barrier to Entry**: High friction for "quick start" usage.
- **Resource Usage**: Docker + Postgres consumes significant RAM (~500MB+ idle).
- **Portability**: Hard to run in constrained environments (CI, weak laptops).

## 2. Existing Solutions & Context
- **Current State**: `crates/maproom` is tightly coupled to `tokio-postgres`. All DB logic is in `src/db/`.
- **Proposed Solution**: Use `sqlite-vec` (an SQLite extension for vector search) embedded directly in the binary.
- **Industry Trend**: Local-first AI tools (like Ollama) use embedded databases or file-based storage.

## 3. Requirements
1.  **Storage Abstraction**: Introduce a `VectorStore` trait in Rust to decouple the business logic from the DB implementation.
2.  **SQLite Implementation**: Implement `VectorStore` using `rusqlite`.
3.  **Vector Extension**: Statically link `sqlite-vec` into the binary so users don't need to install it separately.
4.  **Feature Parity**:
    - Full Text Search (FTS5 in SQLite vs tsvector in Postgres).
    - Vector Similarity (Cosine distance via sqlite-vec).
    - Metadata storage (Files, Chunks, Worktrees).
5.  **Configuration**: Allow runtime switching via `MAPROOM_DB_URL` (e.g., `sqlite://maproom.db` vs `postgres://...`).

## 4. Risks
- **Performance**: SQLite single-writer lock might be a bottleneck for concurrent indexing.
- **Vector Limit**: `sqlite-vec` is newer and less mature than `pgvector`.
- **Migration**: Porting complex SQL queries (recursive CTEs, window functions) from Postgres to SQLite might be non-trivial.
