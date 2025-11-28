# Ticket: SQLIMPL-4001: Implement Context Cache

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Phase 4 - OPTIONAL ENHANCEMENT:** This ticket is part of the optional context assembly phase. Defer if timeline pressure.

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the context cache for storing and retrieving assembled contexts. This speeds up repeated context lookups by caching the expansion results.

## Background
The context cache at `src/context/cache.rs` has 8 stubbed methods. These provide basic cache operations for the context assembly system.

This ticket implements Plan Phase 4, Ticket 4001: "Implement Context Cache".

## Acceptance Criteria
- [x] **Verify schema:** Confirm `context_cache` table exists (or create if needed)
- [x] `get()` retrieves cached context by key
- [x] `put()` stores context with TTL
- [x] `invalidate()` deletes by cache_key prefix
- [x] `evict_expired()` removes expired entries
- [x] `evict_lru_if_needed()` enforces size limits
- [x] All 8 cache methods implemented
- [x] Cache tests (from Phase 1) pass

## Technical Requirements
- Use `SqliteStore::run()` pattern for database access
- Store chunk IDs as JSON array in cache
- TTL stored as ISO8601 timestamp
- LRU eviction based on access time

## Implementation Notes

### Current Stubs (8 methods)
```rust
// src/context/cache.rs
// get(), put(), invalidate()
// evict_expired(), evict_lru_if_needed()
// Plus 3 additional helper methods
```

### Schema (verify or create)
```sql
CREATE TABLE IF NOT EXISTS context_cache (
    cache_key TEXT PRIMARY KEY,
    chunk_ids TEXT,        -- JSON array of chunk IDs
    created_at TEXT,       -- ISO8601 timestamp
    expires_at TEXT,       -- ISO8601 timestamp
    accessed_at TEXT       -- ISO8601 timestamp (for LRU)
);
```

### Target Implementation Patterns

#### Get
```rust
pub async fn get(&self, key: &str) -> Result<Option<CachedContext>> {
    let key = key.to_string();
    self.store.run(move |conn| {
        let result = conn.query_row(
            "SELECT chunk_ids, created_at, expires_at FROM context_cache
             WHERE cache_key = ? AND expires_at > datetime('now')",
            [&key],
            |row| {
                let chunk_ids_json: String = row.get(0)?;
                let created_at: String = row.get(1)?;
                Ok(CachedContext {
                    chunk_ids: serde_json::from_str(&chunk_ids_json)?,
                    created_at,
                })
            }
        ).optional()?;

        // Update accessed_at for LRU
        if result.is_some() {
            conn.execute(
                "UPDATE context_cache SET accessed_at = datetime('now') WHERE cache_key = ?",
                [&key]
            )?;
        }

        Ok(result)
    }).await
}
```

#### Put
```rust
pub async fn put(&self, key: &str, context: &CachedContext, ttl_seconds: i64) -> Result<()> {
    let key = key.to_string();
    let chunk_ids_json = serde_json::to_string(&context.chunk_ids)?;

    self.store.run(move |conn| {
        conn.execute(
            "INSERT OR REPLACE INTO context_cache
             (cache_key, chunk_ids, created_at, expires_at, accessed_at)
             VALUES (?, ?, datetime('now'), datetime('now', '+' || ? || ' seconds'), datetime('now'))",
            params![key, chunk_ids_json, ttl_seconds]
        )?;
        Ok(())
    }).await
}
```

#### Evict Expired
```rust
pub async fn evict_expired(&self) -> Result<usize> {
    self.store.run(|conn| {
        let count = conn.execute(
            "DELETE FROM context_cache WHERE expires_at < datetime('now')",
            []
        )?;
        Ok(count)
    }).await
}
```

## Dependencies
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Schema may not exist
  - **Mitigation**: Verify and create if needed as first step
- **Risk**: Cache invalidation patterns unclear
  - **Mitigation**: Use prefix-based invalidation for flexibility

## Files/Packages Affected
- `crates/maproom/src/context/cache.rs` (primary)
- `crates/maproom/migrations/` (if schema needs creation)
