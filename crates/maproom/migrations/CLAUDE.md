# Database Migrations

**WARNING**: The `.sql` files in this directory are **legacy**. They are NOT loaded at runtime.

## Where Migrations Actually Live

All migrations are defined as Rust structs in `crates/maproom/src/db/sqlite/migrations.rs`, inside the `migrations()` function. Each migration is a `Migration` struct with inline SQL in the `up` field.

**There is no `queries.rs` file.** The migration runner is in `migrations.rs`.

## Critical Pitfalls

- **No SQL files at runtime**: Adding a `.sql` file here does nothing. You must add a `Migration` struct in Rust.
- **SQLite syntax only**: No `JSONB`, `TIMESTAMPTZ`, `UUID`, `CONCURRENTLY`, or schema prefixes (`maproom.tablename`). This is SQLite, not PostgreSQL.
- **No schema prefix**: Use `CREATE TABLE chunks`, never `maproom.chunks`
- **Sequential versions**: No gaps allowed in version numbers

## Full Workflow

See `.claude/docs/migration-workflow.md` for the complete step-by-step process.

See also: `docs/architecture/DATABASE_ARCHITECTURE.md`
