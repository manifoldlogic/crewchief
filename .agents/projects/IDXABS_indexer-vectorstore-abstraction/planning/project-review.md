# Project Review: IDXABS - SQLite-Only Migration

**Review Date:** 2025-11-27
**Reviewer:** Claude Code
**Project Status:** Ready for Ticket Creation (with noted gaps)

## Executive Summary

**Recommendation: PROCEED WITH MODIFICATIONS**

The project planning documents have been revised to reflect the SQLite-only approach. The scope is now clear: **delete PostgreSQL entirely** and simplify to a single backend. This is a significant improvement over the previous dual-backend abstraction approach.

However, several documentation and planning gaps need attention before creating tickets.

## Review Categories

### 1. Scope & Requirements ✅ PASS

**Strengths:**
- Clear problem statement: Remove PostgreSQL complexity (183 references across 33 files)
- Explicit scope boundary: SQLite-only, no new features
- Concrete success criteria with testable commands
- Well-defined benefits: zero-config, simpler code, faster compilation

**What to Delete (well-documented):**
- `db/postgres/` - PostgreSQL implementation
- `db/pool.rs` - PostgreSQL connection pooling
- `db/queries.rs` - 28 PostgreSQL-specific queries
- `db/factory.rs` - Backend switching logic
- `db/materialized_views.rs` - PostgreSQL materialized views

### 2. Architecture ✅ PASS

**Strengths:**
- Clear before/after diagrams showing complexity reduction
- Concrete code examples for refactoring patterns
- Module-by-module refactoring guidance
- Cargo.toml changes clearly specified

**Key Design Decision (good):**
```rust
// Before: Complex trait + factory
pub trait VectorStore: Send + Sync { ... }
pub fn get_store() -> Arc<dyn VectorStore> { ... }

// After: Direct SQLite
pub use sqlite::SqliteStore;
pub async fn connect() -> anyhow::Result<SqliteStore>
```

### 3. Technical Feasibility ⚠️ GAPS FOUND

**Issue 1: Missing SqliteStore Methods for Embedding Pipeline**

The `embedding/pipeline.rs` file uses raw PostgreSQL queries that don't have SqliteStore equivalents:

| Required Operation | PostgreSQL Code | SqliteStore Method |
|-------------------|-----------------|-------------------|
| Count chunks needing embeddings | Raw SQL COUNT query | ❌ NOT FOUND |
| Copy existing embeddings | `UPDATE ... FROM code_embeddings` | ❌ NOT FOUND |
| Fetch chunks needing embeddings | Raw SQL SELECT with NULL check | ❌ NOT FOUND |
| Update chunk embeddings | `UPDATE maproom.chunks SET code_embedding = ...` | ✅ `upsert_embeddings()` exists |
| Populate embedding cache | `INSERT INTO maproom.code_embeddings` | ✅ `upsert_embedding()` exists |

**Missing Methods to Add to SqliteStore:**
1. `get_chunks_needing_embeddings_count()` - Returns count of chunks with NULL embeddings
2. `copy_existing_embeddings_from_cache()` - Bulk copy from code_embeddings to chunks
3. `fetch_chunks_needing_embeddings(incremental: bool, sample_size: Option<usize>)` - Get chunk data for embedding generation

**Issue 2: Architecture.md References Non-Existent Method**

Line 169 references `store.get_chunks_needing_embeddings_count().await?` but grep confirms this method doesn't exist in SqliteStore.

**Recommendation:** Add a ticket (2002A or modify 2002) to implement these missing SqliteStore methods BEFORE refactoring the embedding pipeline.

### 4. Quality Strategy ⚠️ OUTDATED

**Issue:** `quality-strategy.md` still references the OLD dual-backend approach:

- References `BackendType` enum (to be deleted)
- References `--features sqlite` (to be removed)
- Test matrix includes PostgreSQL column
- Manual verification includes PostgreSQL test
- Example tests use `#[cfg(feature = "sqlite")]`

**Recommendation:** Update `quality-strategy.md` for SQLite-only scope OR note it will be updated during Phase 4 (Testing).

### 5. Security Review ⚠️ OUTDATED

