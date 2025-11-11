# SCHMAFIX Ticket Review Report

**Project:** SCHMAFIX_schema-migration-integration
**Review Date:** 2025-11-09
**Reviewer:** Claude Code (Automated Review)
**Total Tickets Reviewed:** 7

---

## Executive Summary

**Overall Assessment:** ⚠️ **CRITICAL ISSUES FOUND - EXECUTION BLOCKED**

**Status:** Project tickets require significant revision before execution can proceed safely.

**Critical Issues:** 3 (must fix before execution)
**Warnings:** 4 (should address)
**Recommendations:** 2 (consider improvements)

### Key Findings

1. **CRITICAL:** Migration 005 contains `TRUNCATE TABLE` that will **delete all indexed code** in production
2. **CRITICAL:** Migration 0017 exists in filesystem but is missing from Rust migration runner
3. **CRITICAL:** Migration 001 contains PostgreSQL-specific syntax incompatible with transaction-safe execution
4. Several migrations have complex multi-step backfills that may fail partially

**Recommendation:** **BLOCK EXECUTION** until critical issues are resolved.

---

## Critical Issues (Block Execution)

### CRITICAL-1: Migration 005 Will Delete All Production Data

**Affected Tickets:** SCHMAFIX-1001, SCHMAFIX-2001, SCHMAFIX-3001, SCHMAFIX-5001

**Problem:**
Migration file `packages/maproom-mcp/migrations/005_complete_branchx_schema.sql` (line 31) contains:

```sql
TRUNCATE TABLE maproom.chunks CASCADE;
```

This will **permanently delete all indexed code chunks** from the database. The migration comment states:

> "No data preservation - clean schema migration for development environment."

**Impact:**
- ❌ **ALL indexed code will be deleted** when migration 0021 runs
- ❌ Users will lose all search history and embeddings
- ❌ Production databases will be wiped clean
- ❌ Violates SCHMAFIX security review requirement: "Data preservation (no data loss)"

**Evidence from Planning Documents:**
- `security-review.md` line 25: "Risk 2: Data Loss During Backfill" rated as HIGH RISK
- `quality-strategy.md` line 203: Manual checklist item "Chunk count preserved (no data loss)"
- `analysis.md` line 14: States `worktree_ids` column **already exists** in production

**Root Cause:**
Migration 005 was created for BRANCHX project assuming a clean development environment. It was never intended for production use where data exists.

**Required Action:**
1. **REMOVE migration 005 entirely from SCHMAFIX scope**
2. Migration 004 already creates all necessary schema (worktree_ids, worktree_index_state)
3. Update SCHMAFIX-1001 acceptance criteria to only copy migrations 001, 002, 004 (NOT 005)
4. Update SCHMAFIX-2001 to add only migrations 0018, 0019, 0020 (skip 0021)
5. Update all test tickets to expect 3 new migrations (not 4)
6. Update documentation tickets to note migration 0021 was skipped

**Priority:** 🚨 **BLOCK EXECUTION - Must fix immediately**

---

### CRITICAL-2: Migration 0017 Missing from Rust Migration Runner

**Affected Tickets:** SCHMAFIX-2001, SCHMAFIX-3001, SCHMAFIX-3901

**Problem:**
- Migration file `crates/maproom/migrations/0017_fix_index_size_limits.sql` exists in filesystem
- Migration runner `crates/maproom/src/db/queries.rs` (line 140) ends at migration 16
- Migration 0017 is **NOT** in the migrations array

**Impact:**
- ❌ Migration numbering conflict: SCHMAFIX wants to use 0018, but 0017 isn't applied yet
- ❌ Test failures: Integration tests will fail if they expect sequential migrations
- ❌ Schema inconsistency: Production databases missing index fixes from 0017
- ❌ Confusion: Why does 0017.sql exist if it's not used?

**Evidence:**
- `crates/maproom/src/db/queries.rs` line 134-140: Last entry is version 16
- `crates/maproom/migrations/0017_fix_index_size_limits.sql`: File exists (47 lines, CONCURRENT indexes)

