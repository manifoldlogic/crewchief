# Analysis: vec_chunks Schema Migration Issue

## Problem Definition

The Maproom Rust indexer has a **schema consistency bug** where code in `mod.rs` still references the `vec_chunks` table that was dropped by migration 6. This causes runtime errors ("no such table: vec_chunks") when the embedding storage functions are called.

### Root Cause

The codebase underwent a schema evolution:

1. **Migration 1**: Created `vec_chunks` table for storing chunk-level embeddings
2. **Migration 3**: Introduced `code_embeddings` table for deduplicated storage by blob SHA
3. **Migration 4**: Created `vec_code` table (1536-dim) as the new vector index
4. **Migration 6**: **Dropped `vec_chunks`** table as deprecated
5. **Migration 7**: Added `vec_code_768` for 768-dimensional embeddings (Ollama)

However, the code in `src/db/sqlite/mod.rs` (lines 503-609) was never updated to use the new schema. It still contains:
- Functions that INSERT/UPDATE/SELECT from `vec_chunks`
- Logic that references `vec_chunks` for embedding storage

### Current Code Paths

**Broken code (mod.rs:478-620)**:
- `upsert_embeddings()` (plural) - attempts to write to `vec_chunks`
- `batch_upsert_embeddings()` - batch operations on `vec_chunks`

**Correct code (embeddings.rs)**:
- `upsert_embedding()` (singular) - writes to `code_embeddings` table
- `sync_embedding_to_vec()` - syncs to `vec_code` or `vec_code_768`
- `sync_all_embeddings_to_vec()` - batch sync to vector tables

### Active Callers (CRITICAL)

The deprecated `upsert_embeddings()` method has an **active caller**:

| Caller | File | Line | Usage |
|--------|------|------|-------|
| `update_chunk_embeddings()` | `embedding/pipeline.rs` | 527 | Called for each chunk during embedding generation |

**Code path:**
```
EmbeddingPipeline::run_with_progress()
  └─► process_batch()
        └─► update_chunk_embeddings(store, chunk.id, code_emb, text_emb)
              └─► store.upsert_embeddings(chunk_id, ...)  ← DEPRECATED, uses vec_chunks
```

This is the **primary code path** for generating embeddings during scans. The deprecated method cannot simply be removed without updating this caller.

**Key insight**: The caller has access to `chunk.blob_sha`, which is needed for the new content-centric storage model. See `pipeline.rs:434` where it's already used for `populate_embedding_cache()`.

### Schema Files Affected

| File | Issue |
|------|-------|
| `src/db/sqlite/mod.rs` | Contains deprecated `vec_chunks` operations (lines 503-609) |
| `src/db/sqlite/schema.rs` | Creates `vec_chunks` table in legacy schema (line 99) - should be removed |

## Existing Solution

The correct embedding storage architecture already exists in `embeddings.rs`:

1. **Deduplication**: Store embeddings in `code_embeddings` by `blob_sha`
2. **Dimension Support**: 768-dim (Ollama) and 1536-dim (OpenAI/Vertex)
3. **Vector Index**: Sync to `vec_code` or `vec_code_768` for similarity search
4. **Batch Operations**: Efficient batch upsert with transaction

## Impact Analysis

### Symptoms
- VSCode extension fails to scan workspaces
- Error: "no such table: vec_chunks"
- Tracing shows errors during embedding generation

### Affected Functionality
- Initial workspace scans
- Embedding generation and storage
- Vector search (if embeddings don't get stored)

### Not Affected
- Database migrations (work correctly)
- FTS search (uses `fts_chunks`)
- Hybrid search fallback to FTS

## Research Findings

### Why Two Implementations Exist

The `mod.rs` implementation appears to be legacy code that predates the `embeddings.rs` refactor. The `embeddings.rs` module was created to:

1. Support blob-SHA deduplication (same code = one embedding)
2. Handle multiple embedding dimensions
3. Properly separate storage (code_embeddings) from indexing (vec_code)

### Schema Intent

The migration history shows clear intent:
- **Migration 4**: "Create vector index table using sqlite-vec"
- **Migration 6**: "Drop the deprecated vec_chunks virtual table"

The `vec_chunks` table was designed for direct chunk-to-embedding mapping, while `vec_code` tables use `rowid` matching with `code_embeddings.id` for efficient joins.

## Conclusion

This is a **technical debt issue** where deprecated code wasn't cleaned up after a schema migration. The fix is straightforward:

1. Remove or replace the deprecated `vec_chunks` code in `mod.rs`
2. Update callers to use `embeddings.rs` functions
3. Remove legacy schema creation in `schema.rs`
4. Ensure all embedding paths use the correct deduplicated architecture
