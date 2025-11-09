# SCHMAFIX Ticket Index

**Project:** SCHMAFIX_schema-migration-integration
**Created:** 2025-11-09
**Total Tickets:** 8 (includes new prerequisite SCHMAFIX-0001)

## Project Overview

Integrate existing BLOBSHA/BRANCHX migration SQL files into the Rust migration runner to fix the critical disconnect between database schema and code expectations. This resolves the issue where MCP TypeScript code references `code_embeddings` table that doesn't exist, causing vector search to crash.

**Scope Revision**: Migration 005 excluded (destructive TRUNCATE). Migration 0017 added as prerequisite. Total: 3 new migrations (0018-0020), not 4.

## Ticket Status Summary

- **Total:** 8 tickets (Phase 0-6)
- **Completed:** 0
- **In Progress:** 0
- **Pending:** 8

## Tickets by Phase

### Phase 0: Prerequisites (1 ticket - NEW)

#### SCHMAFIX-0001: Add Missing Migration 0017 to Rust Runner
- **File:** `SCHMAFIX-0001_add-migration-0017-to-runner.md`
- **Status:** ⏳ Pending
- **Agent:** rust-indexer-engineer
- **Estimated Effort:** 30 minutes
- **Summary:** Add migration 0017 (`fix_index_size_limits.sql`) to the Rust migration runner. This migration exists in the filesystem but was never added to the migrations array, creating a numbering gap that blocks SCHMAFIX execution.
- **Deliverables:**
  - Updated `crates/maproom/src/db/queries.rs` with migration 0017 entry
- **Dependencies:** None (prerequisite for all other SCHMAFIX tickets)
- **Plan Reference:** Created from review report CRITICAL-2

### Phase 1: Migration File Preparation (1 ticket)

#### SCHMAFIX-1001: Copy and Adapt Migration SQL Files
- **File:** `SCHMAFIX-1001_copy-migration-sql-files.md`
- **Status:** ⏳ Pending
- **Agent:** rust-indexer-engineer
- **Estimated Effort:** 1-2 hours
- **Summary:** Copy 3 migration SQL files from `packages/maproom-mcp/migrations/` to `crates/maproom/migrations/` as migrations 0018-0020. Simplify migration 001 for transaction safety. Migration 005 excluded (destructive TRUNCATE).
- **Deliverables:**
  - `crates/maproom/migrations/0018_add_blob_sha.sql` (simplified)
  - `crates/maproom/migrations/0019_create_code_embeddings.sql`
  - `crates/maproom/migrations/0020_add_worktree_tracking.sql`
- **Dependencies:** SCHMAFIX-0001 (BLOCKER)
- **Plan Reference:** Phase 1 (lines 26-38), Review Report CRITICAL-1, CRITICAL-3

### Phase 2: Rust Migration Runner Update (1 ticket)

#### SCHMAFIX-2001: Update Rust Migration Runner
- **File:** `SCHMAFIX-2001_update-rust-migration-runner.md`
- **Status:** ⏳ Pending
- **Agent:** rust-indexer-engineer
- **Estimated Effort:** 30 minutes - 1 hour
- **Summary:** Update `crates/maproom/src/db/queries.rs` to include migrations 0018-0020 in the migrations array, enabling the Rust binary to execute the new schema migrations.
- **Deliverables:**
  - Updated `crates/maproom/src/db/queries.rs` with 4 new migration entries
- **Dependencies:**
  - SCHMAFIX-1001 (BLOCKER)
- **Plan Reference:** Phase 2 (lines 40-50)

### Phase 3: Migration Testing (2 tickets)

#### SCHMAFIX-3001: Write Migration Integration Tests
- **File:** `SCHMAFIX-3001_migration-integration-tests.md`
- **Status:** ⏳ Pending
- **Agent:** rust-indexer-engineer
- **Estimated Effort:** 2-3 hours
- **Summary:** Create Rust integration tests to verify migrations 0018-0020 work correctly on fresh databases, existing databases (incremental upgrade), and when run multiple times (idempotency).
- **Deliverables:**
  - `crates/maproom/tests/migration_integration.rs` with 4 comprehensive tests
- **Dependencies:**
  - SCHMAFIX-1001 (BLOCKER)
  - SCHMAFIX-2001 (BLOCKER)
- **Plan Reference:** Phase 3 (lines 52-63)

