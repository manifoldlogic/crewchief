# Project: SQLVEC_sqlite-vec-backend

## Project Summary
Implement a zero-dependency storage backend for the Maproom Rust daemon using `sqlite-vec`. This project involves refactoring `crates/maproom` to introduce a `VectorStore` trait, abstracting away the direct dependency on `tokio-postgres`. A new `SqliteStore` implementation will be created using `rusqlite` and statically linking the `sqlite-vec` C extension. This enables a "single binary" distribution model where the database is a local file (`maproom.db`) rather than a Docker container. The project includes build system updates (`build.rs`), schema migration (SQL -> SQLite), and a configuration switch to toggle between Postgres (server) and SQLite (local) modes.

## Relevant Agents
- **Rust Engineer**: For trait abstraction and `rusqlite` integration.
- **Database Specialist**: For SQL schema migration and `sqlite-vec` query optimization.
- **Build Engineer**: For `build.rs` and static linking of C extensions.

## Planning Documents
- [Analysis](./planning/analysis.md)
- [Architecture](./planning/architecture.md)
- [Quality Strategy](./planning/quality-strategy.md)
- [Security Review](./planning/security-review.md)
- [Implementation Plan](./planning/plan.md)
