# Analysis: Embedding Inheritance Gap

## Problem Discovery

During genetic optimizer testing, variant worktree scans took hours despite:
- Base branch already indexed (416K chunks)
- Only 1 file different between variants
- BLOBSHA + BRANCHX infrastructure in place

Database investigation revealed:
- 670K total chunks, only 58K unique blob SHAs (88% duplication)
- 42K chunks with NULL embeddings
- Embedding pipeline generating new embeddings instead of reusing existing ones

## Root Cause

The embedding generation flow is disconnected from the deduplication cache:

1. **Scan phase**: `upsert_chunk_with_cache()` checks if embedding exists (for metrics), but inserts chunks with NULL embeddings
2. **Embedding phase**: `auto_generate_embeddings()` finds NULL embeddings and generates new ones via API calls
3. **Missing step**: No lookup of existing embeddings from `code_embeddings` table

## Evidence from Codebase

**upsert.rs:117-118**:
```rust
// Note: Actual embedding generation and insertion into code_embeddings
// happens in the embedding pipeline (not in this upsert path).
```

**main.rs:324**:
```rust
"SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NULL OR text_embedding IS NULL"
```

The pipeline only checks for NULL, never checks `code_embeddings` by blob SHA.

## Expected vs Actual Behavior

**Expected** (per BLOBSHA plan):
- Chunk with blob_sha "abc123" inserted
- Embedding exists in `code_embeddings` for "abc123"
- Pipeline copies embedding to chunk → instant
- No API call needed

**Actual**:
- Chunk with blob_sha "abc123" inserted with NULL embedding
- Pipeline sees NULL embedding
- Generates new embedding via API → slow, expensive
- Two embeddings for same content

## Impact Analysis

**Performance**:
- ~42K unnecessary API calls per full scan
- Hours instead of seconds for worktree scans
- Genetic optimizer impractical (5 variants = 5× slow scans)

**Cost**:
- Duplicate embedding generation costs
- Wasted compute resources

**User Experience**:
- Branch switching slow
- Worktree creation slow
- Competition framework unusable

## Industry Comparison

Content-addressed storage systems (Git, Docker, etc.) always check cache before computation. Our implementation checks for metrics but doesn't use the result.

## Conclusion

The BLOBSHA infrastructure is 90% complete but missing the critical "read from cache" step. Fix is straightforward: add embedding lookup before generation.