#### SCHMAFIX-3901: Run Migration Integration Tests
- **File:** `SCHMAFIX-3901_run-migration-integration-tests.md`
- **Status:** ⏳ Pending
- **Agent:** unit-test-runner
- **Estimated Effort:** 15-30 minutes
- **Summary:** Execute the Rust integration tests created in SCHMAFIX-3001 and report results. This is a critical quality gate - all tests must pass before proceeding to MCP integration.
- **Deliverables:**
  - Test execution report (4/4 tests passing)
- **Dependencies:**
  - SCHMAFIX-1001 (BLOCKER)
  - SCHMAFIX-2001 (BLOCKER)
  - SCHMAFIX-3001 (BLOCKER)
- **Plan Reference:** Phase 3, Critical Path (lines 167-192)

### Phase 4: MCP Integration Verification (1 ticket)

#### SCHMAFIX-4001: Write MCP Integration Tests
- **File:** `SCHMAFIX-4001_mcp-integration-tests.md`
- **Status:** ⏳ Pending
- **Agent:** integration-tester
- **Estimated Effort:** 1-2 hours
- **Summary:** Create TypeScript integration tests to verify the MCP server works correctly with the new database schema, specifically confirming the code_embeddings table exists and vector search doesn't crash.
- **Deliverables:**
  - `packages/maproom-mcp/tests/migrations/schema-integration.test.ts` with 5 tests
- **Dependencies:**
  - SCHMAFIX-1001 (BLOCKER)
  - SCHMAFIX-2001 (BLOCKER)
  - SCHMAFIX-3901 (BLOCKER)
- **Plan Reference:** Phase 4 (lines 65-76)

### Phase 5: Manual Validation (1 ticket)

#### SCHMAFIX-5001: Manual Migration Validation
- **File:** `SCHMAFIX-5001_manual-migration-validation.md`
- **Status:** ⏳ Pending
- **Agent:** verify-ticket
- **Estimated Effort:** 1-2 hours
- **Summary:** Manually validate that migrations apply correctly to both fresh and existing databases, and that the MCP server starts and executes vector search without errors. Complete the manual validation checklist from quality-strategy.md.
- **Deliverables:**
  - Completed manual validation checklist (100%)
  - Fresh database migration verified
  - Incremental migration verified (v0.17 → v0.21)
  - MCP vector search functional
- **Dependencies:**
  - SCHMAFIX-1001 (BLOCKER)
  - SCHMAFIX-2001 (BLOCKER)
  - SCHMAFIX-3901 (BLOCKER)
  - SCHMAFIX-4001 (BLOCKER)
- **Plan Reference:** Phase 5 (lines 77-89)

### Phase 6: Documentation (1 ticket)

#### SCHMAFIX-6001: Update Documentation
- **File:** `SCHMAFIX-6001_update-documentation.md`
- **Status:** ⏳ Pending
- **Agent:** general-purpose
- **Estimated Effort:** 1-2 hours
- **Summary:** Update project documentation to reflect that Rust owns all migrations, document the new schema elements (blob_sha, code_embeddings, worktree tracking), and add comments to migration SQL files.
- **Deliverables:**
  - Updated `packages/maproom-mcp/migrations/README.md` (mark as historical)
  - Updated `crates/maproom/CLAUDE.md` (document migrations 0018-0020)
  - Updated `docs/architecture/DATABASE_ARCHITECTURE.md` (new schema documentation)
  - Migration file headers with explanatory comments
- **Dependencies:**
  - SCHMAFIX-5001 (RECOMMENDED - for accuracy)
- **Plan Reference:** Phase 6 (lines 90-101)

## Critical Path

The project follows a strict sequential dependency chain:

```
SCHMAFIX-0001 (Add migration 0017) ← NEW PREREQUISITE
    ↓
SCHMAFIX-1001 (Copy & simplify 3 SQL files)
    ↓
SCHMAFIX-2001 (Update Rust runner with 0018-0020)
    ↓
SCHMAFIX-3001 (Write tests for 20 migrations)
    ↓
SCHMAFIX-3901 (Run tests) ← CRITICAL QUALITY GATE
    ↓
SCHMAFIX-4001 (MCP integration tests)
    ↓
SCHMAFIX-5001 (Manual validation) ← FINAL QUALITY GATE
    ↓
SCHMAFIX-6001 (Documentation)
```

**Blocking Dependencies:**
- **MUST complete SCHMAFIX-0001** before starting SCHMAFIX-1001 (numbering gap)
- **MUST pass SCHMAFIX-3901** before proceeding to SCHMAFIX-4001
- **MUST pass SCHMAFIX-5001** before marking project complete

