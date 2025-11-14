# Quality Strategy: Embedding Inheritance

## Testing Philosophy

Focus on correctness and regression prevention. The change is small and critical - we need confidence it works.

## Test Coverage

### Unit Tests

**File**: `crates/maproom/src/embedding/pipeline.rs`

1. **Test: Copy from existing embeddings**
   - Setup: Insert chunk with NULL embeddings, matching entry in `code_embeddings`
   - Execute: Run copy function
   - Assert: Chunk has embeddings, stats show copy count

2. **Test: Skip chunks without cache entry**
   - Setup: Chunk with NULL embeddings, no `code_embeddings` entry
   - Execute: Run copy function
   - Assert: Chunk still NULL, stats show 0 copied

3. **Test: Idempotent behavior**
   - Setup: Chunk with embeddings already set
   - Execute: Run copy function
   - Assert: No change, no errors

### Integration Tests

**File**: `crates/maproom/tests/embedding_inheritance.rs`

1. **Test: Full pipeline with copy step**
   - Setup: Scan worktree, add variant worktree
   - Execute: Scan variant (triggers embedding copy)
   - Assert: Variant chunks have embeddings, no API calls made

2. **Test: Mixed cache hits and misses**
   - Setup: Some blobs in cache, some not
   - Execute: Run pipeline
   - Assert: Stats correctly split between copied and generated

### Regression Tests

**File**: Same as integration

1. **Test: Genetic optimizer scenario**
   - Setup: Base branch indexed, 5 variant worktrees
   - Execute: Scan all variants
   - Assert: Each completes in < 10 seconds (vs hours before)

## Performance Validation

Measure before/after on real crewchief repo:

**Metrics**:
- Time to scan variant worktree
- API calls made (should be ~0 for variants)
- Cost saved (track in stats)

**Target**: 200× speedup for variant scans

## Manual Testing Checklist

- [ ] Scan base branch (first time, slow)
- [ ] Scan variant worktree (should be fast)
- [ ] Check database: variants have embeddings
- [ ] Run genetic optimizer (should complete in minutes)
- [ ] Verify no duplicate embeddings generated

## Acceptance Criteria

1. Variant worktree scans complete in < 10 seconds (not hours)
2. Stats show embeddings copied from cache (not generated)
3. No regression in base branch scan performance
4. All unit tests pass
5. Integration test demonstrates 200× speedup
6. Genetic optimizer runs successfully

## What We're NOT Testing

- Embedding quality (unchanged)
- Search accuracy (unchanged)
- Schema migrations (no schema changes)
- Concurrent scan conflicts (out of scope)

Simple, focused, high-value tests.
