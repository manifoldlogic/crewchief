# Architecture: SQLite-Vec Backend

## 1. VectorStore Trait
We will define a trait in `src/db/store.rs`:

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn migrate(&self) -> Result<()>;
    
    // Core operations
    async fn upsert_file(&self, file: &File) -> Result<i64>;
    async fn insert_chunk(&self, chunk: &Chunk) -> Result<i64>;
    async fn search_chunks(&self, query: &VectorQuery) -> Result<Vec<SearchResult>>;
    
    // ... other methods
}
```

## 2. Component Structure

```
crates/maproom/src/
├── db/
│   ├── mod.rs           # Exports
│   ├── store.rs         # VectorStore trait definition
│   ├── postgres/        # Postgres implementation
│   │   ├── mod.rs
│   │   ├── store.rs     # impl VectorStore for PostgresStore
│   │   └── queries.rs   # Raw SQL for Postgres
│   └── sqlite/          # SQLite implementation
│       ├── mod.rs
│       ├── store.rs     # impl VectorStore for SqliteStore
│       └── queries.rs   # Raw SQL for SQLite
```

## 3. SQLite-Vec Integration
- Use `sqlite-vec` crate (if available/stable) or build from source in `build.rs`.
- Load extension on connection: `conn.load_extension("vec0", None)?`.
- Schema:
  ```sql
  CREATE VIRTUAL TABLE vec_chunks USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    embedding FLOAT[768]
  );
  ```

## 4. Dependency Injection
- `main.rs` reads config and instantiates `Box<dyn VectorStore>`.
- Passes this store to `Indexer`, `Searcher`, etc.

## 5. Migration Strategy
- **Postgres**: Keep existing `.sql` files.
- **SQLite**: Create new `migrations/sqlite/` directory. Use `user_version` pragma or a `migrations` table to track state.

