# Pre-Deployment Checklist: Migration 0017

**Migration**: `0017_fix_index_size_limits.sql`
**Project**: IDXSIZE - Index Size Limits
**Ticket**: IDXSIZE-3001
**Purpose**: Systematic verification that all testing is complete and production migration is ready

---

## Overview

This checklist ensures complete readiness verification before executing production migration 0017. This is a MANUAL verification document - each item must be checked and confirmed by a human before proceeding to production deployment.

**DO NOT PROCEED** to migration execution (IDXSIZE-3002) until ALL items in this checklist are verified and signed off.

---

## 1. Testing Verification

Verify that all Phase 1 and Phase 2 testing tickets are complete with documented evidence.

### Phase 1 - Migration Development (Tickets IDXSIZE-1001 to IDXSIZE-1004)

- [ ] **IDXSIZE-1001: Migration SQL Created**
  - Location: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
  - Verification command: `ls -lh /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
  - Expected: File exists, ~2KB size
  - Ticket status: COMPLETED
  - Evidence: Migration file contains DROP + 2 CREATE INDEX statements

- [ ] **IDXSIZE-1002: Rollback Script Created**
  - Location: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`
  - Verification command: `ls -lh /workspace/crates/maproom/migrations/rollback/0017_rollback.sql`
  - Expected: File exists, contains DROP + CREATE statements
  - Ticket status: COMPLETED
  - Evidence: Rollback script reverses migration changes
  - **WARNING**: Rollback may fail on databases with large previews (>2704 bytes)

- [ ] **IDXSIZE-1003: Documentation Updated**
  - Location: `/workspace/docs/architecture/DATABASE_ARCHITECTURE.md`
  - Verification: Documentation describes new multi-index strategy
  - Ticket status: COMPLETED
  - Evidence: Architecture docs explain idx_chunks_search_small_preview and idx_chunks_search_basic

- [ ] **IDXSIZE-1004: Phase 1 Validation**
  - Verification: All Phase 1 deliverables exist and are valid
  - Ticket status: COMPLETED
  - Evidence: Migration SQL, rollback script, and documentation all validated

### Phase 2 - Testing Validation (Tickets IDXSIZE-2001 to IDXSIZE-2004)

- [ ] **IDXSIZE-2001: Automated Test Suite (30/30 tests passed)**
  - Test script: `/workspace/crates/maproom/tests/test_index_migration.sh`
  - Verification command: `bash /workspace/crates/maproom/tests/test_index_migration.sh`
  - Expected result: Exit code 0, "30/30 TESTS PASSED"
  - Test breakdown:
    - L1 (SQL Syntax): 9/9 passed
    - L2 (Empty Database): 12/12 passed
    - L3 (Data Population): 9/9 passed
  - **CRITICAL**: L3 confirms large preview (3000 bytes) INSERT succeeds
  - Ticket status: COMPLETED
  - Last run: 2025-11-09

- [ ] **IDXSIZE-2002: Query Performance Tests (17/17 tests passed)**
  - Test script: `/workspace/crates/maproom/tests/test_query_performance.sh`
  - Verification command: `bash /workspace/crates/maproom/tests/test_query_performance.sh`
  - Expected result: Exit code 0, "17/17 TESTS PASSED"
  - Performance verified:
    - Small previews: 0.037ms (target: <20ms)
    - Large previews: 15-30ms (target: <50ms)
    - Index usage: Correct index selection for each scenario
  - Ticket status: COMPLETED
  - Last run: 2025-11-09

- [ ] **IDXSIZE-2003: Production Clone Test Documentation**
  - Documentation location: `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/`
  - Files verified:
    - `production-clone-test-procedure.md` (1,035 lines)
    - `test-execution-checklist.md` (240 lines)
    - `test-results-template.txt` (367 lines)
    - `README.md` (115 lines)
    - `INDEX.md` (211 lines)
  - Ticket status: COMPLETED
  - Evidence: Complete step-by-step procedure documented

