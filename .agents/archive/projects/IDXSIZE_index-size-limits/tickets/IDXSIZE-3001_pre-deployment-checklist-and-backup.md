# Ticket: IDXSIZE-3001: Pre-deployment checklist and backup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (manual verification documentation created)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This is a manual verification ticket - N/A for automated tests
- Verification is manual execution of pre-deployment checklist

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Execute pre-deployment checklist, create production database backup, verify prerequisites, and prepare for migration execution in production environment.

## Background
Before deploying the migration to production, we must verify all testing is complete, create a verified backup, check disk space, and ensure the team is ready. This is the critical safety gate before making changes to the production database.

This ticket implements Step 3.1 from `.agents/projects/IDXSIZE_index-size-limits/planning/plan.md` (lines 305-334).

The index size limit migration (max_index_size: 512KB) represents a significant schema change that affects all chunks. Before deployment, we need complete confidence that:
- All testing phases are complete (IDXSIZE-2004 verification passed)
- Production clone testing succeeded (IDXSIZE-2003)
- Rollback procedures are tested and ready (IDXSIZE-1002)
- Production data is safely backed up
- System resources are adequate for the migration

## Acceptance Criteria
- [x] Pre-deployment checklist document created with testing verification section
- [x] Production database backup procedures documented with commands
- [x] Backup integrity verification procedures documented
- [x] Disk space verification procedures documented with storage calculations
- [x] Migration window planning guidance documented
- [x] Team notification templates provided
- [x] Migration execution guide created for IDXSIZE-3002
- [x] All prerequisite ticket verifications documented (IDXSIZE-1001 through 2004)
- [x] Baseline metrics capture queries provided
- [x] Rollback readiness verification documented
- [x] Final go/no-go checklist with sign-off section

**Note**: This ticket creates DOCUMENTATION for manual pre-deployment verification. Actual execution of checklist (backup creation, production clone testing, etc.) is deferred to human operators before IDXSIZE-3002.

## Technical Requirements

### Pre-Flight Test Verification
- Verify IDXSIZE-2004 test execution output shows all tests passing
- Confirm IDXSIZE-2003 production clone test results show success
- Confirm IDXSIZE-1002 rollback script was tested on empty database

### Backup Creation and Verification
- Create production backup with timestamp:
  ```bash
  docker exec maproom-postgres pg_dump -U maproom maproom | gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz
  ```
- Verify backup integrity:
  ```bash
  gunzip -c backup_*.sql.gz | head -100
  ```
- Check backup file size is reasonable (should be multi-MB for production data)

### Resource Verification
- Check disk space availability:
  ```bash
  df -h | grep postgres
  ```
- Calculate required space: current index size * 0.31 (31% increase from architecture analysis)
- Ensure sufficient headroom (2x required space recommended)

### Baseline Metrics Documentation
Capture baseline metrics before migration:
```sql
-- Total chunk count
SELECT COUNT(*) FROM maproom.chunks;

-- Table size
SELECT pg_size_pretty(pg_relation_size('maproom.chunks'));

-- Database size
SELECT pg_size_pretty(pg_database_size('maproom'));

-- Index sizes
SELECT
  indexname,
  pg_size_pretty(pg_relation_size(schemaname||'.'||indexname))
FROM pg_indexes
WHERE schemaname = 'maproom' AND tablename = 'chunks';
```

### Migration Execution Checklist
Create documented checklist for Step 3.2 (migration execution) including:
- Pre-migration commands
- Migration execution command
- Post-migration verification queries
- Rollback trigger conditions
- Communication plan

## Implementation Notes

This is a **manual verification ticket**. The database-engineer agent should:

1. **Document the checklist** - Provide all commands and verification steps
2. **Reference prior tickets** - Link to IDXSIZE-2004, IDXSIZE-2003, IDXSIZE-1002 completion evidence
3. **Provide backup commands** - Full commands for backup creation and verification
4. **Calculate resource requirements** - Show disk space calculations
5. **Create execution checklist** - Prepare Step 3.2 migration execution guide

Follow pre-deployment checklist from `.agents/projects/IDXSIZE_index-size-limits/planning/plan.md` Step 3.1 (lines 305-334).

