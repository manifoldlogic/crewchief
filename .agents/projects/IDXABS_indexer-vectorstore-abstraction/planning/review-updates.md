# Project Review Updates

**Original Review Date:** 2025-11-27
**Major Revision:** 2025-11-27
**Post-Review Updates:** 2025-11-27
**Update Status:** Complete

## Major Scope Change: SQLite-Only

**Previous Approach**: Refactor indexer to use VectorStore trait abstraction, supporting both PostgreSQL and SQLite backends.

**New Approach**: **Remove PostgreSQL entirely**, make SQLite the only backend.

### Rationale

User decision to keep implementation as lean as possible. Supporting two backends at this stage:
- Adds maintenance burden
- Creates complexity (traits, factories, feature flags)
- Isn't needed for zero-config operation

### Impact

| Aspect | Before | After |
|--------|--------|-------|
| Approach | Abstract to trait | Delete PostgreSQL |
| Complexity | Maintain two backends | Single backend |
| Tickets | ~15 tickets | ~12 tickets |
| Estimated time | 27-37 hours | 18-26 hours |
| Code changes | Refactor | Delete + Simplify |

### Files Changed

All planning documents rewritten:
- `analysis.md` - Now describes deletion, not abstraction
- `architecture.md` - Now describes SQLite-only architecture
- `plan.md` - Now has deletion-focused tickets
- `README.md` - Updated to reflect new scope

### Previous Work Superseded

The following previous updates are superseded by the SQLite-only decision:
- Embedding pipeline abstraction to VectorStore trait
- generate-embeddings command abstraction
- VectorStore trait usage expansion

These are no longer relevant because we're deleting PostgreSQL, not abstracting to support both.

---

## Post-Review Updates (2025-11-27)

Following `/review-project IDXABS`, these updates were made to address identified gaps:

### Critical Issues Addressed

#### Issue 1: Missing SqliteStore Methods for Embedding Pipeline
**Original Problem:** `embedding/pipeline.rs` uses raw PostgreSQL queries for embedding operations that don't have SqliteStore equivalents.

**Changes Made:**
- `plan.md`: Updated ticket 2002 to explicitly include implementing 3 missing methods:
  - `get_chunks_needing_embeddings_count()`
  - `copy_existing_embeddings_from_cache()`
  - `fetch_chunks_needing_embeddings(incremental, sample_size)`
- `plan.md`: Added acceptance criterion "SqliteStore has all methods required by EmbeddingPipeline"

**Result:** Issue resolved - ticket 2002 now includes method implementation scope.

#### Issue 2: Outdated Quality Strategy Document
**Original Problem:** `quality-strategy.md` referenced deprecated dual-backend approach.

**Changes Made:**
- `quality-strategy.md`: Added deprecation note at top explaining document will be updated in Phase 4

**Result:** Issue resolved - users are warned content is legacy.

#### Issue 3: Outdated Security Review Document
**Original Problem:** `security-review.md` referenced PostgreSQL (now being removed).

**Changes Made:**
- `security-review.md`: Added deprecation note explaining PostgreSQL references are legacy

**Result:** Issue resolved - core security analysis remains valid, PostgreSQL references marked as legacy.

### High-Risk Mitigations Implemented

#### Risk: Embedding Pipeline Gaps
**Mitigation Applied:**
- `analysis.md`: Added "Embedding pipeline gaps" to risk table with Medium/Medium rating
- `analysis.md`: Added explicit note documenting the 3 required methods
- `analysis.md`: Added Rollback Strategy section

**Risk Level:** Reduced from High (undocumented) to Medium (documented with mitigation)

### Gaps Filled

#### VectorStore Trait Deletion Not Explicit
- `architecture.md`: Expanded "Decision 1: Remove VectorStore Trait" to explicitly list what gets deleted:
  - `pub trait VectorStore: Send + Sync { ... }` (~150 lines)
  - `BackendType` enum
  - All `#[async_trait]` impl blocks
  - Factory pattern and `get_store()` function

#### Missing Rollback Strategy
- `analysis.md`: Added "Rollback Strategy" section with 3 approaches:
  1. Git revert (per-ticket commits)
  2. Feature branch until stable
  3. Incremental testing after each phase

## Document Change Summary