- [ ] **IDXSIZE-2004: Phase 2 Validation**
  - Verification: All Phase 2 tests executed with documented evidence
  - Ticket status: COMPLETED
  - Evidence: Test execution results captured in ticket

---

## 2. Production Clone Testing Execution

**CRITICAL**: Production clone testing must be executed manually before production migration.

### Production Clone Test Status

- [ ] **Production Clone Test Executed**
  - Procedure: `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/production-clone-test-procedure.md`
  - Test environment: Isolated Docker PostgreSQL instance with production data clone
  - Expected duration: 30-60 minutes

- [ ] **Test Results Documented**
  - Results file created from template
  - Location: `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/migration_test_results_[DATE].txt`
  - All sections completed:
    - Pre-migration baseline measurements
    - Migration execution timing
    - Post-migration validation
    - Query performance results
    - Index usage statistics

- [ ] **Success Criteria Met**
  - Migration completed without errors
  - Zero data loss (chunk count matches)
  - Old index dropped, 2 new indexes created
  - Large preview queries succeed (core fix verified)
  - Query performance within ±30% baseline
  - Storage increase < 40% (expected: +31%)
  - Migration duration < 10 minutes
  - No PostgreSQL errors in logs

- [ ] **Test Results Reviewed and Approved**
  - Approver name: ___________________________________
  - Approval date: ___________________________________
  - Notes: ___________________________________________

**NOTE**: If production clone test has not been executed, DO NOT PROCEED. Execute the test following the procedure in `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/production-clone-test-procedure.md` before continuing.

---

## 3. Production Database Backup

**CRITICAL**: Create full backup before migration execution.

### Pre-Migration Backup Commands

```bash
# Set production connection details
export PROD_HOST="maproom-postgres"
export PROD_PORT="5432"
export PROD_DB="maproom"
export PROD_USER="maproom"

# Create backup directory
mkdir -p /var/backups/maproom/migration-0017
cd /var/backups/maproom/migration-0017

# Create timestamped backup
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="maproom_pre_migration_0017_${TIMESTAMP}.sql"

echo "Creating production backup: ${BACKUP_FILE}"
docker exec maproom-postgres pg_dump \
  -U ${PROD_USER} \
  -d ${PROD_DB} \
  --verbose \
  --no-owner \
  --no-privileges \
  > ${BACKUP_FILE}

# Verify backup created successfully
if [ -f "${BACKUP_FILE}" ] && [ -s "${BACKUP_FILE}" ]; then
    BACKUP_SIZE=$(ls -lh ${BACKUP_FILE} | awk '{print $5}')
    echo "✅ Backup created successfully: ${BACKUP_SIZE}"
else
    echo "❌ Backup failed - file empty or missing"
    exit 1
fi
```

### Backup Verification Checklist

- [ ] **Backup File Created**
  - Backup file path: ___________________________________
  - Backup timestamp: ___________________________________
  - Backup file size: ___________________________________
  - Expected size: 50MB-500MB (depending on indexed data)

- [ ] **Backup Integrity Verified**
  ```bash
  # Verify backup contains completion marker
  if tail -n 5 ${BACKUP_FILE} | grep -q "PostgreSQL database dump complete"; then
      echo "✅ Backup appears complete"
  else
      echo "❌ Warning: Backup may be incomplete"
  fi
  ```
  - Result: [ ] Complete [ ] Incomplete

- [ ] **Backup File Size Reasonable**
  ```bash
  # Check backup is non-empty and reasonable size
  BACKUP_BYTES=$(stat -f%z ${BACKUP_FILE} 2>/dev/null || stat -c%s ${BACKUP_FILE})
  if [ ${BACKUP_BYTES} -gt 1000000 ]; then
      echo "✅ Backup size reasonable: $(echo ${BACKUP_BYTES} | numfmt --to=iec)"
  else
      echo "❌ Warning: Backup suspiciously small"
  fi
  ```
  - Backup size in bytes: ___________________________________

