# EMBCOPY Ticket Index

**Project:** Embedding Inheritance Fix
**Status:** Ready for implementation
**Total Tickets:** 5

## Overview

This project fixes the critical performance issue where variant worktree scans take hours because embeddings are regenerated instead of copied from the deduplication cache. The fix adds a pre-generation copy step to reuse existing embeddings, reducing scan time from hours to seconds (200-500× improvement).

## Ticket Organization

### Phase 1: Implementation (1xxx)

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| EMBCOPY-1001 | Implement Embedding Copy Step in Pipeline | rust-indexer-engineer | ⬜ Pending | None |
| EMBCOPY-1002 | Add Unit Tests for Embedding Copy Function | rust-indexer-engineer | ⬜ Pending | EMBCOPY-1001 |
| EMBCOPY-1003 | Add Integration Test for Variant Worktree Embedding Copy | rust-indexer-engineer | ⬜ Pending | EMBCOPY-1001, EMBCOPY-1002 |

### Phase 1: Validation (19xx)

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| EMBCOPY-1901 | Validate Fix with Genetic Optimizer End-to-End Test | verify-ticket | ⬜ Pending | EMBCOPY-1001, EMBCOPY-1002, EMBCOPY-1003 |
| EMBCOPY-1902 | Commit Embedding Inheritance Fix | commit-ticket | ⬜ Pending | EMBCOPY-1001, EMBCOPY-1002, EMBCOPY-1003, EMBCOPY-1901 |

## Execution Sequence

```
EMBCOPY-1001 (Implementation)
    ↓
EMBCOPY-1002 (Unit Tests)
    ↓
EMBCOPY-1003 (Integration Test)
    ↓
EMBCOPY-1901 (Validation)
    ↓
EMBCOPY-1902 (Commit)
```

## Ticket Details

### EMBCOPY-1001: Implement Embedding Copy Step in Pipeline
**Agent:** rust-indexer-engineer
**Files:** `crates/maproom/src/embedding/pipeline.rs`
**Summary:** Add `copy_existing_embeddings()` method that copies embeddings from `code_embeddings` table to chunks with NULL embeddings before attempting API generation.

**Key Deliverables:**
- New async method with SQL UPDATE/JOIN query
- Extended `PipelineStats` with `copied_from_cache` and `cost_saved_usd`
- Integration into `run()` method
- Compiles without errors

---

### EMBCOPY-1002: Add Unit Tests for Embedding Copy Function
**Agent:** rust-indexer-engineer
**Files:** `crates/maproom/src/embedding/pipeline.rs` (test module)
**Summary:** Write 3 comprehensive unit tests verifying copy success, skip behavior, and idempotent operation.

**Key Deliverables:**
- `test_copy_existing_embeddings_success` - verifies copy works
- `test_copy_skips_without_cache` - verifies graceful skip
- `test_copy_idempotent` - verifies safe re-execution
- All tests passing: `cargo test copy_existing`

---

### EMBCOPY-1003: Add Integration Test for Variant Worktree Embedding Copy
**Agent:** rust-indexer-engineer
**Files:** `crates/maproom/tests/embedding_inheritance_test.rs` (new)
**Summary:** Create end-to-end test simulating genetic optimizer scenario: base scan + variant scan with performance validation.

**Key Deliverables:**
- Test completes in < 10 seconds (not hours)
- Stats show >99% copy ratio
- Demonstrates 200-500× speedup
- Test passes: `cargo test embedding_inheritance`

---

### EMBCOPY-1901: Validate Fix with Genetic Optimizer End-to-End Test
**Agent:** verify-ticket
**Files:** None (manual validation)
**Summary:** Run actual genetic optimizer to verify variant scans now complete quickly in real-world scenario.

**Key Deliverables:**
- Genetic optimizer runs successfully
- Each variant scan < 10 seconds
- Embedding stats show cache reuse
- No regression in base branch performance
- Competition framework now practical

---

### EMBCOPY-1902: Commit Embedding Inheritance Fix
**Agent:** commit-ticket
**Files:** Git commit
**Summary:** Create conventional commit with performance metrics after all implementation and validation complete.

**Key Deliverables:**
- All tests passing
- Properly formatted conventional commit
- Performance impact documented
- Project archived

## Success Metrics

- ✅ Variant worktree scans: hours → < 10 seconds (200-500× faster)
- ✅ Embedding copy count > 99% for variant scans
- ✅ API cost reduction: ~400× for typical branch switches
- ✅ Genetic optimizer: minutes not hours
- ✅ No regression in base branch performance

## Planning References

- **Analysis:** `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/analysis.md`
- **Architecture:** `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/architecture.md`
- **Quality Strategy:** `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/quality-strategy.md`
- **Security Review:** `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/security-review.md`
- **Plan:** `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/plan.md`

## Notes

This project implements the missing step from BLOBSHA deduplication infrastructure. The cache table exists, but the embedding pipeline wasn't using it. Simple fix, huge impact - eliminates duplicate embedding generation for 88% of chunks.
