# Plan: edge extraction

## Overview

Three-phase implementation: Phase 1 delivers calls edges for TypeScript/JavaScript (MVP to unblock SRCHREL), Phase 2 adds cross-file resolution and test edges, Phase 3 extends to Python/Rust. Total timeline: 2-3 weeks.

## Phases

### Phase 1: TypeScript/JavaScript Calls (MVP)

**Objective:** Extract same-file `calls` edges for TypeScript/JavaScript and populate `chunk_edges` table.

**Deliverables:**
- Edge extractor module with shared types (EDGEEXT-1001)
- TypeScript call extraction (EDGEEXT-1002)
- Integration with `scan_worktree()` and `upsert_files()` (EDGEEXT-1003)
- EdgeUpdater enhancement (recompute edges on file change) (EDGEEXT-1003)
- Unit tests for call extraction (EDGEEXT-1002)
- Integration test with synthetic TypeScript repo (EDGEEXT-1004)

**Agent Assignments:**
- **rust-expert**: Create edge extractor module, TypeScript traversal logic, symbol resolution
- **database-engineer**: Review database integration, ensure batch insertion works
- **test-engineer**: Unit tests for edge extraction, integration tests with test repo

**Timeline:** 1 week

**Success Criteria:**
- [ ] `chunk_edges` table populated during scan
- [ ] ≥10,000 edges created for test repository (140K chunks)
- [ ] Same-file calls: ≥85% accuracy
- [ ] Performance overhead <30%

### Phase 2: Cross-File + Test Edges

**Objective:** Add cross-file resolution for calls, implement `test_of` edges using file path heuristics.

**Deliverables:**
- Cross-file symbol resolution (using `find_chunk_by_symbol()`)
- Test file detection (`*.test.ts`, `*.spec.ts`, `__tests__/`)
- `test_of` edge extraction (test functions → target symbols)
- Performance optimization (batch database queries)
- Integration tests for cross-file calls and test edges

**Agent Assignments:**
- **rust-expert**: Cross-file resolution, test detection heuristics
- **database-engineer**: Optimize `find_chunk_by_symbol()` for batch queries
- **test-engineer**: Integration tests, accuracy validation

**Timeline:** 1 week

**Success Criteria:**
- [ ] Cross-file calls resolved (≥60% accuracy)
- [ ] `test_of` edges created for test files
- [ ] Performance still within budget (<30% overhead)

### Phase 3: Python + Rust Support

**Objective:** Extend edge extraction to Python (calls) and Rust (calls).

**Deliverables:**
- Python call extraction (`edges/python.rs`)
- Rust call extraction (`edges/rust.rs`)
- Language-specific tests
- Documentation for adding new languages

**Agent Assignments:**
- **rust-expert**: Python/Rust extractors
- **test-engineer**: Language-specific tests

**Timeline:** 1 week (can overlap with SRCHREL work)

**Success Criteria:**
- [ ] Python calls extracted (parity with TypeScript)
- [ ] Rust calls extracted
- [ ] Pattern documented for future languages

## Dependencies

**Phase 1 → Phase 2:** Core extraction must work before adding cross-file resolution.

**Phase 2 → Phase 3:** No dependency (can start Phase 3 while Phase 2 stabilizes).

**External Dependencies:**
- Database schema (complete)
- `SqliteStore::insert_chunk_edge()` (exists)
- Tree-sitter parsers (installed)

**Blocker for:**
- SRCHREL_relationship-aware-search (Phase 1 unblocks)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Symbol resolution accuracy too low (<70%) | Medium | High | Start with same-file only (85% accuracy), defer cross-file to Phase 2 |
| Performance overhead >30% | Low | Medium | Batch operations, skip edge extraction if no calls found, profile early |
| Tree-sitter parse failures | Low | Low | Log warning and continue (partial edges better than none) |
| EdgeUpdater integration breaks incremental updates | Low | High | Test with file modification scenarios, reuse existing deletion logic |
| Cross-file resolution creates spurious edges | Medium | Medium | Log unresolved calls at trace level, add confidence scores in future |

## Success Metrics

**Phase 1 (MVP):**
- [ ] `chunk_edges` table has ≥10,000 rows after scan
- [ ] Same-file calls: ≥85% precision, ≥60% recall
- [ ] Scan time increase <30% (measured on test repo)
- [ ] Incremental updates work (edges recomputed on file change)
- [ ] SRCHREL unblocked (can implement quality scoring)

**Phase 2 (Enhanced):**
- [ ] Cross-file calls resolved (≥60% accuracy)
- [ ] Test edges created (`test_of` type)
- [ ] Performance optimized (batched database queries)

**Phase 3 (Extended):**
- [ ] Python and Rust calls extracted
- [ ] Documentation complete for adding new languages

## Rollout Plan

1. **Phase 1 Deployment:**
   - Merge edge extractor module
   - Deploy to staging (rescan test repository)
   - Validate edge count and accuracy
   - Deploy to production (trigger full rescan)

2. **Phase 2 Deployment:**
   - Enable cross-file resolution (feature flag if needed)
   - Monitor performance impact
   - Roll out test edge extraction

3. **Phase 3 Deployment:**
   - Enable Python/Rust extractors
   - Document patterns for future languages

## Contingency Plans

**If accuracy is too low:**
- Reduce scope to same-file only (still valuable)
- Add logging to identify failure patterns
- Iterate on resolution heuristics

**If performance is too slow:**
- Skip cross-file resolution in MVP
- Optimize symbol table lookup (use btree_map)
- Parallelize edge extraction per file

**If SRCHREL needs edges sooner:**
- Ship Phase 1 immediately (TypeScript calls only)
- Defer Python/Rust to later iteration