**Human execution confirms readiness** - This ticket provides the checklist and commands, but human operator must execute and verify in production environment before proceeding to IDXSIZE-3002 (migration execution).

### Baseline Metrics to Capture
```sql
-- Chunk count
SELECT COUNT(*) FROM maproom.chunks;

-- Chunks without embeddings (should be 0)
SELECT COUNT(*) FROM maproom.chunks WHERE embedding IS NULL;

-- Table size
SELECT pg_size_pretty(pg_relation_size('maproom.chunks'));

-- HNSW index size
SELECT pg_size_pretty(pg_relation_size('maproom.chunks_embedding_idx'));

-- Database total size
SELECT pg_size_pretty(pg_database_size('maproom'));

-- Average embedding size
SELECT AVG(octet_length(embedding::text)) FROM maproom.chunks WHERE embedding IS NOT NULL;
```

### Critical Verification Points
- ✅ IDXSIZE-2004: All automated tests passing
- ✅ IDXSIZE-2003: Production clone test successful
- ✅ IDXSIZE-1002: Rollback script tested
- ✅ Backup created and verified
- ✅ Disk space sufficient
- ✅ Team notified

## Dependencies
- **IDXSIZE-2004** - Phase 2 testing complete and verified (must be completed)
- **IDXSIZE-2003** - Production clone test successful (must be completed)
- **IDXSIZE-1002** - Rollback script exists and tested (must be completed)

All Phase 1 and Phase 2 tickets must be complete before this ticket can proceed.

## Risk Assessment

### Risk: Backup corrupted or incomplete
- **Severity**: Critical
- **Mitigation**:
  - Verify backup by reading header (first 100 lines)
  - Check file size is reasonable for production data
  - Consider creating two backups for redundancy
  - Store backup in multiple locations

### Risk: Insufficient disk space during migration
- **Severity**: High
- **Mitigation**:
  - Calculate required space before migration (current index size * 0.31)
  - Ensure 2x headroom for safety
  - Free space if needed before proceeding
  - Monitor disk usage during migration

### Risk: Migration during high-traffic period
- **Severity**: Medium
- **Mitigation**:
  - Schedule during known low-traffic window
  - Notify team of maintenance window in advance
  - Plan for potential downtime
  - Have rollback script ready

### Risk: Incomplete testing verification
- **Severity**: High
- **Mitigation**:
  - Explicitly verify each prior ticket's completion
  - Review test outputs from IDXSIZE-2004
  - Confirm production clone test results from IDXSIZE-2003
  - Do not proceed if any tests failed or were skipped

## Files/Packages Affected
- `/workspace/.agents/projects/IDXSIZE_index-size-limits/deployment/README.md` (NEW - 352 lines, deployment directory overview)
- `/workspace/.agents/projects/IDXSIZE_index-size-limits/deployment/pre-deployment-checklist.md` (NEW - 713 lines, comprehensive pre-deployment verification)
- `/workspace/.agents/projects/IDXSIZE_index-size-limits/deployment/migration-execution-guide.md` (NEW - 862 lines, step-by-step migration execution)

**Total**: 3 files created (1,927 lines, 67KB)

**When Checklist is Executed** (future, by human operators):
- Production backup file: `backup_YYYYMMDD_HHMMSS.sql.gz`
- Baseline metrics captured
- Go/no-go approval documented

## Documentation Summary

### Created Files

**1. README.md** - Deployment Directory Overview
- Complete workflow diagram
- Document index and navigation
- Quick start instructions
- Migration details reference
- Testing summary
- Critical file locations

