# Implementation Plan: Embedding Inheritance Fix

## Overview

Single-phase project to add embedding inheritance from `code_embeddings` table before generation.

**Timeline**: 1-2 hours
**Complexity**: Low - surgical change to existing pipeline

## Phase 1: Implementation

### Ticket 1: Add embedding copy step to pipeline

**Agent**: rust-indexer-engineer

**Files**:
- `crates/maproom/src/embedding/pipeline.rs`

**Changes**:

1. Add `copy_existing_embeddings()` method to `EmbeddingPipeline`:
   ```rust
   async fn copy_existing_embeddings(&self, client: &Client) -> Result<usize>
   ```

2. Implement SQL query:
   ```sql
   UPDATE maproom.chunks c
   SET
       code_embedding = ce.code_embedding,
       text_embedding = ce.text_embedding,
       updated_at = NOW()
   FROM maproom.code_embeddings ce
   WHERE c.blob_sha = ce.blob_sha
     AND (c.code_embedding IS NULL OR c.text_embedding IS NULL)
   RETURNING c.id
   ```

3. Call copy before generation in `run()` method

4. Update `PipelineStats` to track:
   - `copied_from_cache: usize`
   - `cost_saved_usd: f64`

**Acceptance**:
- [ ] Copy function implemented
- [ ] Stats updated
- [ ] Called before generation
- [ ] Compiles without errors

---

### Ticket 2: Add unit tests

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/embedding/pipeline.rs` (inline tests)

**Tests**:

1. `test_copy_existing_embeddings_success`
   - Setup: Insert test chunk + code_embeddings entry
   - Assert: Chunk gets embeddings, count = 1

2. `test_copy_skips_without_cache`
   - Setup: Chunk without cache entry
   - Assert: Still NULL, count = 0

3. `test_copy_idempotent`
   - Setup: Chunk already has embeddings
   - Assert: No change, no error

**Acceptance**:
- [ ] All 3 tests written
- [ ] Tests pass: `cargo test copy_existing`

---

### Ticket 3: Add integration test

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/tests/embedding_inheritance_test.rs` (new)

**Test**: `test_variant_worktree_embedding_copy`

1. Scan base worktree (generates embeddings)
2. Create variant worktree (1 file different)
3. Scan variant worktree
4. Assert:
   - Scan completes quickly (< 10s)
   - Stats show high copy count, low generation count
   - Variant chunks have embeddings

**Acceptance**:
- [ ] Integration test written
- [ ] Test passes: `cargo test embedding_inheritance`
- [ ] Demonstrates speedup

---

### Ticket 4: Validation & verification

**Agent**: verify-ticket (manual validation first)

**Manual steps**:

1. Run genetic optimizer test:
   ```bash
   cd packages/cli
   npx tsx scripts/run-genetic-optimizer-ultra.ts
   ```

2. Observe:
   - Variant scans complete quickly (not hours)
   - Embedding stats show copies, not generation
   - Competition completes successfully

**Verification**:
- [ ] Genetic optimizer runs successfully
- [ ] Variant scans < 10 seconds each
- [ ] No regression in base branch scan
- [ ] Stats show embedding reuse

---

### Ticket 5: Commit

**Agent**: commit-ticket

**Message**:
```
fix(indexer): copy embeddings from cache before generation

Before generating embeddings for chunks with NULL values, check if
an embedding already exists in code_embeddings for that blob SHA
and copy it. This eliminates duplicate embedding generation when
scanning variant worktrees.

Performance impact:
- Variant worktree scans: hours → seconds (200-500× faster)
- API cost reduction: ~400× for typical branch switches
- Genetic optimizer: now practical (minutes not hours)

Implements missing step from BLOBSHA deduplication infrastructure.
```

**Acceptance**:
- [ ] All tests pass
- [ ] Validation complete
- [ ] Changes committed
- [ ] Project archived

## Success Metrics

- ✅ Variant worktree scans complete in < 10 seconds
- ✅ Embedding copy count > 99% for variant scans
- ✅ Genetic optimizer runs successfully
- ✅ No regression in base branch performance

## Rollback Plan

If issues occur:
1. Revert commit
2. Pipeline falls back to generation (existing behavior)
3. No data loss - embeddings unchanged

Low risk due to:
- Small, isolated change
- Backward compatible
- Well-tested

## Notes

This is the missing piece from BLOBSHA. The infrastructure existed but wasn't being used during embedding generation. Simple fix, huge impact.
