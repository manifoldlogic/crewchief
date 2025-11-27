# IDXABS Tickets Review Report

**Project**: IDXABS - Indexer SQLite-Only Migration
**Review Date**: 2025-11-27
**Tickets Reviewed**: 12
**Overall Assessment**: **NEEDS WORK** - Several gaps and inaccuracies require correction before execution

---

## Executive Summary

| Metric | Value |
|--------|-------|
| Total Tickets | 12 |
| Critical Issues | 3 |
| Warnings | 5 |
| Recommendations | 4 |
| Ready for Execution | No (after fixes) |

The tickets are generally well-structured with clear acceptance criteria, but codebase analysis reveals **significant gaps**:
1. Several modules with PostgreSQL references are not covered by any ticket
2. File inventories in tickets are incomplete/inaccurate
3. Some assumed SqliteStore methods may not exist

---

## Critical Issues

### CRITICAL-1: Missing Coverage for `upsert.rs` Module

**Tickets Affected**: None (gap in coverage)

**Problem**: The file `crates/maproom/src/upsert.rs` contains **7 PostgreSQL references** (`tokio_postgres::Client`) but is not mentioned in any ticket. This is a standalone module (not in indexer/) that handles cache-aware chunk upserting.

**Impact**: After all tickets complete, the crate will fail to compile due to unresolved PostgreSQL imports in upsert.rs.

**Required Action**:
- Add `upsert.rs` to IDXABS-2001 (Refactor Indexer Module) scope, OR
- Create a new ticket IDXABS-2006 specifically for upsert.rs refactoring

**Priority**: BLOCK EXECUTION

---

### CRITICAL-2: Missing Coverage for `incremental/` Module

**Tickets Affected**: None (gap in coverage)

**Problem**: The `crates/maproom/src/incremental/` directory contains **3 files with PostgreSQL references**:
- `edge_updater.rs` (4 references)
- `processor.rs` (1 reference)
- `tree_sha_update.rs` (3 references)

None of these are mentioned in any ticket.

**Impact**: After all tickets complete, compilation will fail due to `&Client` usage in incremental module.

**Required Action**:
- Create ticket IDXABS-2006: Refactor incremental/ Module
- Add to Phase 2 dependencies between 2004 and 2005

**Priority**: BLOCK EXECUTION

---

### CRITICAL-3: Missing Coverage for `migrate/` Module

**Tickets Affected**: None (gap in coverage)

**Problem**: The file `crates/maproom/src/migrate/markdown.rs` contains **2 PostgreSQL references** but is not covered by any ticket.

**Impact**: Compilation failure after migration.

**Required Action**:
- Add `migrate/markdown.rs` to IDXABS-2005 (Refactor db Support Files) scope, OR
- Create separate ticket

**Priority**: BLOCK EXECUTION

---

## Warnings

### WARNING-1: Incomplete File List in IDXABS-1001

**Ticket**: IDXABS-1001 (Delete PostgreSQL Database Files)

**Problem**: The ticket lists files to delete but the actual `db/` directory contains additional files:
- **Listed**: `postgres/`, `pool.rs`, `queries.rs`, `factory.rs`, `materialized_views.rs`
- **Also exists**: `connection.rs` (has `#[cfg(feature = "sqlite")]` conditionals)

The `connection.rs` file has fallback logic to PostgreSQL and will need updating (not deletion) in a subsequent ticket.

**Impact**: Incomplete understanding of what needs to be modified.

**Suggested Remediation**:
- Note in ticket that `connection.rs` will be simplified in ticket 1002 or a separate step
- Update ticket 1002 to explicitly handle `connection.rs` simplification (remove PostgreSQL fallback logic)

---

### WARNING-2: IDXABS-2003 File Count Mismatch

**Ticket**: IDXABS-2003 (Refactor Search Module)

**Problem**: Ticket lists 6 files in search/ but the directory contains **18 files**. Additional files that may have PostgreSQL references:
- `query_processor.rs`
- `results.rs`
- `dedup.rs`
- `cache.rs`
- `warming.rs`
- `expander.rs`
- `tokenizer.rs`
- `types.rs`
- `mod.rs` (has 2 PostgreSQL references per grep)
- `fusion/` subdirectory

**Impact**: May miss PostgreSQL references in unlisted files.

**Suggested Remediation**:
- Update ticket to say "Update all files in `search/` directory as needed"
- Add explicit verification step: `grep -r "tokio_postgres\|&Client" crates/maproom/src/search/`

---

### WARNING-3: IDXABS-2004 File Count Mismatch

**Ticket**: IDXABS-2004 (Refactor Context Module)