- [ ] **Backup Storage Location Confirmed**
  - Primary backup location: /var/backups/maproom/migration-0017/
  - Primary backup accessible: [ ] Yes [ ] No
  - Secondary backup location (recommended): ___________________________________

- [ ] **Multiple Backups Created (Recommended)**
  ```bash
  # Create second backup in different location
  cp ${BACKUP_FILE} /path/to/secondary/storage/

  # Create compressed backup for archival
  gzip -c ${BACKUP_FILE} > ${BACKUP_FILE}.gz
  ```
  - Secondary backup created: [ ] Yes [ ] No
  - Compressed backup created: [ ] Yes [ ] No

### Backup Quick Validation

```bash
# Quick sanity check - verify backup can be parsed
head -n 20 ${BACKUP_FILE}
# Should show PostgreSQL dump header

# Verify backup contains maproom.chunks table
grep -c "CREATE TABLE maproom.chunks" ${BACKUP_FILE}
# Should return: 1

# Verify backup contains data
grep -c "COPY maproom.chunks" ${BACKUP_FILE}
# Should return: 1 if data exists
```

- [ ] **Backup Header Valid**: PostgreSQL dump header present
- [ ] **Backup Contains Schema**: maproom.chunks table definition present
- [ ] **Backup Contains Data**: COPY statements present (if database has data)

---

## 4. Resource Verification

Verify sufficient disk space and system resources for migration.

### Disk Space Availability

```bash
# Check available disk space
df -h /var/lib/postgresql/data

# Expected output format:
# Filesystem      Size  Used Avail Use% Mounted on
# /dev/sda1       100G   50G   50G  50% /var/lib/postgresql/data
```

- [ ] **Current Database Size Captured**
  ```bash
  docker exec maproom-postgres psql -U maproom -d maproom -c \
    "SELECT pg_size_pretty(pg_database_size('maproom')) AS db_size;"
  ```
  - Current size: ___________________________________

- [ ] **Available Disk Space Verified**
  - Available space: ___________________________________
  - Minimum required: Current DB size × 1.5 (for +31% growth + buffer)
  - **Recommended**: Current DB size × 2.0 (provides safety margin)
  - Sufficient space: [ ] Yes [ ] No

### Storage Impact Calculation

Based on architecture analysis and testing:

- **Expected storage increase**: +31% (+~155MB for typical 500MB database)
- **Calculation**:
  ```
  Current DB size:     _____________ MB
  Expected increase:   × 1.31
  New DB size:         _____________ MB
  Additional space:    _____________ MB

  Available space:     _____________ MB
  After migration:     _____________ MB (should be > 20% free)
  ```

- [ ] **Storage Impact Acceptable**
  - Expected new size: ___________________________________
  - Remaining free space: ___________________________________
  - Free space percentage: _____________%
  - **WARNING**: If free space < 20%, investigate or provision more storage

### PostgreSQL Resource Check

```bash
# Check PostgreSQL is running and responsive
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT version();"

# Check connection count
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT count(*) FROM pg_stat_activity WHERE datname = 'maproom';"

# Check for long-running queries
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT pid, age(clock_timestamp(), query_start), query
   FROM pg_stat_activity
   WHERE state = 'active' AND query NOT ILIKE '%pg_stat_activity%';"
```

- [ ] **PostgreSQL Responsive**: Version query succeeds
- [ ] **Connection Count Reasonable**: < 80% of max_connections
- [ ] **No Long-Running Queries**: No queries running > 5 minutes
- [ ] **PostgreSQL Logs Clean**: No recent errors or warnings

---

## 5. Baseline Metrics Capture

Capture current database state for comparison after migration.

### Chunk Count Baseline

```bash
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) AS chunk_count FROM maproom.chunks;"
```

- [ ] **Chunk count captured**: ___________________________________

### Table Size Baseline

```bash
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT
     pg_size_pretty(pg_total_relation_size('maproom.chunks')) AS total_size,
     pg_size_pretty(pg_relation_size('maproom.chunks')) AS table_size,
     pg_size_pretty(pg_total_relation_size('maproom.chunks') - pg_relation_size('maproom.chunks')) AS index_size;"
```

