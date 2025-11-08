# BLOBSHA Ticket Index

**Project**: BLOBSHA - Content-Addressed Chunk Storage
**Status**: Planning Complete, Ready for Implementation
**Total Tickets**: 10 tickets across 4 phases

## Overview

This index organizes all implementation tickets for the BLOBSHA project, which implements content-addressed chunk storage using Git blob SHA for 70-90% cost reduction through embedding deduplication.

## Execution Order

Tickets must be executed sequentially by phase. Each phase must complete and pass tests before proceeding to the next phase.

---

## Phase 1: Blob SHA Foundation (Days 1-2)

**Goal**: Implement blob SHA computation and add blob_sha column to database

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| BLOBSHA-1001 | Implement Rust Blob SHA Computation Function | rust-indexer-engineer | ⬜ Pending | None |
| BLOBSHA-1002 | Database Migration - Add Blob SHA Column | general-purpose | ⬜ Pending | BLOBSHA-1001 |
| BLOBSHA-1901 | Execute Phase 1 Test Suite | unit-test-runner | ⬜ Pending | BLOBSHA-1001, BLOBSHA-1002 |

**Deliverables**:
- ✅ Rust `compute_blob_sha()` function with 100% test coverage
- ✅ PostgreSQL `compute_git_blob_sha()` function matching Rust output
- ✅ `blob_sha` column added to chunks table
- ✅ All existing chunks backfilled with blob SHA values
- ✅ Deduplication metrics measured (baseline)

**Success Criteria**:
- All Phase 1 tests passing
- Zero NULL blob_sha values in chunks table
- Rust/SQL blob SHA outputs match for identical content

---

## Phase 2: Code Embeddings Table (Days 3-4)

**Goal**: Create deduplicated embedding storage and establish foreign key relationship

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| BLOBSHA-2001 | Create Code Embeddings Table and Migrate Data | general-purpose | ⬜ Pending | BLOBSHA-1901 (Phase 1 complete) |
| BLOBSHA-2002 | Execute Phase 2 Test Suite | unit-test-runner | ⬜ Pending | BLOBSHA-2001 |

**Deliverables**:
- ✅ `code_embeddings` table created with blob_sha PRIMARY KEY
- ✅ All unique embeddings migrated with deduplication
- ✅ HNSW vector index created for similarity search
- ✅ Foreign key constraint: chunks.blob_sha → code_embeddings.blob_sha
- ✅ Storage savings measured (70-90% reduction expected)

**Success Criteria**:
- Zero data loss (all blob_sha values have embeddings)
- Deduplication achieved (COUNT code_embeddings < COUNT chunks)
- HNSW index used for vector queries (verified via EXPLAIN ANALYZE)
- Foreign key enforces referential integrity

---

## Phase 3: Application Integration (Day 5)

**Goal**: Update queries and implement cache-aware upsert logic

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| BLOBSHA-3001 | Update Search Queries to JOIN code_embeddings | rust-indexer-engineer | ⬜ Pending | BLOBSHA-2002 (Phase 2 complete) |
| BLOBSHA-3002 | Implement Cache-Aware Upsert with Metrics | rust-indexer-engineer | ⬜ Pending | BLOBSHA-3001 |
| BLOBSHA-3901 | Execute Phase 3 Test Suite | unit-test-runner | ⬜ Pending | BLOBSHA-3001, BLOBSHA-3002 |

**Deliverables**:
- ✅ All vector search queries use JOIN with code_embeddings
- ✅ Cache-aware upsert checks blob SHA before generating embeddings
- ✅ CacheMetrics struct tracks hits/misses/cost
- ✅ Query performance within 10% of baseline
- ✅ Cache hit rate 70-90% for typical branch overlaps

**Success Criteria**:
- Query results identical to baseline (search equivalence test passes)
- Cache hit detection working (verified via integration tests)
- Performance benchmarks within targets (latency, throughput)
- Metrics accurately track cache behavior

---

## Phase 4: Cleanup and Optimization (Days 6-7)

**Goal**: Drop old embedding column, validate performance, and document architecture

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| BLOBSHA-4001 | Drop Old Embedding Column and Reclaim Storage | general-purpose | ⬜ Pending | BLOBSHA-3901 (Phase 3 complete) |
| BLOBSHA-4002 | Final Performance Validation and Documentation | general-purpose | ⬜ Pending | BLOBSHA-4001 |
| BLOBSHA-4901 | Execute Final Integration and Smoke Tests | unit-test-runner | ⬜ Pending | BLOBSHA-4001, BLOBSHA-4002 |

**Deliverables**:
- ✅ Old embedding column dropped from chunks table
- ✅ VACUUM FULL reclaims disk space (50%+ savings)
- ✅ Final performance benchmarks within targets
- ✅ Architecture documentation complete
- ✅ Migration guide written
- ✅ CHANGELOG updated

**Success Criteria**:
- Storage reclaimed (measured before/after)
- No chunks.embedding references remain in codebase
- Performance regression-free
- Documentation complete and accurate
- Manual smoke tests successful

---

## Critical Path Tests

These tests must pass on every commit:

1. `test_blob_sha_deterministic` - Cache correctness foundation
2. `test_cache_hit_duplicate_content` - Core deduplication behavior
3. `test_search_query_equivalence` - Backward compatibility
4. `test_no_data_loss` - Zero data loss requirement

---

## Project Success Metrics

The BLOBSHA project is complete when:

### Functional Requirements
- [x] Blob SHA computed for all chunks
- [x] Embeddings deduplicated in code_embeddings table
- [x] Cache hit/miss detection working
- [x] All queries return correct results
- [x] Foreign key integrity enforced

### Performance Requirements
- [x] Query latency within 10% of baseline
- [x] Cache hit rate 70-90% for typical branch overlap
- [x] Migration completes in reasonable time (<1 hour for 1M chunks)

### Quality Requirements
- [x] All unit tests passing
- [x] All integration tests passing
- [x] E2E search equivalence test passing
- [x] Zero data loss verified

### Documentation Requirements
- [x] Architecture documented
- [x] Migration guide written
- [x] Changelog updated

---

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
- Query latency: Maintained (within 10%)
- Cache hit rate: 70-90% for branch overlaps
- Storage efficiency: 50%+ reduction

---

## Agent Workflow

### Per-Phase Execution

```
Phase N:
  1. Implementation tickets (BLOBSHA-N00X)
     - rust-indexer-engineer or general-purpose
     - Write code/migrations
     - Write tests

  2. Test ticket (BLOBSHA-N90X)
     - unit-test-runner
     - Execute tests
     - Report results

  3. If tests fail → return to step 1
     If tests pass → proceed to next phase

Final Phase:
  1. verify-ticket
     - Check all acceptance criteria
     - Validate success metrics

  2. If any criteria fail → return to failing phase
     If all pass → commit-ticket
```

---

## Planning Document References

- **Analysis**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/analysis.md`
- **Architecture**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/architecture.md`
- **Quality Strategy**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/quality-strategy.md`
- **Security Review**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/security-review.md`
- **Implementation Plan**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/plan.md`
- **Project README**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/README.md`

---

## Next Steps

1. ✅ All tickets created and indexed
2. ⬜ Begin execution with: `/single-ticket BLOBSHA-1001`
3. ⬜ Or execute entire project: `/work-on-project BLOBSHA`

**Estimated Timeline**: 6-7 working days for complete implementation

**Blocks**: BRANCHX (Branch-Aware Indexing), BRWATCH (Branch Switch Detection)