## Parallel Opportunities

While the critical path is sequential, some work can be prepared in parallel:
- SCHMAFIX-3001 (Rust tests) and SCHMAFIX-4001 (MCP tests) can be written in parallel (different codebases)
- SCHMAFIX-6001 (Documentation) can start during SCHMAFIX-5001 (Manual validation)

However, **execution must remain sequential** to ensure quality gates are respected.

## Execution Commands

### Single Ticket Workflow
```bash
/single-ticket SCHMAFIX-1001  # Execute, test, verify, commit one ticket
```

### Full Project Workflow
```bash
/work-on-project SCHMAFIX  # Execute all tickets sequentially
```

### Review Tickets
```bash
/review-tickets SCHMAFIX  # Review ticket quality before execution
```

## Success Criteria

**Project Complete When:**
1. ✅ All 8 tickets have checkmarks in `Task completed` section
2. ✅ Migration 0017 successfully added to Rust runner (SCHMAFIX-0001)
3. ✅ All Rust migration tests pass (SCHMAFIX-3901)
4. ✅ All MCP integration tests pass (SCHMAFIX-4001)
5. ✅ Manual validation checklist 100% complete (SCHMAFIX-5001)
6. ✅ Documentation updated (SCHMAFIX-6001)
7. ✅ Final commit created with conventional commit message

**Evidence of Success:**
- `SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 1` shows version = 19 (after all migrations)
- `\d maproom.chunks` shows `blob_sha` and `worktree_ids` columns
- `\dt maproom.code_embeddings` shows table exists
- `\dt maproom.worktree_index_state` shows table exists
- `mcp__maproom__search({ mode: 'vector', query: 'test' })` doesn't crash
- No "table does not exist" errors in MCP logs

## Timeline Estimates

**Per Ticket:**
- SCHMAFIX-0001: 30 min (add migration 0017)
- SCHMAFIX-1001: 1-2 hours (copy & simplify 3 migrations)
- SCHMAFIX-2001: 30 min - 1 hour (update runner)
- SCHMAFIX-3001: 2-3 hours (write tests)
- SCHMAFIX-3901: 15-30 min (run tests)
- SCHMAFIX-4001: 1-2 hours (MCP tests)
- SCHMAFIX-5001: 1-2 hours (manual validation)
- SCHMAFIX-6001: 1-2 hours (documentation)

**Total Project:**
- **Optimistic:** 8.5 hours (all tests pass first try)
- **Realistic:** 13 hours (some iteration needed)
- **Pessimistic:** 21 hours (major schema conflicts)

## Risk Summary

### High Risk (Must Address)
- Migration 0018 backfill failure → Use transactions, test on small dataset first
- Schema drift between environments → Use IF NOT EXISTS extensively
- Data loss during migration → Test thoroughly before production

### Medium Risk (Monitor)
- Database connection exposure → Use environment variables
- Migration ordering race → Advisory locks (already implemented)
- Breaking changes to data → Additive only (enforced by design)

### Low Risk (Accept)
- Brief service interruption (15-70 seconds) during migration
- Unauthorized schema inspection (database-level security)

## Related Documents

- **Project README:** `.agents/projects/SCHMAFIX_schema-migration-integration/README.md`
- **Execution Plan:** `.agents/projects/SCHMAFIX_schema-migration-integration/planning/plan.md`
- **Architecture:** `.agents/projects/SCHMAFIX_schema-migration-integration/planning/architecture.md`
- **Quality Strategy:** `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md`
- **Security Review:** `.agents/projects/SCHMAFIX_schema-migration-integration/planning/security-review.md`
- **Analysis:** `.agents/projects/SCHMAFIX_schema-migration-integration/planning/analysis.md`

## Notes

- **Migration Numbering:** Follows sequential numbering (0018-0020 after existing 0000-0017)
- **Ticket Numbering:** Phase-based (1xxx, 2xxx, 3xxx, etc.) with test tickets at x9xx
- **Agent Roles:** Primary agent assigned per ticket, supporting agents as needed
- **Test Strategy:** Comprehensive automated tests (Rust + TypeScript) plus manual validation
- **Documentation:** Updated throughout to reflect Rust migration ownership

---

**Last Updated:** 2025-11-09
**Status:** Ready for execution via `/work-on-project SCHMAFIX` or individual `/single-ticket` commands