**Required Action:**
1. **Add migration 0017 to queries.rs FIRST** (before SCHMAFIX work begins)
2. Set `use_concurrent_handler = true` (migration has CREATE INDEX CONCURRENTLY)
3. Create pre-SCHMAFIX ticket: "SCHMAFIX-0001: Add Missing Migration 0017 to Runner"
4. Update all SCHMAFIX tickets to reference migrations 0018-0020 (or 0018-0021 if 005 is included)
5. Run migration 0017 in development before starting SCHMAFIX execution

**Priority:** 🚨 **BLOCK EXECUTION - Must fix before SCHMAFIX-1001**

---

### CRITICAL-3: Migration 001 Uses Non-Transactional Features

**Affected Tickets:** SCHMAFIX-1001, SCHMAFIX-2001, SCHMAFIX-3001

**Problem:**
Migration 001 (blob_sha backfill) contains features incompatible with transactional execution:

**Line 44:** `CREATE INDEX CONCURRENTLY` - Cannot run inside transaction
**Lines 54-100:** Batched `DO` block with multiple `COMMIT` statements - Cannot run inside transaction
**Line 96:** Explicit `COMMIT` inside DO block - Will cause error in transaction

**Evidence from Rust Migration Runner:**
`crates/maproom/src/db/queries.rs` line 142-165 shows migrations run in transactions UNLESS `use_concurrent_handler = true`.

SCHMAFIX-2001 sets `concurrent = false` for migration 0018, which means:
```rust
let tx = client.transaction().await?;  // Start transaction
tx.execute(sql, &[]).await?;            // Execute migration 001 SQL
tx.commit().await?;                     // ERROR: SQL contains COMMIT statements
```

**Impact:**
- ❌ Migration 0018 will fail with: "COMMIT cannot run inside a transaction block"
- ❌ Test ticket SCHMAFIX-3901 will fail (migration execution errors)
- ❌ Manual validation SCHMAFIX-5001 will fail
- ❌ Project cannot proceed past Phase 2

**Required Action (Choose One):**

**Option A: Modify Migration 001 SQL** (Recommended)
1. Remove `CONCURRENTLY` from CREATE INDEX (line 44)
2. Remove batched backfill (lines 54-100), replace with single UPDATE
3. Simplified backfill:
   ```sql
   UPDATE maproom.chunks
   SET blob_sha = maproom.compute_git_blob_sha(preview)
   WHERE blob_sha IS NULL;
   ```
4. Accept longer lock time (migration will block writes during backfill)
5. Keep migration transaction-safe

**Option B: Set concurrent = true in SCHMAFIX-2001**
1. Update SCHMAFIX-2001: Change migration 0018 to `concurrent = true`
2. Accept that migration 0018 runs statement-by-statement (no rollback safety)
3. Risk: Partial failure leaves database in inconsistent state
4. Conflicts with security-review.md line 45: "Wrap backfill in transaction (ROLLBACK on failure)"

**Recommendation:** Use Option A - Simplify migration 001 to be transaction-safe.

**Priority:** 🚨 **BLOCK EXECUTION - Must fix SCHMAFIX-1001**

---

## Warnings (Should Address)

### WARNING-1: Migration 001 Backfill Performance Assumptions Unvalidated

**Affected Tickets:** SCHMAFIX-5001

**Concern:**
Migration 001 backfill (line 72) uses `preview` column to compute blob_sha:
```sql
SET blob_sha = maproom.compute_git_blob_sha(preview)
```

