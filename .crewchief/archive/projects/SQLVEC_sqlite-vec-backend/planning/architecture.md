# Architecture: SQLite-Vec Backend

## 1. Core Abstraction: `VectorStore` Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    // ... (same as before) ...
}
```

## 2. Component Structure

```
crates/maproom/src/
├── db/
│   ├── mod.rs             # Trait definition
│   ├── query_builder.rs   # NEW: SQL dialect abstraction (FTS/Vector syntax)
│   ├── postgres/          # Moved current implementation
│   └── sqlite/            # New implementation
│       ├── mod.rs
│       ├── schema.rs      
│       └── connection.rs  # NEW: Connection pooling & WAL setup
├── build.rs               # Updated to compile sqlite-vec.c
```

## 3. SQLite Schema Strategy
- **Tables**: Mirror Postgres tables.
- **Vectors**: `vec0` virtual table. **Validation Required**: Ensure 1536-dim support.
- **FTS**: `FTS5` virtual table.
- **Concurrency**:
  - Enable `PRAGMA journal_mode=WAL`.
  - Use `r2d2` or `deadpool-sqlite` for connection pooling.
  - Enforce **serialized writes** (mutex or single-thread channel) if `SQLITE_BUSY` becomes an issue, but WAL should suffice for moderate concurrency.

## 4. Query Dialect Abstraction
We cannot reuse SQL strings.
- **Postgres**: `plainto_tsquery($1)`, `<->` operator.
- **SQLite**: `MATCH`, `vec_distance_cosine`.
- **Strategy**: The `VectorStore` implementation is responsible for constructing the correct SQL string. We will NOT try to genericize the SQL itself, but genericize the *interface* (`search`, `upsert`).

## 5. Build System Integration
- Vendor `sqlite-vec` source.
- Use `cc` crate.
- **Cross-Compilation**: Ensure `CRATE_CC_NO_DEFAULTS` or similar env vars are handled for target-specific compilers.