- [ ] **Total relation size**: ___________________________________
- [ ] **Table size**: ___________________________________
- [ ] **Current index size**: ___________________________________

### Current Index Information

```bash
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT
     indexname,
     pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) AS index_size
   FROM pg_indexes
   WHERE tablename = 'chunks' AND schemaname = 'maproom'
   ORDER BY indexname;"
```

- [ ] **Current indexes documented**:
  - Index name: idx_chunks_search_covering
  - Index size: ___________________________________
  - Other indexes: ___________________________________

### Database Size Baseline

```bash
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT pg_size_pretty(pg_database_size('maproom')) AS database_size;"
```

- [ ] **Total database size**: ___________________________________

### Preview Size Distribution

```bash
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT
     COUNT(*) FILTER (WHERE LENGTH(preview) <= 2000) AS small_previews,
     COUNT(*) FILTER (WHERE LENGTH(preview) > 2000 AND LENGTH(preview) <= 2704) AS medium_previews,
     COUNT(*) FILTER (WHERE LENGTH(preview) > 2704) AS large_previews,
     COUNT(*) AS total_chunks,
     ROUND(100.0 * COUNT(*) FILTER (WHERE LENGTH(preview) <= 2000) / COUNT(*), 1) AS small_pct
   FROM maproom.chunks;"
```

- [ ] **Preview distribution captured**:
  - Small previews (≤2000 bytes): ___________________________________
  - Medium previews (2001-2704 bytes): ___________________________________
  - Large previews (>2704 bytes): ___________________________________
  - Total chunks: ___________________________________
  - Small preview percentage: _____________%

**Expected**: ~95% of chunks should have small previews (≤2000 bytes)

### Baseline Metrics Summary

- [ ] **All baseline metrics captured and documented**
- [ ] **Metrics saved for post-migration comparison**
- [ ] **Baseline file created**: `/var/backups/maproom/migration-0017/baseline_metrics_${TIMESTAMP}.txt`

```bash
# Save all baseline metrics to file
cat > /var/backups/maproom/migration-0017/baseline_metrics_${TIMESTAMP}.txt <<EOF
Baseline Metrics - Migration 0017
Captured: ${TIMESTAMP}

Chunk Count: [value]
Total Relation Size: [value]
Table Size: [value]
Index Size: [value]
Database Size: [value]

Preview Distribution:
  Small (≤2000): [count] ([pct]%)
  Medium (2001-2704): [count]
  Large (>2704): [count]

Current Indexes:
  idx_chunks_search_covering: [size]
  [other indexes]
EOF
```

---

## 6. Migration Window Planning

Plan the migration execution window and stakeholder communication.

### Low-Traffic Period Identification

- [ ] **Migration Window Scheduled**
  - Scheduled date: ___________________________________
  - Scheduled time: ___________________________________
  - Scheduled timezone: ___________________________________
  - Duration estimate: 5-10 minutes (CONCURRENTLY allows concurrent queries)
  - Low-traffic period confirmed: [ ] Yes [ ] No

- [ ] **Maintenance Window Duration Estimate**
  - Based on production clone test: _____________ minutes
  - Safety buffer added: _____________ minutes
  - Total window: _____________ minutes
  - **Note**: CONCURRENTLY allows queries during migration, but plan for brief disruption

### Team Notification

- [ ] **Stakeholders Identified**
  - Engineering team: ___________________________________
  - Operations team: ___________________________________
  - Product team: ___________________________________
  - Other stakeholders: ___________________________________

- [ ] **Pre-Migration Communication Sent**
  - Communication sent date: ___________________________________
  - Advance notice: _____________ hours/days
  - Communication includes:
    - [ ] Migration purpose and benefits
    - [ ] Scheduled maintenance window
    - [ ] Expected impact (minimal, CONCURRENTLY)
    - [ ] Rollback plan available if needed
    - [ ] Contact information for issues

