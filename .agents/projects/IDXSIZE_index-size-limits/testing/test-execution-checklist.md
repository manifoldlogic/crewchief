# Migration 0017 Test Execution Checklist

**Quick Reference**: Use this checklist during test execution. Full details in `production-clone-test-procedure.md`.

**Date**: _______________
**Tester**: _______________
**Database Size**: _______________

---

## Pre-Test Setup

- [ ] Docker running and accessible
- [ ] Production database access confirmed
- [ ] Sufficient disk space (2x DB size + 500MB)
- [ ] Migration file verified at `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- [ ] Test results file created

## Environment Setup

- [ ] **Step 1**: Production backup created and verified
  - Backup size: _______________
  - Backup complete footer verified

- [ ] **Step 2**: Test PostgreSQL instance running (port 5433)
  - Container name: `migration-test-pg`
  - Container running: `docker ps | grep migration-test-pg`

- [ ] **Step 3**: Backup restored to test instance
  - Schema verified
  - Tables exist: repos, worktrees, files, chunks, chunk_relationships

## Pre-Migration Baseline

- [ ] **Step 4.1**: Chunk count recorded
  - Total chunks: _______________
  - Small preview (<= 2000 bytes): _______________%
  - Large preview (> 2000 bytes): _______________%

- [ ] **Step 4.2**: Current indexes documented
  - `idx_chunks_search_covering` size: _______________

- [ ] **Step 4.3**: Storage metrics captured
  - Table size: _______________
  - Total index size: _______________
  - Total size: _______________

- [ ] **Step 4.4**: Query performance baseline
  - Query 1 time: _______________ms
  - Query 2 time: _______________ms
  - Query 3 time: _______________ms
  - Query 4 time: _______________ms

## Migration Execution

- [ ] **Step 5**: Migration executed
  - Start time: _______________
  - End time: _______________
  - Duration: _______________seconds
  - Exit code: 0 (success)
  - Errors: [ ] None  [ ] Found: _______________

## Post-Migration Validation

### Index Changes

- [ ] **Step 6.1**: New indexes created
  - [ ] `idx_chunks_search_small_preview` exists
    - Size: _______________
    - Comment verified
  - [ ] `idx_chunks_search_basic` exists
    - Size: _______________
    - Comment verified

- [ ] **Step 6.2**: Old index removed
  - [ ] `idx_chunks_search_covering` does NOT exist

### Data Integrity

- [ ] **Step 7**: Zero data loss
  - Pre-migration chunks: _______________
  - Post-migration chunks: _______________
  - Match: [ ] Yes  [ ] **NO - CRITICAL**
  - Large preview chunks preserved: _______________

### Storage Impact

- [ ] **Step 8**: Storage measurements
  - Post-migration table size: _______________
  - Post-migration index size: _______________
  - Storage increase: _______________MB (_______________%)
  - Within range (25-40%): [ ] Yes  [ ] No

## Critical Path Testing

- [ ] **Step 9.1**: Query 1 - File search
  - Execution time: _______________ms
  - Index used: _______________
  - Performance: [ ] Good (<20ms)  [ ] Acceptable (20-50ms)  [ ] Slow (>50ms)

- [ ] **Step 9.2**: Query 2 - Aggregation
  - Execution time: _______________ms
  - Index used: _______________
  - Performance: [ ] Good (<20ms)  [ ] Acceptable (20-50ms)  [ ] Slow (>50ms)

- [ ] **Step 9.3**: Query 3 - Large preview **CRITICAL**
  - Execution time: _______________ms
  - Query succeeded: [ ] **Yes**  [ ] No (FAILURE)
  - Index used: _______________
  - Performance: [ ] Good (<30ms)  [ ] Acceptable (30-50ms)  [ ] Slow (>50ms)

- [ ] **Step 9.4**: Query 4 - Symbol search
  - Execution time: _______________ms
  - Index used: _______________

- [ ] **Step 10**: Index usage verified
  - `idx_chunks_search_small_preview` scans: _______________
  - `idx_chunks_search_basic` scans: _______________
  - Both indexes used: [ ] Yes  [ ] No

## Log Verification

- [ ] **Step 11**: PostgreSQL logs checked
  - Errors: [ ] None  [ ] Found: _______________
  - Warnings: [ ] None  [ ] Found: _______________
  - Slow queries (>100ms): [ ] None  [ ] Found: _______________

## Results Documentation

- [ ] **Step 12**: Test results documented
  - All measurements recorded
  - Success criteria evaluated
  - Issues documented
  - Follow-up actions listed

## Cleanup

- [ ] **Step 13**: Test results saved to project directory
- [ ] Test environment removed (container stopped/deleted)
- [ ] Backup files archived or deleted

---

## Success Criteria Quick Check

### MUST PASS (Blocking)

- [ ] Migration completed without errors
- [ ] Zero data loss (chunk count matches)
- [ ] Old index dropped, 2 new indexes created
- [ ] Large preview query succeeds (no errors)
- [ ] All critical path queries return correct results
- [ ] Query performance within ±30% of baseline

### SHOULD PASS (Investigate if failed)

- [ ] Storage increase < 40%
- [ ] Migration duration < 10 minutes
- [ ] No PostgreSQL errors in logs
- [ ] Both indexes show usage statistics

---

## Final Result

**OVERALL TEST RESULT**: [ ] **PASS**  [ ] **FAIL**  [ ] **CONDITIONAL PASS**

**Ready for Production**: [ ] **YES**  [ ] **NO** (reason: _______________)

**Approver**: _______________
**Approval Date**: _______________

---

## Quick Troubleshooting

### If migration fails:
1. Check PostgreSQL logs: `docker logs migration-test-pg`
2. Verify migration file syntax
3. Check disk space
4. Review error messages in test results file

### If queries are slow:
1. Run `ANALYZE maproom.chunks;`
2. Check EXPLAIN plans for index usage
3. Verify test instance resources (CPU/RAM)

### If data loss detected:
1. **DO NOT PROCEED TO PRODUCTION**
2. Investigate cause immediately
3. Re-run test with fresh backup
4. Document findings in test results

### If indexes not created:
1. Check for errors during CREATE INDEX CONCURRENTLY
2. Verify PostgreSQL version (should be 15+)
3. Check table permissions
4. Review PostgreSQL logs

---

**Document Version**: 1.0
**Last Updated**: 2025-11-09