**2. pre-deployment-checklist.md** - Pre-Deployment Verification (713 lines)
**9 comprehensive sections**:
1. **Testing Verification** - Links to all Phase 1 (IDXSIZE-1001 to 1004) and Phase 2 (IDXSIZE-2001 to 2004) ticket completions
2. **Production Clone Testing Execution** - Manual verification that production clone test was executed using IDXSIZE-2003 documentation
3. **Production Database Backup** - Commands for timestamped backup, integrity verification, size validation
4. **Resource Verification** - Disk space checks, storage impact calculations (+31%, ~155MB), headroom verification (2x recommended)
5. **Baseline Metrics Capture** - SQL queries for chunk count, table sizes, index sizes, database size, preview distribution
6. **Migration Window Planning** - Low-traffic identification, duration estimate (5-10 min), team notification checklist
7. **Rollback Readiness** - Script location, trigger conditions, execution commands, testing verification
8. **Final Go/No-Go Checklist** - All critical verifications with approval sign-off section
9. **Appendix** - Complete SQL queries, commands, and reference information

**3. migration-execution-guide.md** - Migration Execution Procedures (862 lines)
**24 numbered execution steps**:
- **Steps 1-3**: Pre-execution verification (database access, migration file, rollback readiness)
- **Steps 4-8**: Migration execution with timing (backup verification, migration run, duration check)
- **Steps 9-14**: Post-migration verification (indexes, data integrity, storage, query performance, logs, index selection)
- **Steps 15-16**: Success criteria evaluation (6 MUST PASS, 4 SHOULD PASS)
- **Rollback Decision Tree**: 4 decision points with systematic evaluation matrix
- **Steps 17-19**: Rollback procedures (if needed)
- **Steps 20-24**: Post-deployment tasks (communication, monitoring, archival)
- **Troubleshooting Guide**: 5 common scenarios with solutions

### Key Documentation Features

✅ **Complete Testing Verification**:
- All Phase 1 tickets verified (IDXSIZE-1001 to 1004)
- All Phase 2 tickets verified (IDXSIZE-2001 to 2004)
- Production clone test confirmation required
- Links to all ticket evidence

✅ **Backup and Recovery**:
- Full backup commands with timestamping
- Integrity verification procedures
- Multiple backup recommendations
- Backup storage location guidance
- Rollback script ready and tested

✅ **Resource Planning**:
- Disk space calculation (+31% storage increase)
- Headroom verification (2x recommended)
- Storage impact monitoring
- Expected values documented

✅ **Baseline Metrics**:
- Chunk count capture
- Table and index size measurement
- Database size documentation
- Preview size distribution analysis
- Pre/post comparison preparation

✅ **Safety and Rollback**:
- Clear rollback trigger conditions
- Rollback decision matrix
- Rollback execution commands
- WARNING about large preview limitation
- Multiple safety checkpoints

✅ **Communication**:
- Team notification templates
- Maintenance window planning
- Success announcement template
- Rollback communication template
- Status update procedures

### Success Criteria (from Migration Execution Guide)

**MUST PASS (Blocking - Rollback if ANY fail)**:
1. Migration completes without errors (exit code 0)
2. Zero data loss (chunk count matches baseline)
3. Old index dropped (`idx_chunks_search_covering` ABSENT)
4. 2 new indexes created (`idx_chunks_search_small_preview` and `idx_chunks_search_basic` PRESENT)
5. Large preview queries succeed (>2704 bytes) - **CRITICAL FIX VERIFICATION**
6. Query performance within ±30% baseline

**SHOULD PASS (Investigate if fail, not blocking)**:
1. Storage increase < 40% (expected: +31%)
2. Migration duration < 10 minutes
3. No PostgreSQL errors in logs
4. Correct index selection by query planner

### Usage

**For Pre-Deployment**:
1. Start with deployment/README.md
2. Complete deployment/pre-deployment-checklist.md systematically
3. Obtain sign-off approval
4. Proceed to IDXSIZE-3002 when ready

**For Migration Execution** (IDXSIZE-3002):
1. Verify pre-deployment checklist complete
2. Follow deployment/migration-execution-guide.md step-by-step
3. Evaluate success criteria
4. Make rollback decision if needed
5. Complete post-deployment tasks

## Planning References
- `.agents/projects/IDXSIZE_index-size-limits/planning/plan.md` - Step 3.1: Pre-Deployment Checklist (lines 305-334)
- `.agents/projects/IDXSIZE_index-size-limits/planning/architecture.md` - Storage impact analysis
- `.agents/projects/IDXSIZE_index-size-limits/planning/quality-strategy.md` - Testing validation requirements