**Problem**: Ticket lists 4 main files but context/ contains **18+ files** including:
- `detectors/hooks.rs` (4 PostgreSQL references)
- `detectors/jsx.rs` (4 PostgreSQL references)
- `detectors/component.rs` (possible references)
- `strategies/` subdirectory

The detector files are **not mentioned** in the ticket but **have PostgreSQL references**.

**Impact**: Compilation will fail if detectors are not updated.

**Suggested Remediation**:
- Update acceptance criteria: "Detector and strategy files updated (if any PostgreSQL refs)" → explicit file list
- Add: `context/detectors/hooks.rs`, `context/detectors/jsx.rs`

---

### WARNING-4: db/columns.rs Not Addressed

**Ticket**: IDXABS-2005 (Refactor db Support Files)

**Problem**: The file `db/columns.rs` exists and is not mentioned. While it may not have PostgreSQL-specific code, it should be verified.

**Impact**: Low - likely no issues, but incomplete inventory.

**Suggested Remediation**: Add verification step to confirm `columns.rs` doesn't need updates.

---

### WARNING-5: Cargo.toml Feature Flags More Complex Than Described

**Ticket**: IDXABS-1003 (Update Cargo.toml)

**Problem**: Actual Cargo.toml has:
```toml
[features]
default = ["postgres"]
profiling = ["puffin"]
sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]
postgres = []
```

The ticket doesn't mention:
- Removing `default = ["postgres"]`
- Removing `postgres = []` feature
- The `r2d2` and `r2d2_sqlite` dependencies (used for SQLite connection pooling)

**Impact**: Incomplete Cargo.toml changes.

**Suggested Remediation**: Update ticket to explicitly list:
- Remove `default = ["postgres"]`
- Remove `postgres = []` feature line
- Keep `rusqlite`, `r2d2`, `r2d2_sqlite` as required (not optional)

---

## Recommendations

### REC-1: Split IDXABS-4001 (Fix and Update Tests)

**Current Scope**: 3-4 hours estimated for all test fixes

**Recommendation**: This ticket is potentially very large depending on test coverage. Consider splitting if needed during execution, or add explicit checkpoint:

> If more than 15 test files need modification, pause and create sub-tickets.

---

### REC-2: Add Checkpoint After Phase 1

**Recommendation**: After Phase 1 (tickets 1001-1003), run `cargo check` and document all remaining errors. This creates a baseline for Phase 2 work and catches any missed deletions.

Add to ticket 1003 acceptance criteria:
- [ ] Document list of compilation errors remaining for Phase 2 reference

---

### REC-3: Verify SqliteStore Method Existence

**Tickets Affected**: IDXABS-2002, IDXABS-2003, IDXABS-2004, IDXABS-2005

**Recommendation**: Several tickets assume SqliteStore has certain methods. Before starting Phase 2, verify method availability:

```bash
grep -E "pub (async )?fn" crates/maproom/src/db/sqlite/mod.rs | head -50
```

Document actual methods vs. assumed methods. If gaps exist, add method implementation to ticket scope.

---

### REC-4: Update Ticket Index After Fixes

**Recommendation**: After addressing critical issues, update `IDXABS_TICKET_INDEX.md` with:
- New ticket(s) for missing modules
- Updated dependency chain
- Revised time estimates

---

## Integration Assessment

### Overall Integration Health: FAIR

**Key Integration Points**:

| Integration Point | Status | Notes |
|------------------|--------|-------|
| db/mod.rs → all modules | Good | Well-documented in tickets |
| indexer → db | Good | Covered by 2001 |
| embedding → db | Good | Covered by 2002 |
| search → db | Partial | Missing some files |
| context → db | Partial | Missing detector files |
| incremental → db | **MISSING** | Not covered |
| upsert → db | **MISSING** | Not covered |
| migrate → db | **MISSING** | Not covered |
| main.rs → all | Good | Covered by 3001 |

### Risks to Existing Functionality

1. **File watching** (`incremental/`) - Critical for `watch` command. Missing coverage.
2. **Upsert caching** (`upsert.rs`) - Used for cache-aware indexing. Missing coverage.
3. **Migration** (`migrate/`) - Data migration utilities. Missing coverage.

### Mitigation Recommendations

1. Add missing module tickets (Critical issues 1-3)
2. Run `cargo check` checkpoint after each phase
3. Test `watch` command functionality explicitly in E2E

---

## Dependency Analysis

### Current Dependency Chain

```
1001 → 1002 → 1003
         ↓
2001 → 2002 → 2003 → 2004 → 2005
         ↓
      3001
         ↓
      4001 → 4002
         ↓
      5001
```

### Issues Found

