# Architecture: vec_chunks Schema Fix

## Current State

### Database Schema (Post-Migration 9)

```
code_embeddings          vec_code               vec_code_768
┌─────────────────┐     ┌──────────────┐       ┌──────────────┐
│ id (PK)         │─────│ rowid        │       │ rowid        │
│ blob_sha (UK)   │     │ embedding    │       │ embedding    │
│ embedding       │     │ float[1536]  │       │ float[768]   │
│ embedding_dim   │     └──────────────┘       └──────────────┘
│ model_version   │            │                      │
│ created_at      │            │                      │
└─────────────────┘            │                      │
       │                       │                      │
       └───────────────────────┴──────────────────────┘
                      rowid = code_embeddings.id
```

### Problem: Orphaned Code Path

```
mod.rs (DEPRECATED)                embeddings.rs (CORRECT)
┌─────────────────────┐           ┌─────────────────────┐
│ upsert_embedding()  │           │ upsert_embedding()  │
│       ↓             │           │       ↓             │
│  vec_chunks (!)     │           │  code_embeddings    │
│   [DROPPED]         │           │       ↓             │
└─────────────────────┘           │ sync_embedding_to_  │
                                  │   vec()             │
                                  │       ↓             │
                                  │ vec_code or         │
                                  │ vec_code_768        │
                                  └─────────────────────┘
```

## Target Architecture

### Embedding Storage Flow

```
Indexer
   │
   ▼
┌───────────────────────────────────────────────────────┐
│                    SqliteStore                         │
│                                                        │
│  store_embedding(chunk_id, blob_sha, embedding)       │
│       │                                                │
│       ▼                                                │
│  ┌─────────────────────────────────────────────────┐  │
│  │              embeddings.rs                       │  │
│  │                                                  │  │
│  │  1. upsert_embedding(blob_sha, embedding, model) │  │
│  │       └─► INSERT/UPDATE code_embeddings          │  │
│  │                                                  │  │
│  │  2. sync_embedding_to_vec(id, embedding)         │  │
│  │       └─► INSERT vec_code / vec_code_768         │  │
│  └─────────────────────────────────────────────────┘  │
│                                                        │
│  get_embedding_for_chunk(chunk_id)                    │
│       │                                                │
│       ▼                                                │
│  JOIN chunks → code_embeddings ON blob_sha            │
└───────────────────────────────────────────────────────┘
```

## Solution Design

### 1. Remove Deprecated Code

**Files to modify:**
- `src/db/sqlite/mod.rs` - Remove functions referencing `vec_chunks`
- `src/db/sqlite/schema.rs` - Remove `vec_chunks` table creation

### 2. Update Public API

The `SqliteStore` trait should expose embedding operations via the correct module:

```rust
// In mod.rs - delegate to embeddings module
pub async fn store_embedding(
    &self,
    blob_sha: &str,
    embedding: &[f32],
    model_version: &str,
) -> Result<i64> {
    self.run(move |conn| {
        embeddings::upsert_embedding(conn, blob_sha, embedding, model_version)
    }).await
}

pub async fn sync_embedding(
    &self,
    embedding_id: i64,
    embedding: &[f32],
) -> Result<()> {
    self.run(move |conn| {
        embeddings::sync_embedding_to_vec(conn, embedding_id, embedding)
    }).await
}
```

### 3. Pipeline Migration (CRITICAL)

The `embedding/pipeline.rs` module calls the deprecated method. Here's the concrete migration:

**Current code (pipeline.rs:509-549):**
```rust
async fn update_chunk_embeddings(
    &self,
    store: &SqliteStore,
    chunk_id: i64,
    code_embedding: &[f32],
    text_embedding: &[f32],
) -> Result<()> {
    store
        .upsert_embeddings(chunk_id, Some(code_embedding), Some(text_embedding), self.dimension)
        .await?;  // ← DEPRECATED: writes to vec_chunks
    Ok(())
}
```

**Migration approach - Option A (Remove method entirely):**

The `update_chunk_embeddings()` method is redundant. At the call site (line 424-437), `populate_embedding_cache()` is already called with the correct API. We can:

1. **Remove** `update_chunk_embeddings()` entirely
2. **Consolidate** embedding storage into `populate_embedding_cache()` which already uses the correct `store.upsert_embedding(blob_sha, ...)` API

**Current call site (pipeline.rs:424-437):**
```rust
for (i, chunk) in batch.iter().enumerate() {
    self.update_chunk_embeddings(store, chunk.id, &code_embeddings[i], &text_embeddings[i]).await?;

    // Already uses correct API!
    if let Some(blob_sha) = &chunk.blob_sha {
        self.populate_embedding_cache(store, blob_sha, &code_embeddings[i]).await?;
    }
}
```

**After migration:**
```rust
for (i, chunk) in batch.iter().enumerate() {
    // Store embedding by content hash (deduplication)
    if let Some(blob_sha) = &chunk.blob_sha {
        store.upsert_embedding(blob_sha, &code_embeddings[i], &self.provider_name).await?;
    }
}
```

**Key insight**: The deprecated `upsert_embeddings()` stored by `chunk_id`, but the new architecture stores by `blob_sha` for deduplication. The `populate_embedding_cache()` method already does this correctly - we just need to remove the redundant deprecated call.

### 4. Vec Extension Flags

The `vec_available` and `vec_checked` flags in `SqliteStore` are used by both deprecated and new code paths. After removing the deprecated functions:

- Keep the flags - they're still used by `sync_embedding_to_vec()` in `embeddings.rs`
- The new `SqliteStore::upsert_embedding()` method (line 1778) uses them correctly

## Design Decisions

### Decision 1: Remove vs. Replace

**Choice: Remove deprecated functions entirely**

Rationale:
- The `embeddings.rs` module is complete and tested
- No reason to maintain two implementations
- Removing dead code reduces maintenance burden

### Decision 2: Schema Cleanup

**Choice: Remove legacy `vec_chunks` from schema.rs**

Rationale:
- Schema.rs is only used for reference/testing
- Migration 6 handles dropping it in production databases
- Keeping it creates confusion

### Decision 3: API Compatibility

**Choice: Update public API signatures if needed**

Rationale:
- The old functions had different signatures
- Better to fix callers now than maintain compatibility shims
- All callers are internal to this crate

## Files Changed

| File | Change |
|------|--------|
| `src/db/sqlite/mod.rs` | Remove `upsert_embeddings()` and `batch_upsert_embeddings()` functions (lines 478-620) |
| `src/db/sqlite/schema.rs` | Remove `vec_chunks` table creation (line 99) |
| `src/embedding/pipeline.rs` | Remove `update_chunk_embeddings()` method, consolidate to `populate_embedding_cache()` |

**Note**: After cleanup, only `upsert_embedding()` (singular, at line 1778) should remain - this uses the correct `embeddings.rs` module.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing callers | Medium | Medium | Search for all usages before removal |
| Database inconsistency | Low | Low | Migration already handles schema |
| Test failures | Medium | Low | Update/remove tests referencing `vec_chunks` |

## Non-Goals

- **Schema migration**: Not needed - migration 6 already drops `vec_chunks`
- **Data migration**: Not needed - any data in `vec_chunks` is from fresh installs only (migrations run sequentially)
- **Backward compatibility**: Not needed - this is internal code