**Issues:**
1. `preview` column may not contain full content (it's a preview, likely truncated)
2. Blob SHA computed from preview ≠ blob SHA from full content
3. Migration 002 creates code_embeddings keyed by blob_sha
4. Mismatch will cause embedding lookups to fail

**Expected vs. Actual:**
- **Expected:** blob_sha = SHA256("blob " + length(full_content) + "\0" + full_content)
- **Actual:** blob_sha = SHA256("blob " + length(preview) + "\0" + preview)

**Evidence:**
- Migration 001 line 72: Uses `preview` column
- Migration 001 comment line 9: "Must produce identical output to Rust implementation"
- No `content` column exists in current schema (migration 005 adds it, but we're removing 005)

**Impact:**
- ⚠️ Blob SHA values will be incorrect
- ⚠️ Embedding deduplication (future BLOBSHA-IMPL) will fail
- ⚠️ Violates architectural intent of content-addressed storage

**Required Action:**
1. Update migration 001 to use `content` column (if it exists)
2. If `content` doesn't exist, document that blob_sha is computed from preview
3. Add warning comment: "blob_sha computed from preview (truncated), not full content"
4. Update BLOBSHA-IMPL project plan to re-compute blob_sha when implementing deduplication
5. Add validation query to migration 001 to warn if preview != content

**Alternative:**
- Skip migration 001 entirely (blob_sha not used yet)
- Defer to BLOBSHA-IMPL project to add blob_sha with correct implementation
- SCHMAFIX only adds code_embeddings table (migration 002) and worktree tracking (migration 004)

**Priority:** ⚠️ **Should fix before SCHMAFIX-5001 (manual validation)**

---

### WARNING-2: Migration Numbering Gap Between 0017 and 0018

**Affected Tickets:** SCHMAFIX-1001, SCHMAFIX-2001

**Concern:**
- Last Rust migration: 0016 (in queries.rs)
- Filesystem has: 0017 (not in queries.rs)
- SCHMAFIX wants: 0018-0021

This creates ambiguity: Should SCHMAFIX use 0017, 0018, or 0019 as first migration number?

**Impact:**
- ⚠️ Confusion for contributors: Which number to use next?
- ⚠️ Risk of migration number collision if someone adds 0017 to queries.rs independently
- ⚠️ Breaks sequential migration assumption in tests

**Required Action:**
1. Add migration 0017 to queries.rs FIRST (see CRITICAL-2)
2. Then SCHMAFIX uses 0018-0020 (or 0018-0021 if 005 included)
3. Document in SCHMAFIX-6001 that 0017 was added as prerequisite

**Priority:** ⚠️ **Should fix - Addresses CRITICAL-2**

---

### WARNING-3: Migration 004 Column Names Don't Match Analysis Document

**Affected Tickets:** SCHMAFIX-1001, SCHMAFIX-5001

**Concern:**
Migration 004 creates `worktree_index_state` table with column `last_tree_sha`, but:
- Analysis.md line 14: States database has `tree_sha` column
- Migration 004 line 104: Creates `last_tree_sha` column

**Evidence:**
```sql
-- Migration 004 line 104
CREATE TABLE IF NOT EXISTS maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY,
  last_tree_sha TEXT NOT NULL,  -- <-- Named "last_tree_sha"
  ...
);
```

But analysis.md states:
```markdown
Database Schema (as of 2025-11-09):
- ✅ worktree_index_state table exists  <!-- Already exists? -->
```

**Possible Scenarios:**
1. Table already exists with column named `tree_sha` (not `last_tree_sha`)
2. Migration 004 will fail: `ADD COLUMN last_tree_sha` when `tree_sha` already exists
3. OR: Analysis is wrong, table doesn't actually exist yet

**Impact:**
- ⚠️ Migration 004 may fail if column name mismatch
- ⚠️ Manual validation will fail
- ⚠️ Confusion about actual database state

**Required Action:**
1. Check actual production database schema BEFORE starting SCHMAFIX
2. Run: `\d maproom.worktree_index_state` to see column names
3. If `tree_sha` exists, update migration 004 to use `tree_sha` (not `last_tree_sha`)
4. If table doesn't exist, update analysis.md to reflect correct state
5. Add to SCHMAFIX-5001 manual checklist: "Verify column name matches migration"

**Priority:** ⚠️ **Should verify before SCHMAFIX-1001**

---

### WARNING-4: Test Tickets Don't Account for Migration Execution Time

**Affected Tickets:** SCHMAFIX-3001, SCHMAFIX-3901, SCHMAFIX-4001

**Concern:**
Integration tests will spin up testcontainers and run all migrations (0000-0021).

**Timeline Assumptions from Plan:**
- plan.md line 226: "Expected: Realistic timeline (8-12 hours)"
- quality-strategy.md line 94: "Migration execution time: 15-70 seconds"

But migration 001 backfill (lines 54-100) processes chunks in batches of 1000:
- 10,000 chunks = 10 batches × ~5 seconds = 50 seconds
- 100,000 chunks = 100 batches × ~5 seconds = 500 seconds (8 minutes)

**Impact:**
- ⚠️ Integration tests may timeout waiting for migrations
- ⚠️ Testcontainers default timeout: 60 seconds
- ⚠️ Large test databases will cause CI failures

**Required Action:**
1. Update SCHMAFIX-3001: Increase test timeout to 5 minutes
2. Use small test datasets (< 1000 chunks) for integration tests
3. Add comment in migration 001: "Backfill time: ~5 seconds per 1000 chunks"
4. Update quality-strategy.md with realistic migration times

**Priority:** ⚠️ **Should address in SCHMAFIX-3001**

---

## Recommendations (Consider Improvements)

### RECOMMENDATION-1: Split Migration 001 Into Two Migrations

**Affected Tickets:** SCHMAFIX-1001, SCHMAFIX-2001

**Suggestion:**
Split migration 001 (blob_sha) into two smaller migrations:

**Migration 0018a: Add Column and Function**
```sql
CREATE FUNCTION maproom.compute_git_blob_sha(...);
ALTER TABLE maproom.chunks ADD COLUMN blob_sha TEXT;
CREATE INDEX idx_chunks_blob_sha ON maproom.chunks(blob_sha);
```

**Migration 0018b: Backfill Data**
```sql
UPDATE maproom.chunks SET blob_sha = compute_git_blob_sha(preview);
ALTER TABLE maproom.chunks ALTER COLUMN blob_sha SET NOT NULL;
```

**Benefits:**
- ✅ Smaller, focused migrations (easier to debug)
- ✅ Can test schema changes separately from data backfill
- ✅ Rollback is cleaner (just drop column if backfill fails)
- ✅ Follows single-responsibility principle

**Tradeoff:**
- More migration files to manage
- Slightly more complex migration runner updates

**Priority:** 💡 **Optional improvement - Consider for quality**

---

### RECOMMENDATION-2: Add Migration Dry-Run Command

**Affected Tickets:** SCHMAFIX-5001

**Suggestion:**
Add a `--dry-run` flag to the Rust migration command:
```bash
crewchief-maproom db --dry-run
```

This would:
1. Connect to database
2. Check which migrations would be applied
3. Show SQL content without executing
4. Report estimated migration time

**Benefits:**
- ✅ Safer manual validation (see what will happen first)
- ✅ Easier debugging (inspect SQL before execution)
- ✅ Better documentation (show users what migrations do)

**Implementation:**
Update `crates/maproom/src/db/queries.rs` to add dry_run parameter.

**Priority:** 💡 **Optional improvement - Enhances safety**

---

## Ticket Actions Required

### Tickets to Rework

#### SCHMAFIX-1001: Copy and Adapt Migration SQL Files
**Changes Required:**
1. **Remove migration 005 from scope** - Will delete all production data
2. Copy only 3 migrations (001, 002, 004), not 4
3. Modify migration 001 to remove CONCURRENT and batched COMMIT
4. Add header warning about blob_sha computed from preview (not full content)
5. Update acceptance criteria: 3 files (not 4)
6. Target files: 0018, 0019, 0020 (not 0021)

**Rationale:** Migration 005 is destructive and incompatible with production use.

---

#### SCHMAFIX-2001: Update Rust Migration Runner
**Changes Required:**
1. **Add migration 0017 FIRST** (prerequisite, not part of SCHMAFIX)
2. Add only 3 migrations (0018, 0019, 0020), not 4
3. Set migration 0018 `concurrent = false` (transaction-safe after fixing migration 001)
4. Set migrations 0019, 0020 `concurrent = false`
5. Update acceptance criteria: 3 migrations (not 4)

**Rationale:** Migration 0017 missing causes numbering conflict. Migration 005 removed.

---

#### SCHMAFIX-3001: Write Migration Integration Tests
**Changes Required:**
1. Test expects 20 total migrations (0000-0019), not 22
2. Update test names: "test_fresh_database_migrations" checks version 19 (not 21)
3. Update "test_incremental_migration": Apply 0018-0019 (not 0018-0021)
4. Update schema validation: Check blob_sha and code_embeddings, but NOT relpath/content (removed with 005)
5. Increase test timeout to 5 minutes (migration backfill may take time)
6. Use small test dataset (< 1000 chunks)

**Rationale:** Migration count reduced from 22 to 20 (removed 005, added 0017 separately).

---

#### SCHMAFIX-3901: Run Migration Integration Tests
**Changes Required:**
1. Update expected test count: 4 tests (unchanged)
2. Update expected schema version: 19 (not 21)
3. Update expected output to show migrations 0018-0019 (not 0018-0021)

**Rationale:** Reflects revised migration count.

---

#### SCHMAFIX-4001: Write MCP Integration Tests
**Changes Required:**
1. Remove test for relpath/content columns (only added by migration 005, which is removed)
2. Keep tests for: code_embeddings table, blob_sha column, worktree_ids column
3. Update comments to reference 20 total migrations (not 22)

**Rationale:** Migration 005 schema changes not included.

---

#### SCHMAFIX-5001: Manual Migration Validation
**Changes Required:**
1. Update expected schema version: 19 (not 21)
2. Remove validation of relpath/content columns
3. Add manual step: Verify blob_sha values look reasonable (not all NULL)
4. Add manual step: Check migration 001 backfill completed without errors
5. Update validation query: Check version 19 is latest
6. Add warning: "blob_sha computed from preview, may not match full content"

**Rationale:** Revised schema and migration concerns.

---

#### SCHMAFIX-6001: Update Documentation
**Changes Required:**
1. Document only 3 migrations (0018-0019), not 4
2. Add note explaining why migration 005 was excluded (data preservation)
3. Update migration mapping table: Only show 001→0018, 002→0019, 004→0019
4. Add warning in DATABASE_ARCHITECTURE.md: "blob_sha computed from preview (interim solution)"
5. Document that migration 0017 was added as prerequisite

**Rationale:** Accurate documentation of actual changes.

---

### Tickets to Create (Prerequisites)

#### SCHMAFIX-0001: Add Migration 0017 to Rust Runner (NEW)
**Priority:** 🚨 **MUST COMPLETE BEFORE SCHMAFIX-1001**

**Summary:**
Add missing migration 0017 (`fix_index_size_limits.sql`) to Rust migration runner in `crates/maproom/src/db/queries.rs`.

**Acceptance Criteria:**
- Migration 0017 added to migrations array
- Set `use_concurrent_handler = true` (uses CREATE INDEX CONCURRENTLY)
- Code compiles without errors
- Migration 0017 runs successfully on test database

**Agent:** rust-indexer-engineer
**Estimated Effort:** 30 minutes
**Blocking:** All SCHMAFIX tickets

---

### Tickets to Defer

**None.** All tickets are relevant to project goals after rework.

---

### Tickets to Skip

**None.** All tickets needed after rework.

---

### Tickets to Split

**None.** Ticket scopes are appropriate after rework.

---

### Tickets to Merge

**None.** Tickets are appropriately granular.

---

## Integration Assessment

### Overall Integration Health: ⚠️ **NEEDS IMPROVEMENT**

**Key Integration Points:**

1. **Rust ↔ Database:** ✅ Good
   - Migration runner framework is solid
   - Transaction handling is correct
   - Idempotency support via schema_migrations table

2. **MCP ↔ Database:** ⚠️ **Fragile**
   - MCP code assumes code_embeddings table exists (will crash without it)
   - Migration 002 fixes this - GOOD
   - But blob_sha computed from preview may cause issues later

3. **Migration Files ↔ Production Data:** 🚨 **BROKEN**
   - Migration 005 will delete all data (CRITICAL)
   - Migration 001 uses preview instead of content (WARNING)
   - Must remove migration 005 entirely

4. **Migration Numbering ↔ Filesystem:** ⚠️ **Inconsistent**
   - Migration 0017 exists but not in runner
   - Must add 0017 before starting SCHMAFIX

### Risks to Existing Functionality

**HIGH RISK:**
- 🚨 Migration 005 will delete all indexed code (100% data loss)
- 🚨 Migration 001 transaction errors will block project
- 🚨 Migration 0017 missing causes numbering conflicts

**MEDIUM RISK:**
- ⚠️ Blob SHA values may be incorrect (computed from preview)
- ⚠️ Column name mismatches may cause migration failures
- ⚠️ Long migration times may timeout tests

**LOW RISK:**
- Migration 002 is straightforward (CREATE TABLE code_embeddings)
- Migration 004 is well-tested (worktree tracking from BRANCHX)
- Rust migration framework is robust

### Mitigation Recommendations

1. **IMMEDIATELY:** Remove migration 005 from all tickets
2. **BEFORE EXECUTION:** Add migration 0017 to queries.rs
3. **BEFORE EXECUTION:** Simplify migration 001 (remove CONCURRENT, batched commits)
4. **DURING VALIDATION:** Verify actual database schema matches assumptions
5. **AFTER EXECUTION:** Re-evaluate blob_sha correctness for BLOBSHA-IMPL

---

## Dependency Analysis

### Dependency Chain Validation: ⚠️ **MOSTLY VALID (with fixes)**

**Critical Path:**
```
[NEW] SCHMAFIX-0001 (Add migration 0017) ← MUST ADD
    ↓
SCHMAFIX-1001 (Copy 3 migrations: 001, 002, 004) ← Rework to remove 005
    ↓
SCHMAFIX-2001 (Add 3 migrations to runner: 0018, 0019, 0020) ← Rework
    ↓
SCHMAFIX-3001 (Write tests for 20 migrations) ← Rework
    ↓
SCHMAFIX-3901 (Run tests) ← Update expectations
    ↓
SCHMAFIX-4001 (MCP integration tests) ← Remove relpath/content tests
    ↓
SCHMAFIX-5001 (Manual validation for version 19) ← Rework
    ↓
SCHMAFIX-6001 (Documentation for 3 migrations) ← Rework
```

**Dependency Issues:**

1. **Missing Prerequisite:** SCHMAFIX-0001 must be created and completed first
2. **Circular Dependency:** None found ✅
3. **Blocking Dependencies:** All properly identified after adding SCHMAFIX-0001

### Sequencing Recommendations

**Phase 0 (New):**
- SCHMAFIX-0001: Add migration 0017 (30 min)

**Phase 1:**
- SCHMAFIX-1001: Copy 3 migrations (1-2 hours, with rework)

**Phase 2:**
- SCHMAFIX-2001: Update Rust runner (30 min-1 hour)

**Phase 3:**
- SCHMAFIX-3001: Write tests (2-3 hours, with rework)
- SCHMAFIX-3901: Run tests (15-30 min) ← CRITICAL QUALITY GATE

**Phase 4:**
- SCHMAFIX-4001: MCP integration tests (1-2 hours, with rework)

**Phase 5:**
- SCHMAFIX-5001: Manual validation (1-2 hours, with rework) ← FINAL QUALITY GATE

**Phase 6:**
- SCHMAFIX-6001: Documentation (1-2 hours, with rework)

### Parallel Execution Opportunities

**After Fixes:**
- SCHMAFIX-3001 (Rust tests) and SCHMAFIX-4001 (MCP tests) can be written in parallel
- SCHMAFIX-6001 (Documentation) can start during SCHMAFIX-5001 (Manual validation)

**Critical:** SCHMAFIX-3901 MUST pass before any Phase 4+ tickets begin.

---

## Recommendations for Execution

### Pre-Execution Checklist

**BEFORE starting SCHMAFIX-1001:**
- [ ] Create and complete SCHMAFIX-0001 (add migration 0017)
- [ ] Verify migration 0017 runs successfully on dev database
- [ ] Verify actual production database schema (worktree_ids, worktree_index_state)
- [ ] Back up production database (if testing on prod-like data)
- [ ] Rework all 7 tickets to remove migration 005
- [ ] Simplify migration 001 (remove CONCURRENT, batched commits)
- [ ] Update all ticket acceptance criteria for 3 migrations (not 4)

### Risk Mitigation Strategies

**For Migration Execution:**
1. Test on copy of production database first (not prod directly)
2. Verify migration 001 backfill completes without errors
3. Check blob_sha values are non-NULL and look reasonable
4. Verify code_embeddings table created successfully
5. Confirm MCP vector search doesn't crash after migrations

**For Data Preservation:**
1. Take database snapshot before running migrations
2. Verify chunk count before and after migrations
3. Spot-check that existing chunks still searchable
4. Rollback plan: Restore from snapshot if data loss detected

**For Migration Failures:**
1. Migrations run in transactions (rollback on error)
2. Test on fresh database AND existing database
3. Have manual SQL ready to fix common issues
4. Document rollback procedure for each migration

### Key Checkpoints During Execution

**After SCHMAFIX-1001:**
- ✅ Verify 3 migration files copied (not 4)
- ✅ Check migration 001 simplified (no CONCURRENT/COMMIT)
- ✅ Confirm headers reference SCHMAFIX-1001

**After SCHMAFIX-2001:**
- ✅ Verify migrations array has 0018, 0019, 0020 (not 0021)
- ✅ Check all set `concurrent = false`
- ✅ Confirm code compiles without errors

**After SCHMAFIX-3901 (CRITICAL):**
- ✅ ALL 4 tests must pass (0 failures)
- ✅ Fresh database test shows version 19
- ✅ Incremental test applies 0018-0019 correctly
- ✅ Schema validation confirms blob_sha, code_embeddings exist
- 🚨 **STOP if any test fails** - do not proceed to Phase 4

**After SCHMAFIX-5001 (FINAL):**
- ✅ Migrations applied to fresh database (no errors)
- ✅ Migrations applied to v0.16 database (incremental works)
- ✅ MCP server starts without "table does not exist" errors
- ✅ Vector search executes without crash
- ✅ Manual checklist 100% complete
- ✅ No data loss (chunk count preserved)

### Success Criteria for Project Completion

**Quantitative:**
- ✅ 8 tickets completed (including new SCHMAFIX-0001)
- ✅ 100% test pass rate (Rust + MCP)
- ✅ 0 migration failures
- ✅ Manual checklist 100% complete
- ✅ Schema version = 19 (not 21)

**Qualitative:**
- ✅ MCP vector search doesn't crash
- ✅ Schema has blob_sha, code_embeddings, worktree_ids
- ✅ Migrations are idempotent
- ✅ Documentation updated
- ✅ NO data loss

**Evidence:**
- `SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 1` returns version 19
- `\d maproom.chunks` shows blob_sha and worktree_ids columns
- `\dt maproom.code_embeddings` shows table exists
- `SELECT COUNT(*) FROM maproom.chunks` returns same count as before migrations
- MCP search query executes without "relation does not exist" error

---

## Conclusion

The SCHMAFIX project tickets are well-structured and comprehensive, but contain **3 critical issues** that block execution:

1. 🚨 **Migration 005 will delete all production data** (MUST REMOVE)
2. 🚨 **Migration 0017 missing from runner** (MUST ADD FIRST)
3. 🚨 **Migration 001 incompatible with transactions** (MUST SIMPLIFY)

**Recommendation:** **DO NOT EXECUTE** until all critical issues are resolved.

**Next Steps:**
1. Create SCHMAFIX-0001 (add migration 0017) as prerequisite
2. Rework all 7 tickets to remove migration 005
3. Simplify migration 001 to be transaction-safe
4. Re-review tickets after rework
5. Proceed with execution only after rework validated

**Estimated Rework Time:** 2-3 hours
**Estimated Total Project Time (after fixes):** 10-14 hours (realistic)

---

**Review Status:** ✅ COMPLETE
**Recommendation:** ⚠️ **REWORK REQUIRED BEFORE EXECUTION**
