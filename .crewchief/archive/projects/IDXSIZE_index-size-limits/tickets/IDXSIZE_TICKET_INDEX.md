# IDXSIZE Ticket Index

**Project**: Index Size Limit Fix - PostgreSQL B-tree Migration
**Project Slug**: IDXSIZE
**Status**: Ready for Implementation
**Created**: 2025-11-09

## Overview

This project eliminates PostgreSQL B-tree index size limit errors (2704-byte max) that occur when indexing real-world codebases with large preview text. The solution implements a multi-index strategy replacing the single failing `idx_chunks_search_covering` with three specialized indexes.

**Planning Documents**:
- [Analysis](../planning/analysis.md) - Problem deep-dive and impact analysis
- [Architecture](../planning/architecture.md) - Multi-index solution design
- [Quality Strategy](../planning/quality-strategy.md) - 4-level test pyramid
- [Security Review](../planning/security-review.md) - Risk assessment
- [Implementation Plan](../planning/plan.md) - Detailed execution roadmap

---

## Phase 1: Migration Development (Day 1)

**Goal**: Create migration SQL, rollback script, and documentation

| Ticket ID | Title | Agent | Status | Plan Reference |
|-----------|-------|-------|--------|----------------|
| [IDXSIZE-1001](./IDXSIZE-1001_create-migration-sql.md) | Create migration SQL file | database-engineer | ⬜ Pending | Step 1.1 (lines 24-80) |
| [IDXSIZE-1002](./IDXSIZE-1002_create-rollback-script.md) | Create rollback script | database-engineer | ⬜ Pending | Step 1.2 (lines 89-118) |
| [IDXSIZE-1003](./IDXSIZE-1003_update-documentation.md) | Update documentation | general-purpose | ⬜ Pending | Step 1.3 (lines 119-143) |
| [IDXSIZE-1004](./IDXSIZE-1004_test-phase1-deliverables.md) | Test Phase 1 deliverables | unit-test-runner | ⬜ Pending | Quality L1 testing |

**Dependencies**: None (first phase)

**Deliverables**:
- Migration SQL: `crates/maproom/migrations/0013_fix_index_size_limits.sql`
- Rollback script: `crates/maproom/migrations/rollback/0013_rollback.sql`
- Updated CHANGELOG.md and DATABASE_INDICES.md

---

## Phase 2: Testing and Validation (Day 2)

**Goal**: Automated testing (L1-L3) and query performance validation

| Ticket ID | Title | Agent | Status | Plan Reference |
|-----------|-------|-------|--------|----------------|
| [IDXSIZE-2001](./IDXSIZE-2001_create-automated-test-suite.md) | Create automated test suite | database-engineer | ⬜ Pending | Step 2.1 (lines 149-224) |
| [IDXSIZE-2002](./IDXSIZE-2002_query-performance-testing.md) | Query performance testing | database-engineer | ⬜ Pending | Step 2.2 (lines 226-256) |
| [IDXSIZE-2003](./IDXSIZE-2003_production-clone-test.md) | Production clone test (manual) | database-engineer | ⬜ Pending | Step 2.3 (lines 258-298) |
| [IDXSIZE-2004](./IDXSIZE-2004_validate-phase-2-test-execution.md) | Validate Phase 2 test execution | unit-test-runner | ⬜ Pending | Quality validation |

**Dependencies**: Phase 1 complete

**Deliverables**:
- Test script: `crates/maproom/tests/test_index_migration.sh`
- Query performance test results
- Production clone test report with actual migration timing and metrics

---

## Phase 3: Production Deployment (Day 3 morning)

**Goal**: Deploy migration to production with monitoring

| Ticket ID | Title | Agent | Status | Plan Reference |
|-----------|-------|-------|--------|----------------|
| [IDXSIZE-3001](./IDXSIZE-3001_pre-deployment-checklist-and-backup.md) | Pre-deployment checklist and backup | database-engineer | ⬜ Pending | Step 3.1 (lines 305-334) |
| [IDXSIZE-3002](./IDXSIZE-3002_execute-production-migration.md) | Execute production migration | database-engineer | ⬜ Pending | Step 3.2 (lines 336-371) |
| [IDXSIZE-3003](./IDXSIZE-3003_post-deployment-monitoring.md) | Post-deployment monitoring | database-engineer | ⬜ Pending | Step 3.3 (lines 373-414) |