- [ ] **Migration Monitoring Plan**
  - Point person during migration: ___________________________________
  - Backup contact: ___________________________________
  - Communication channel: ___________________________________
  - Escalation path: ___________________________________

### Communication Template

```
Subject: [SCHEDULED] Maproom Database Migration 0017 - [DATE] [TIME]

Team,

We will be performing a database migration on the maproom database to fix
PostgreSQL index size limit errors that affect large codebases.

Schedule:
  Date: [DATE]
  Time: [TIME] [TIMEZONE]
  Duration: ~10 minutes

Impact:
  - Migration uses CONCURRENTLY - queries will continue during migration
  - Minimal performance impact expected
  - Rollback plan available if issues occur

Changes:
  - Fixes: PostgreSQL B-tree index size limit errors
  - Replaces 1 problematic index with 2 optimized indexes
  - Storage increase: +31% (+~155MB typical)
  - Performance: No degradation expected

Contact:
  Primary: [NAME] ([EMAIL/SLACK])
  Backup: [NAME] ([EMAIL/SLACK])

This migration has been thoroughly tested on production clone with all
tests passing (30/30 automated, 17/17 performance).

[MIGRATION LEAD NAME]
```

---

## 7. Rollback Readiness

Ensure rollback plan is prepared and accessible if migration encounters issues.

### Rollback Script Verification

- [ ] **Rollback Script Location Confirmed**
  - Location: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`
  - Script accessible: [ ] Yes [ ] No
  - Script reviewed: [ ] Yes [ ] No

- [ ] **Rollback Script Contents Verified**
  ```bash
  cat /workspace/crates/maproom/migrations/rollback/0017_rollback.sql
  ```
  - Contains DROP for idx_chunks_search_small_preview: [ ] Yes
  - Contains DROP for idx_chunks_search_basic: [ ] Yes
  - Contains CREATE for idx_chunks_search_covering: [ ] Yes
  - Contains WARNING about large preview limitation: [ ] Yes

### Rollback Trigger Conditions

**ROLLBACK IF**:
- Migration fails with errors
- Post-migration validation shows data loss
- Query performance degrades significantly (>50% slower)
- PostgreSQL errors appear in logs
- Storage increase exceeds 50% (vs expected 31%)
- Application errors related to chunk queries appear

**DO NOT ROLLBACK IF**:
- Database contains large previews (>2704 bytes) - rollback will fail
- Migration completed successfully but minor performance variation observed
- Storage increase within expected range (+31%)

- [ ] **Rollback Trigger Conditions Understood**
- [ ] **Rollback Decision Tree Documented** (see migration-execution-guide.md)

### Rollback Execution Commands

```bash
# ONLY USE IF ROLLBACK IS NECESSARY

# 1. Verify database state
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname FROM pg_indexes WHERE tablename = 'chunks' AND schemaname = 'maproom';"

# 2. Execute rollback
docker exec -i maproom-postgres psql -U maproom -d maproom < \
  /workspace/crates/maproom/migrations/rollback/0017_rollback.sql

# 3. Verify rollback succeeded
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname FROM pg_indexes WHERE tablename = 'chunks' AND schemaname = 'maproom';"
# Should show: idx_chunks_search_covering (old index restored)

