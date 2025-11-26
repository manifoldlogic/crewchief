# Ticket: VECSTORE-1000: SQLite 768-dim Embedding Support

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Enable SQLite to store and search 768-dimensional embeddings (Ollama/nomic-embed-text) in addition to the existing 1536-dimensional embeddings (OpenAI).

**Priority**: CRITICAL - Blocks EMBPERF project and zero-config experience (SQLite + Ollama).

## Background
SQLite is currently hardcoded to only accept 1536-dimensional embeddings (OpenAI). This blocks the "zero-config" promise where users can use SQLite (no PostgreSQL setup) with Ollama (no API keys required).

**Current Problem** (in `crates/maproom/src/db/sqlite/embeddings.rs`):
```rust
if embedding.len() != 1536 {
    anyhow::bail!(
        "Unsupported embedding dimension: {}. Only 1536-dimensional embeddings are currently supported.",
        embedding.len()
    );
}
```

**Impact**:
- Zero-config (SQLite + Ollama) doesn't work
- EMBPERF project (Ollama parallel optimization) can't benefit SQLite users
- PostgreSQL already supports both 768 and 1536 dimensions

**Reference**: Plan Phase 0 - SQLite Multi-Dimension Support (CRITICAL)

## Acceptance Criteria
- [ ] SQLite migration creates 768-dim virtual tables (`vec_code_embeddings_768`, `vec_text_embeddings_768`)
- [ ] `upsert_embeddings(..., dimension=768)` succeeds without errors
- [ ] `upsert_embeddings(..., dimension=1536)` continues to work (backward compatibility)
- [ ] `search_chunks_vector()` works with 768-dim query embedding
- [ ] `search_chunks_hybrid()` works with 768-dim embedding
- [ ] Existing 1536-dim data and functionality unaffected
- [ ] Unit tests pass with `cargo test --features sqlite --lib db::sqlite`

## Technical Requirements

### Schema Changes
Add new sqlite-vec virtual tables for 768-dimensional embeddings:

```sql
-- 768-dim vector table (NEW)
CREATE VIRTUAL TABLE IF NOT EXISTS vec_code_embeddings_768 USING vec0(
    embedding float[768]
);

CREATE VIRTUAL TABLE IF NOT EXISTS vec_text_embeddings_768 USING vec0(
    embedding float[768]
);

-- Track dimension in metadata table
ALTER TABLE code_embeddings ADD COLUMN embedding_dim INTEGER NOT NULL DEFAULT 1536;
```

### Query Routing Function
Create a helper function to route to the correct table based on dimension:

```rust
// In sqlite/embeddings.rs or new sqlite/dimensions.rs
fn get_vec_table_name(base_name: &str, dimension: usize) -> String {
    match dimension {
        768 => format!("{}_768", base_name),
        1536 => base_name.to_string(),
        _ => panic!("Unsupported embedding dimension: {}", dimension),
    }
}
```

### Files to Modify

1. **`crates/maproom/src/db/sqlite/schema.rs`**:
   - Add 768-dim virtual table DDL
   - Update migration to create new tables

2. **`crates/maproom/src/db/sqlite/embeddings.rs`**:
   - Remove hardcoded 1536 validation
   - Add `get_vec_table_name()` routing function
   - Update `upsert_embedding()` to route by dimension
   - Update `batch_upsert_embeddings()` to handle mixed dimensions

3. **`crates/maproom/src/db/sqlite/vector.rs`**:
   - Update `search_vector()` to query correct table based on embedding size

4. **`crates/maproom/src/db/sqlite/hybrid.rs`**:
   - Update hybrid search to work with 768-dim embeddings

## Implementation Notes

### Approach
1. First, add the new virtual tables via schema/migration
2. Create dimension routing helper function
3. Update upsert functions to accept both dimensions
4. Update search functions to query the correct table
5. Add comprehensive tests for both dimensions

### Key Considerations
- sqlite-vec requires fixed dimensions per virtual table - cannot have dynamic dimensions
- Embedding dimension is determined by the provider (Ollama=768, OpenAI=1536)
- The `dimension` parameter already exists in trait methods - just need to honor it
- Search queries must match the embedding dimension to the correct table

### PostgreSQL Reference
PostgreSQL handles this with separate columns:
- `code_embedding` (1536-dim, OpenAI)
- `code_embedding_ollama` (768-dim, Ollama)
- `select_columns_for_dimension()` in `columns.rs` routes queries

### Testing Strategy
1. Test 768-dim storage and retrieval
2. Test 1536-dim continues to work (regression)
3. Test search with 768-dim query embedding
4. Test that searching wrong dimension returns empty (not error)
5. Test mixed dimension data in same database

## Dependencies
- None - This is the foundation ticket for VECSTORE

## Risk Assessment
- **Risk**: Breaking existing 1536-dim functionality
  - **Mitigation**: Default to 1536 for backward compatibility, comprehensive regression tests
- **Risk**: Migration fails on existing databases
  - **Mitigation**: New tables are additive, don't modify existing tables

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/schema.rs`
- `crates/maproom/src/db/sqlite/embeddings.rs`
- `crates/maproom/src/db/sqlite/vector.rs`
- `crates/maproom/src/db/sqlite/hybrid.rs`
- `crates/maproom/src/db/sqlite/migrations.rs` (if separate from schema)