1. **Missing nodes**: incremental, upsert, migrate modules have no tickets
2. **Potential parallelism**: 2001-2005 could partially run in parallel (no data dependencies)
3. **Bottleneck**: 3001 (main.rs) is single-threaded and depends on ALL Phase 2 tickets

### Recommended Updated Chain

```
1001 → 1002 → 1003
         ↓
2001 ──┬── 2002 ──┬── 2003 ──┬── 2004 ──┬── 2005
       │         │          │          │
       └── 2006 (incremental) ─────────┴── 2007 (upsert/migrate)
                    ↓
                  3001
                    ↓
                4001 → 4002
                    ↓
                  5001
```

---

## Ticket Actions Required

### Tickets to Rework

| Ticket | Required Changes |
|--------|------------------|
| IDXABS-1002 | Add `connection.rs` to files to modify (remove PG fallback logic) |
| IDXABS-1003 | Add: remove `default = ["postgres"]`, `postgres = []`; keep r2d2 deps |
| IDXABS-2003 | Expand file list or use "all files in search/" |
| IDXABS-2004 | Add `detectors/hooks.rs`, `detectors/jsx.rs` explicitly |
| IDXABS-2005 | Add `migrate/markdown.rs` to scope |

### Tickets to Create

| New Ticket | Scope | Phase |
|------------|-------|-------|
| IDXABS-2006 | Refactor incremental/ module (edge_updater, processor, tree_sha_update) | Phase 2 |
| IDXABS-2007 | Refactor upsert.rs (7 PostgreSQL refs) | Phase 2 |

### Tickets to Defer

None - all tickets are appropriately scoped for MVP.

### Tickets to Skip

None - all tickets are necessary.

### Tickets to Split

| Ticket | Reason | Suggested Split |
|--------|--------|-----------------|
| IDXABS-4001 | Potentially large | Monitor during execution; split if >15 files |

### Tickets to Merge

None - ticket granularity is appropriate.

---

## Recommendations for Execution

### Suggested Execution Order

1. **Pre-flight**: Address all Critical Issues (create missing tickets)
2. **Phase 1**: Execute 1001 → 1002 → 1003 in sequence
3. **Checkpoint**: Run `cargo check`, document errors
4. **Phase 2**: Execute 2001-2007 (new expanded set)
5. **Checkpoint**: `cargo check -p crewchief-maproom --lib` should pass
6. **Phase 3**: Execute 3001
7. **Checkpoint**: `cargo build --bin crewchief-maproom` should succeed
8. **Phase 4**: Execute 4001 → 4002
9. **Phase 5**: Execute 5001

### Risk Mitigation Strategies

1. **Git branches**: Work on `feature/idxabs-sqlite-only` branch
2. **Incremental commits**: Commit after each ticket
3. **Checkpoint verification**: Run cargo check between phases
4. **Rollback points**: Tag before each phase starts

### Key Checkpoints

| Checkpoint | When | Verification |
|------------|------|--------------|
| Phase 1 Complete | After 1003 | `cargo check` shows only module errors, no dependency errors |
| Phase 2 Complete | After 2007 | `cargo check -p crewchief-maproom --lib` passes |
| Phase 3 Complete | After 3001 | Binary builds successfully |
| Phase 4 Complete | After 4002 | All tests pass, E2E script succeeds |
| Project Complete | After 5001 | Documentation accurate, no PG references |

### Success Criteria for Project Completion

1. `cargo build --bin crewchief-maproom` succeeds without `--features` flags
2. `cargo test -p crewchief-maproom` passes
3. E2E script completes successfully
4. `grep -r "tokio_postgres\|pgvector\|deadpool-postgres" crates/maproom/src/` returns nothing
5. Database created at `~/.maproom/maproom.db` by default

---

## Summary of Required Actions Before Execution

### Must Do (Critical)

1. ❌ Create IDXABS-2006 for `incremental/` module
2. ❌ Create IDXABS-2007 for `upsert.rs` module
3. ❌ Add `migrate/markdown.rs` to IDXABS-2005 scope

### Should Do (Warnings)

4. ⚠️ Update IDXABS-1002 to include `connection.rs`
5. ⚠️ Update IDXABS-1003 with complete feature flag changes
6. ⚠️ Update IDXABS-2003 with complete search/ file list
7. ⚠️ Update IDXABS-2004 with detector files
8. ⚠️ Update IDXABS_TICKET_INDEX.md with new tickets

### Consider (Recommendations)

9. 💡 Add checkpoint steps to ticket acceptance criteria
10. 💡 Verify SqliteStore method inventory before Phase 2

---

**Review Status**: COMPLETE
**Next Step**: Address Critical Issues 1-3, then re-run `/review-tickets IDXABS`