# 4. Test query execution
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) FROM maproom.chunks;"
# Should return original chunk count
```

- [ ] **Rollback Commands Accessible**
- [ ] **Rollback Commands Tested** (during production clone test)
- [ ] **Rollback Point Person Identified**: ___________________________________

### Rollback Communication Plan

- [ ] **Rollback Communication Template Prepared**
  ```
  Subject: [ROLLBACK] Maproom Migration 0017 Rolled Back - [DATE] [TIME]

  Team,

  The maproom database migration 0017 has been rolled back due to: [REASON]

  Actions Taken:
    - Migration rolled back at [TIME]
    - Original index restored
    - Database verified stable
    - Data integrity confirmed

  Next Steps:
    - [INVESTIGATION PLAN]
    - [TIMELINE FOR RETRY]

  Contact: [NAME] for questions
  ```

---

## 8. Final Go/No-Go Checklist

**DO NOT PROCEED** with migration execution until ALL items below are checked.

### Testing Completeness

- [ ] All Phase 1 tickets (IDXSIZE-1001 to 1004) completed
- [ ] All Phase 2 tickets (IDXSIZE-2001 to 2004) completed
- [ ] Automated test suite passed (30/30 tests)
- [ ] Query performance tests passed (17/17 tests)
- [ ] Production clone test executed and documented
- [ ] Production clone test approved by reviewer

### Backup and Safety

- [ ] Production backup created and verified
- [ ] Backup integrity confirmed
- [ ] Multiple backups stored in different locations
- [ ] Backup restoration tested (during production clone test)

### Resource Readiness

- [ ] Disk space sufficient (2x current DB size recommended)
- [ ] Storage impact calculated and acceptable
- [ ] PostgreSQL responsive and healthy
- [ ] No long-running queries blocking migration

### Operational Readiness

- [ ] Baseline metrics captured and documented
- [ ] Migration window scheduled in low-traffic period
- [ ] Team notifications sent with advance notice
- [ ] Rollback plan ready and accessible
- [ ] Rollback trigger conditions understood
- [ ] Point person and backup contact identified

### Documentation Readiness

- [ ] Migration execution guide reviewed (migration-execution-guide.md)
- [ ] Step-by-step commands prepared and accessible
- [ ] Success criteria documented and understood
- [ ] Post-migration validation queries prepared

---

## 9. Sign-Off and Approval

### Pre-Deployment Approval

**I confirm that all items in this checklist have been verified and the production migration is ready for execution.**

- [ ] **All checklist items completed**: _____ / _____ (total items)
- [ ] **All testing passed**: Phase 1 and Phase 2 complete
- [ ] **Production clone test successful**: Results documented and approved
- [ ] **Backup verified**: Multiple backups created and confirmed
- [ ] **Resources verified**: Sufficient disk space and system resources
- [ ] **Rollback ready**: Rollback plan prepared and accessible
- [ ] **Team notified**: Stakeholders informed and prepared

**Approved by**:

- Name: ___________________________________
- Role: ___________________________________
- Date: ___________________________________
- Time: ___________________________________
- Signature: ___________________________________

**Backup approver** (if primary unavailable):

- Name: ___________________________________
- Role: ___________________________________

---

## 10. Next Steps

Once this checklist is complete and signed off:

1. **Review migration execution guide**: `/workspace/.agents/projects/IDXSIZE_index-size-limits/deployment/migration-execution-guide.md`
2. **Schedule migration window**: Coordinate with team
3. **Execute migration**: Follow step-by-step guide (IDXSIZE-3002)
4. **Monitor and validate**: Post-migration verification
5. **Document results**: Capture metrics and outcomes

---

## Appendix: Quick Reference

### Key File Locations

- **Migration SQL**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Rollback SQL**: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`
- **Test Scripts**: `/workspace/crates/maproom/tests/test_index_migration.sh`
- **Test Documentation**: `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/`
- **Migration Guide**: `/workspace/.agents/projects/IDXSIZE_index-size-limits/deployment/migration-execution-guide.md`

### Expected Migration Impact

- **Storage**: +31% (+~155MB for 500MB database)
- **Duration**: 5-10 minutes (CONCURRENTLY allows queries during migration)
- **Downtime**: None (CONCURRENTLY)
- **Performance**: No degradation expected (5-10ms small, 15-30ms large)
- **Data loss**: Zero (verified in all tests)

### Critical Success Criteria

1. Migration completes without errors
2. Zero data loss (chunk count matches baseline)
3. Old index dropped, 2 new indexes created
4. Large preview queries succeed (>2704 bytes)
5. Query performance within ±30% baseline
6. Storage increase < 40%
7. No PostgreSQL errors in logs

---

**Document Version**: 1.0
**Last Updated**: 2025-11-09
**Maintained By**: IDXSIZE Project Team
**Next Document**: `migration-execution-guide.md` (step-by-step execution)