### analysis.md
- Lines modified: ~15
- Key changes: Added embedding pipeline risk, rollback strategy

### architecture.md
- Lines modified: ~10
- Key changes: Explicit VectorStore deletion details

### plan.md
- Lines modified: ~10
- Key changes: Ticket 2002 expanded with missing methods

### quality-strategy.md
- Lines modified: ~4
- Key changes: Added deprecation note header

### security-review.md
- Lines modified: ~4
- Key changes: Added deprecation note header

## Success Criteria

```bash
# These work WITHOUT --features sqlite:
cargo run --bin crewchief-maproom -- scan --path /repo
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs
cargo run --bin crewchief-maproom -- watch
cargo run --bin crewchief-maproom -- generate-embeddings
cargo run --bin crewchief-maproom -- search "function"

# All tests pass:
cargo test
```

## Verification

**Next Steps:**
1. Run `/review-project IDXABS` to verify all issues resolved ✅
2. Proceed to `/create-project-tickets IDXABS`
3. Execute tickets in order
4. Verify all commands work with SQLite

**Success Metrics:**
- [x] All critical issues resolved (3/3)
- [x] High-risk areas mitigated (1/1)
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for ticket creation

---

## Ticket Review Updates (2025-11-27)

Following `/review-tickets IDXABS`, additional gaps were identified and resolved:

### New Critical Issues Resolved

#### CRITICAL-1: Missing Coverage for `upsert.rs` Module
- **Status**: ✅ RESOLVED
- **Action**: Created ticket IDXABS-2007
- **Details**: `upsert.rs` has 7 PostgreSQL references for cache-aware chunk upserting

#### CRITICAL-2: Missing Coverage for `incremental/` Module
- **Status**: ✅ RESOLVED
- **Action**: Created ticket IDXABS-2006
- **Details**: 3 files with 8 total PostgreSQL references:
  - `edge_updater.rs` (4 refs)
  - `processor.rs` (1 ref)
  - `tree_sha_update.rs` (3 refs)

#### CRITICAL-3: Missing Coverage for `migrate/` Module
- **Status**: ✅ RESOLVED
- **Action**: Added to IDXABS-2005 scope
- **Details**: `migrate/markdown.rs` has 2 PostgreSQL references

### Ticket Updates Applied

| Ticket | Updates |
|--------|---------|
| IDXABS-1002 | Added connection.rs to scope (remove PG fallback logic) |
| IDXABS-1003 | Added complete feature flag removal (`default = ["postgres"]`, `postgres = []`, r2d2 deps) |
| IDXABS-2003 | Expanded to include mod.rs and all 18 search/ files |
| IDXABS-2004 | Added detector files (hooks.rs, jsx.rs - 8 refs) |
| IDXABS-2005 | Added migrate/markdown.rs to scope |

### New Tickets Created

| Ticket | Purpose |
|--------|---------|
| IDXABS-2006 | Refactor incremental/ module (8 PostgreSQL refs) |
| IDXABS-2007 | Refactor upsert.rs (7 PostgreSQL refs) |

### Updated Metrics

| Metric | Before Ticket Review | After Ticket Review |
|--------|---------------------|---------------------|
| Total Tickets | 12 | 14 |
| Phase 2 Tickets | 5 | 7 |
| Estimated Duration | 18-26 hours | 20-29 hours |
| PostgreSQL refs covered | ~85% | 100% |

### Files Changed

**Tickets Updated:**
- `tickets/IDXABS-1002_simplify-db-mod.md`
- `tickets/IDXABS-1003_update-cargo-toml.md`
- `tickets/IDXABS-2003_refactor-search-module.md`
- `tickets/IDXABS-2004_refactor-context-module.md`
- `tickets/IDXABS-2005_refactor-db-support-files.md`

**Tickets Created:**
- `tickets/IDXABS-2006_refactor-incremental-module.md`
- `tickets/IDXABS-2007_refactor-upsert-module.md`

**Index Updated:**
- `tickets/IDXABS_TICKET_INDEX.md`

**Plan Updated:**
- `planning/plan.md` - All scope changes, new tickets, dependencies, timeline

### Verification

See `tickets-review-report.md` for the complete review analysis.

**Ready for Execution**: ✅ YES

Execute with `/work-on-project IDXABS` or `/single-ticket IDXABS-1001`