**Dependencies**: Phase 2 complete and validated

**Deliverables**:
- Production database backup
- Migration deployed successfully (<10 minutes)
- 24-hour monitoring report

---

## Phase 4: Documentation and Cleanup (Day 3 afternoon)

**Goal**: Document results and close out project

| Ticket ID | Title | Agent | Status | Plan Reference |
|-----------|-------|-------|--------|----------------|
| [IDXSIZE-4001](./IDXSIZE-4001_update-migration-log.md) | Update migration log | general-purpose | ⬜ Pending | Step 4.1 (lines 419-457) |
| [IDXSIZE-4002](./IDXSIZE-4002_verify-end-to-end-indexing.md) | Verify indexing works end-to-end | rust-indexer-engineer | ⬜ Pending | Step 4.2 (lines 459-476) |
| [IDXSIZE-4003](./IDXSIZE-4003_update-project-readme-completion.md) | Update project README with completion | general-purpose | ⬜ Pending | Step 4.3 (lines 478-509) |

**Dependencies**: Phase 3 complete

**Deliverables**:
- Migration log entry in `docs/migrations/README.md`
- End-to-end indexing verification on real codebase
- Project marked complete in README
- Project ready for archiving

---

## Ticket Workflow

Each ticket follows the standard workflow:

1. **Implementation** - Primary agent completes work
2. **Testing** - unit-test-runner executes tests (test tickets only)
3. **Verification** - verify-ticket validates acceptance criteria
4. **Commit** - commit-ticket creates Conventional Commit

**Test Execution Requirements** (NEW):
- Test tickets MUST show test execution output (not just "tests pass")
- Required format: Command + Result + Full Output
- Verification will FAIL if test execution evidence missing

---

## Success Criteria

**Must-Have** (Blocking):
- ✅ Migration completes without errors
- ✅ All 3 new indexes created
- ✅ Can index large-preview chunks without errors (>2704 bytes)
- ✅ Query performance within ±30% of baseline
- ✅ Zero data loss

**Should-Have** (Investigate if Missing):
- 95%+ queries use index-only scans (partial index)
- Migration completes in <10 minutes
- Storage increase <40% (expected: ~31%)
- No PostgreSQL errors in logs

**Nice-to-Have** (Optimize Later):
- Index-only scan rate >98%
- Average query time <10ms
- Zero monitoring alerts

---

## Risk Summary

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Migration takes too long | Low | Medium | CREATE INDEX CONCURRENTLY (no locks) |
| Query performance degrades | Low | High | Baseline + ANALYZE + monitoring |
| Storage exceeds capacity | Low | Medium | Pre-calculate +31%, verify disk space |
| Rollback needed | Very Low | High | Backup + rollback script (prefer forward-fix) |

---

## Timeline

- **Day 1**: Phase 1 (Migration Development) - 4 tickets
- **Day 2**: Phase 2 (Testing and Validation) - 4 tickets
- **Day 3 AM**: Phase 3 (Production Deployment) - 3 tickets
- **Day 3 PM**: Phase 4 (Documentation and Cleanup) - 3 tickets
- **Buffer**: 0.5 days for unexpected issues

**Total**: 2-3 days wall time, 14 tickets

---

## Quick Reference Commands

### Check Index Status
```sql
SELECT indexname, pg_size_pretty(pg_relation_size(indexrelid))
FROM pg_indexes
WHERE tablename = 'chunks' AND indexname LIKE 'idx_chunks_search%';
```

### Test Query Performance
```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT symbol_name, preview FROM chunks WHERE file_id = X AND kind = 'function';
```

### Monitor Index Usage
```sql
SELECT * FROM pg_stat_user_indexes WHERE tablename = 'chunks';
```

### Emergency Rollback
```bash
psql $DATABASE_URL < migrations/rollback/0013_rollback.sql
```

---

**Created**: 2025-11-09
**Total Tickets**: 14 (4 per phase across 4 phases)
**Estimated Duration**: 2-3 days
**Risk Level**: Low (well-tested, rollback available)
**Impact**: High (fixes critical blocker affecting 50%+ of users)