**Issue:** `security-review.md` still discusses:
- "PostgreSQL or SQLite" (line 13)
- "Neither PostgreSQL nor SQLite backends encrypt" (line 115)
- PostgreSQL comparison table

**Impact:** Low - core security analysis (parameterized queries, file permissions) remains valid.

**Recommendation:** Update or add note that PostgreSQL references are legacy.

### 6. Plan & Tickets ✅ PASS

**Strengths:**
- Clear phase structure (5 phases, 12 tickets)
- Logical dependency chain: Delete → Refactor → Cleanup → Test → Document
- Time estimates per phase (18-26 hours total)
- Single agent assignment (rust-indexer-engineer)

**Ticket Quality:**
| Ticket | Summary | Scope | Acceptance Criteria |
|--------|---------|-------|---------------------|
| 1001 | Delete PostgreSQL files | ✅ Clear | ✅ Checkable |
| 1002 | Simplify db/mod.rs | ✅ Clear | ✅ Checkable |
| 1003 | Update Cargo.toml | ✅ Clear | ✅ Checkable |
| 2001-2005 | Refactor modules | ✅ Clear | ✅ Checkable |
| 3001 | Clean main.rs | ✅ Clear | ✅ Checkable |
| 4001-4002 | Testing | ✅ Clear | ✅ Checkable |
| 5001 | Documentation | ✅ Clear | ✅ Checkable |

### 7. Risk Assessment ✅ PASS

**Identified Risks (analysis.md):**
- Breaking PostgreSQL users: N/A (intentional removal)
- Missing SQLite features: Low (SqliteStore already implements VectorStore)
- Test failures: Medium, with mitigation (fix as encountered)
- Performance regression: Low (acceptable for zero-config)

**Unidentified Risk (NEW):**
- Embedding pipeline methods missing from SqliteStore (MEDIUM impact)

## Critical Findings

### Must Fix Before Ticket Creation

1. **Document Missing SqliteStore Methods**

   Add to `architecture.md` or `plan.md` ticket 2002:
   ```
   SqliteStore methods to implement:
   - get_chunks_needing_embeddings_count() -> i64
   - copy_existing_embeddings_from_cache() -> usize
   - fetch_chunks_needing_embeddings(incremental, sample_size) -> Vec<ChunkRow>
   ```

2. **Update or Mark Outdated Documents**

   Either update these files or add a note at the top:
   - `quality-strategy.md` - References dual-backend approach
   - `security-review.md` - References PostgreSQL

### Should Fix (Non-Blocking)

1. Add explicit note that `VectorStore` trait will be removed (not just unused)
2. Consider adding rollback strategy (git revert if critical issues found)

## Validation Checklist

| Category | Status | Notes |
|----------|--------|-------|
| Problem clearly defined | ✅ | Remove PostgreSQL, SQLite-only |
| Solution architecture documented | ✅ | Before/after diagrams, code examples |
| Scope boundaries clear | ✅ | Explicit in/out of scope |
| Technical feasibility validated | ⚠️ | Missing SqliteStore methods |
| Risks identified | ⚠️ | One risk missed |
| Testing strategy defined | ⚠️ | Outdated for SQLite-only |
| Dependencies documented | ✅ | None (simplification project) |
| Ticket breakdown complete | ✅ | 12 tickets, 5 phases |

## Recommendation

**Proceed to ticket creation** with the following modifications:

1. **Add missing methods to ticket 2002** (Refactor Embedding Pipeline):
   - Expand scope to include implementing 3 missing SqliteStore methods
   - Add acceptance criteria: "SqliteStore has methods for embedding pipeline"

2. **Add note to quality-strategy.md header**:
   ```markdown
   > **Note:** This document will be updated during Phase 4. Current content
   > references the deprecated dual-backend approach.
   ```

3. **Add note to security-review.md header**:
   ```markdown
   > **Note:** PostgreSQL references are legacy. Project is now SQLite-only.
   ```

## Next Steps

1. Update ticket 2002 in `plan.md` to include missing SqliteStore methods
2. Add header notes to outdated documents
3. Run `/create-project-tickets IDXABS` to generate ticket files
4. Begin execution with Phase 1 (Delete PostgreSQL Code)
