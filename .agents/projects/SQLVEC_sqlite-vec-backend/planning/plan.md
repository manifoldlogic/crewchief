# Execution Plan: SQLite-Vec Backend

## Phase 0: De-risking (Ticket 0)
0.  **Prototype Build**: Create a standalone "hello world" Rust binary that statically links `sqlite-vec` and inserts/queries a 1536-dim vector. Verify build works on Linux/macOS.

## Phase 1: Abstraction & Vendoring (Tickets 1-3)
1.  **Vendor Extension**: Download and commit `sqlite-vec` source code. Update `build.rs` (apply learnings from Ticket 0).
2.  **Define Trait**: Extract `VectorStore` trait from current db logic.
3.  **Refactor Postgres**: Move existing logic into `PostgresStore`.

## Phase 2: SQLite Implementation (Tickets 4-6)
4.  **Connection & Schema**: Implement `SqliteStore` with `rusqlite`, `r2d2` pooling, and **WAL mode**. Create tables.
5.  **Vector Integration**: Implement vector ops. Verify 1536-dim support.
6.  **FTS Integration**: Implement FTS5 logic, translating search queries to SQLite syntax.

## Phase 3: Integration & Switching (Tickets 7-9)
7.  **Config Switch**: Update `main.rs` / `lib.rs` to choose backend.
8.  **Integration Tests**: Create shared test suite.
9.  **Benchmarks**: Measure indexing/search performance.

## Agent Assignments
- **Rust Engineer**: All implementation tickets.
- **Build Engineer**: Ticket 0 & 1.

## Success Definition
- `crewchief-maproom serve` runs locally with a single `.db` file.
- Vector search returns relevant results (parity check).
- No `SQLITE_BUSY` errors during indexing.
