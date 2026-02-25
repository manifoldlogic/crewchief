# SQLite Migration Workflow

Migrations are defined **in Rust code**, not in SQL files. The `.sql` files in `crates/maproom/migrations/` are legacy and not loaded at runtime.

## Where Migrations Live

**Source of truth**: `crates/maproom/src/db/sqlite/migrations.rs` — the `migrations()` function returns a `Vec<Migration>`.

## Migration Struct

```rust
Migration {
    version: i32,       // Sequential, no gaps
    name: &'static str, // Descriptive name (e.g., "initial_schema")
    up: &'static str,   // SQL to apply (executed via execute_batch)
    down: &'static str, // Rollback SQL (best-effort, not currently used)
}
```

## Adding a New Migration

1. Open `crates/maproom/src/db/sqlite/migrations.rs`
2. Find the `migrations()` function and add a new `Migration` at the end of the vec
3. Use the next sequential version number
4. Write SQLite-compatible SQL in the `up` field
5. Run `cargo build -p crewchief-maproom` to verify
6. Test: `cargo test -p crewchief-maproom`

## SQLite Pitfalls (Not PostgreSQL!)

- **No schemas**: Use `CREATE TABLE chunks`, not `maproom.chunks`
- **No JSONB**: Use `TEXT` with JSON functions, or `JSON` type alias
- **No TIMESTAMPTZ**: Use `TEXT` with `datetime('now')` or `INTEGER` epoch
- **No CONCURRENTLY**: `CREATE INDEX` is always blocking
- **No UUID**: Use `INTEGER PRIMARY KEY AUTOINCREMENT`
- **IF NOT EXISTS**: Always use for idempotency

## Example

```rust
Migration {
    version: 22,
    name: "add_chunk_metadata",
    up: r#"
        ALTER TABLE chunks ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        CREATE INDEX IF NOT EXISTS idx_chunks_metadata ON chunks(metadata);
    "#,
    down: "ALTER TABLE chunks DROP COLUMN metadata;",
}
```

## Verification

```bash
cargo build -p crewchief-maproom              # Compiles migration SQL
cargo test -p crewchief-maproom -- migrate     # Run migration tests
cargo run --bin crewchief-maproom -- db migrate # Apply to real database
```

See also: `docs/architecture/DATABASE_ARCHITECTURE.md`
