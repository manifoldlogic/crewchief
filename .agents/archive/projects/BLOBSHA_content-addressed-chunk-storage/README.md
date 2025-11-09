# BLOBSHA: Content-Addressed Chunk Storage

**Status**: Planning Complete
**Slug**: BLOBSHA
**Timeline**: 6-7 days
**Dependencies**: None
**Blocks**: BRANCHX (Branch-Aware Indexing), BRWATCH (Branch Switch Detection)

## Problem Statement

The current Maproom indexing system stores embeddings directly in the `chunks` table, causing massive duplication when the same code exists across branches or gets refactored between files. This results in:

- **80% wasted embedding costs** ($8 per index cycle for typical 10-branch setup)
- **3x storage bloat** (3GB vs 840MB for 10 branches)
- **Unnecessary API calls** when moving functions between files
- **Slow branch operations** (regenerating embeddings for unchanged code)

## Proposed Solution

Implement **content-addressed chunk storage** using Git's blob SHA algorithm:

1. Compute SHA-256 hash of each chunk's content
2. Store embeddings in deduplicated table keyed by content hash
3. Chunks reference embeddings via `blob_sha` foreign key
4. Cache embeddings: identical content = cache hit, no API call

**Key insight**: Git applies blob SHA to whole files. We apply it to tree-sitter chunks for fine-grained deduplication.

## Success Metrics

- **Zero data loss** during migration (all embeddings preserved)
- **70-90% deduplication** for typical branch overlaps
- **Query performance** within 10% of baseline
- **Cache hit rate** measurable and accurate

## Project Boundaries

### In Scope
- Blob SHA computation (Rust + PostgreSQL)
- Database schema migration (4 phases)
- Deduplicated embedding storage
- Cache-aware upsert logic
- Query updates to use JOIN
- Performance testing and validation

### Out of Scope
- Branch tracking with JSONB worktree_ids → **BRANCHX project**
- Incremental updates using git diff-tree → **BRANCHX project**
- Automatic branch switch detection → **BRWATCH project**
- Search query modifications for branch filtering → Future work

## Architecture Overview

### Current Schema
```sql
CREATE TABLE chunks (
  chunk_id UUID PRIMARY KEY,
  file_id INT,
  embedding vector(1536),  -- Duplicated!
  content TEXT
);
```

### New Schema
```sql
-- Deduplicated embedding cache
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536),
  model_version TEXT,
  created_at TIMESTAMP
);

-- Chunks reference embeddings by content hash
CREATE TABLE code_chunks (
  chunk_id UUID PRIMARY KEY,
  blob_sha TEXT REFERENCES code_embeddings(blob_sha),
  file_path TEXT,
  content TEXT
);
```

**Key benefit**: Same content = same blob SHA = one embedding shared across all instances

## Implementation Phases

### Phase 1: Blob SHA Foundation (Days 1-2)
- Implement `compute_blob_sha()` in Rust
- Create PostgreSQL function `compute_git_blob_sha()`
- Add `blob_sha` column to chunks table
- Backfill existing chunks

**Deliverable**: All chunks have blob SHA values

### Phase 2: Code Embeddings Table (Days 3-4)
- Create `code_embeddings` table
- Migrate embeddings (deduplicated)
- Add foreign key constraint
- Create HNSW vector index

**Deliverable**: Embeddings deduplicated and accessible

### Phase 3: Application Integration (Day 5)
- Update search queries to JOIN with code_embeddings
- Implement cache-aware upsert logic
- Add cache hit/miss metrics
- Verify query equivalence

**Deliverable**: All queries using new schema, cache working

### Phase 4: Cleanup (Day 6)
- Drop old `embedding` column from chunks
- Reclaim disk space (VACUUM)
- Final performance validation
- Documentation

**Deliverable**: Migration complete, storage reclaimed

## Testing Strategy

### Critical Path Tests (Run on Every Commit)
1. `test_blob_sha_deterministic` - Cache correctness depends on this
2. `test_cache_hit_duplicate_content` - Core deduplication behavior
3. `test_search_query_equivalence` - Backward compatibility
4. `test_no_data_loss` - Zero data loss requirement

### Test Pyramid
- **70% Unit tests** - Blob SHA computation, pure functions
- **25% Integration tests** - Database migrations, cache behavior
- **5% E2E tests** - Full workflow, performance validation

### Performance Benchmarks
- Search latency must be within 10% of baseline
- Migration must complete in <1 hour for 1M chunks
- Cache hit rate 70-90% for branch overlaps

## Agent Assignments

1. **rust-indexer-engineer** - Blob SHA computation, upsert logic, cache metrics
2. **database-engineer** - Migration scripts, schema changes, PostgreSQL function
3. **unit-test-runner** - Execute tests after each phase, report results
4. **verify-ticket** - Final verification of acceptance criteria
5. **commit-ticket** - Create commit for completed work

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Migration breaks queries | High | Extensive testing, gradual rollout |
| Data loss during migration | Critical | Backups, transaction-based migration, validation |
| Performance regression | Medium | Benchmark before/after, rollback plan |
| Downtime during migration | Medium | Use CONCURRENTLY, batch updates, maintenance window |

**Rollback plan**: Documented for each phase, backup taken before Phase 1

## Project Links

### Planning Documents
- [Analysis](./planning/analysis.md) - Problem definition and research
- [Architecture](./planning/architecture.md) - Database schema and migration strategy
- [Quality Strategy](./planning/quality-strategy.md) - Testing approach
- [Security Review](./planning/security-review.md) - Security considerations
- [Plan](./planning/plan.md) - Detailed implementation plan

### Related Projects
- **BRANCHX** (Next) - Branch-aware indexing with worktree tracking
- **BRWATCH** (Future) - Automatic branch switch detection

### Research References
- `.agents/research/branch-aware-indexing-architecture.md` - Original design document
- `.agents/research/branch-aware-indexing-industry-research.md` - Industry validation

## Expected Outcomes

### Cost Savings
**Example: 10 branches, 80% code overlap**
- Without dedup: $10.00 (500k embeddings)
- With dedup: $2.00 (100k embeddings)
- **Savings: $8.00 per index cycle (80%)**

### Storage Savings
- Without dedup: 3GB (500k embeddings × 6KB)
- With dedup: 840MB (100k embeddings × 6KB)
- **Savings: 2.16GB (72%)**

### Performance Improvements
- Branch reindex: 10 minutes → 20 seconds (6x faster)
- Return to cached branch: <1 second (near-instant)
- Query latency: Maintained (within 10%)

## Acceptance Criteria

Project is complete when:

- [ ] All phases (1-4) complete
- [ ] All tests passing (unit + integration + E2E)
- [ ] Performance benchmarks within targets
- [ ] Documentation updated
- [ ] Manual smoke test successful
- [ ] Cache metrics showing expected behavior (70-90% hit rate)
- [ ] Deduplication verified (embedding count < chunk count)
- [ ] Zero data loss verified (all chunks have embeddings)
- [ ] Backup taken and rollback procedure tested

**Timeline**: Estimated 6-7 working days (1 buffer day)

---

**Next Steps**: Generate tickets using `/create-project-tickets BLOBSHA`
