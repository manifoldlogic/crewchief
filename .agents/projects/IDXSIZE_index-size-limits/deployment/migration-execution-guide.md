# Migration Execution Guide: Migration 0017

**Migration**: `0017_fix_index_size_limits.sql`
**Project**: IDXSIZE - Index Size Limits
**Ticket**: IDXSIZE-3002
**Purpose**: Step-by-step commands for executing production migration with verification

---

## Overview

This guide provides exact commands for executing migration 0017 in production. This migration replaces the problematic `idx_chunks_search_covering` index with a multi-index strategy that eliminates PostgreSQL's 2704-byte index entry size limit.

**PREREQUISITES**: Complete ALL items in `pre-deployment-checklist.md` before proceeding.

---

## Pre-Execution Verification

### Step 1: Verify Pre-Deployment Checklist Complete

- [ ] Pre-deployment checklist signed off
- [ ] All testing completed (30/30 automated, 17/17 performance)
- [ ] Production clone test successful
- [ ] Production backup created and verified
- [ ] Team notified
- [ ] Migration window scheduled

**If ANY item unchecked, STOP and complete pre-deployment checklist first.**

### Step 2: Confirm Migration Window

```bash
# Verify current date/time matches scheduled window
date
echo "Scheduled window: [DATE] [TIME]"
```

- [ ] Current time within scheduled window: [ ] Yes [ ] No
- [ ] Team notified migration starting: [ ] Yes [ ] No

### Step 3: Final Safety Checks

```bash
# Verify PostgreSQL is running
docker ps | grep maproom-postgres
# Expected: Container running

# Verify database accessible
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT 1;"
# Expected: Returns 1

# Verify no long-running queries
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT count(*) FROM pg_stat_activity WHERE state = 'active' AND query_start < now() - interval '5 minutes';"
# Expected: 0 (no queries running > 5 minutes)
```

- [ ] PostgreSQL running: [ ] Yes [ ] No
- [ ] Database accessible: [ ] Yes [ ] No
- [ ] No blocking queries: [ ] Yes [ ] No

**If ANY check fails, STOP and investigate before proceeding.**

---

## Migration Execution

### Step 4: Capture Pre-Migration Timestamp

```bash
# Record exact migration start time
MIGRATION_START=$(date +%Y%m%d_%H%M%S)
echo "Migration started: ${MIGRATION_START}"

# Save to log file
echo "Migration 0017 Execution Log" > /tmp/migration_0017_${MIGRATION_START}.log
echo "Started: $(date)" >> /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] Start timestamp captured: ___________________________________

### Step 5: Verify Migration File

```bash
# Verify migration file exists
ls -lh /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql

# Expected output:
# -rw-r--r-- 1 user group ~2K [date] 0017_fix_index_size_limits.sql
```

- [ ] Migration file exists: [ ] Yes [ ] No
- [ ] File size reasonable (~2KB): [ ] Yes [ ] No

### Step 6: Preview Migration SQL

```bash
# Review migration contents before execution
cat /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql

# Verify migration contains:
# - SET statement_timeout = '10min';
# - DROP INDEX IF EXISTS idx_chunks_search_covering;
# - CREATE INDEX CONCURRENTLY idx_chunks_search_small_preview
# - CREATE INDEX CONCURRENTLY idx_chunks_search_basic
# - ANALYZE maproom.chunks;
```

- [ ] Migration SQL reviewed: [ ] Yes [ ] No
- [ ] All expected statements present: [ ] Yes [ ] No

### Step 7: Execute Migration

**CRITICAL**: This is the actual migration execution. Monitor carefully.

```bash
# Execute migration with timing and output capture
echo "Executing migration 0017..."
time docker exec -i maproom-postgres psql -U maproom -d maproom \
  < /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql \
  2>&1 | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Save exit code
MIGRATION_EXIT_CODE=${PIPESTATUS[0]}
echo "Migration exit code: ${MIGRATION_EXIT_CODE}" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

**Expected output**:
```
SET
DROP INDEX
CREATE INDEX
CREATE INDEX
ANALYZE
```

**Expected timing**: 5-10 minutes (based on production clone test)

- [ ] Migration command executed: [ ] Yes [ ] No
- [ ] Migration exit code: ___________________________________
- [ ] Migration duration: ___________________________________ minutes
- [ ] Exit code is 0 (success): [ ] Yes [ ] No

### Step 8: Capture Post-Migration Timestamp

```bash
# Record migration completion time
MIGRATION_END=$(date +%Y%m%d_%H%M%S)
echo "Migration completed: ${MIGRATION_END}"
echo "Completed: $(date)" >> /tmp/migration_0017_${MIGRATION_START}.log

# Calculate duration
echo "Duration: From ${MIGRATION_START} to ${MIGRATION_END}" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] End timestamp captured: ___________________________________
- [ ] Duration reasonable (<10 minutes): [ ] Yes [ ] No

---

## Post-Migration Verification

### Step 9: Verify Index Changes

```bash
# Verify old index dropped and new indexes created
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname, pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) AS index_size
   FROM pg_indexes
   WHERE tablename = 'chunks' AND schemaname = 'maproom'
   ORDER BY indexname;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

**Expected output**:
```
                indexname                |  index_size
-----------------------------------------+--------------
 idx_chunks_search_basic                 | [size]
 idx_chunks_search_small_preview         | [size]
 [other existing indexes]                | [size]
```

**MUST NOT appear**: `idx_chunks_search_covering` (old index should be dropped)

- [ ] idx_chunks_search_covering ABSENT: [ ] Yes [ ] No
- [ ] idx_chunks_search_small_preview PRESENT: [ ] Yes [ ] No
- [ ] idx_chunks_search_basic PRESENT: [ ] Yes [ ] No

**If old index still present OR new indexes missing, migration FAILED. See rollback section.**

### Step 10: Verify Data Integrity

```bash
# Verify chunk count matches baseline
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) AS chunk_count FROM maproom.chunks;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] Chunk count: ___________________________________
- [ ] Matches baseline from pre-deployment checklist: [ ] Yes [ ] No

**If chunk count does NOT match baseline, DATA LOSS occurred. See rollback section.**

### Step 11: Verify Large Preview Support (Critical Test)

```bash
# Test that large previews (>2704 bytes) work - this is THE core fix
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) AS large_preview_count
   FROM maproom.chunks
   WHERE LENGTH(preview) > 2704;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Test query execution on large previews
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT id, LENGTH(preview) AS preview_length
   FROM maproom.chunks
   WHERE LENGTH(preview) > 2704
   LIMIT 5;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

**Expected**: Query succeeds without errors (this validates the migration fix)

- [ ] Large preview query succeeded: [ ] Yes [ ] No
- [ ] Large preview count: ___________________________________

**If large preview query FAILS, migration did NOT fix the issue. See rollback section.**

### Step 12: Verify Storage Impact

```bash
# Measure post-migration storage
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT
     pg_size_pretty(pg_total_relation_size('maproom.chunks')) AS total_size,
     pg_size_pretty(pg_relation_size('maproom.chunks')) AS table_size,
     pg_size_pretty(pg_total_relation_size('maproom.chunks') - pg_relation_size('maproom.chunks')) AS index_size;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Compare to baseline
echo "Baseline total size: [from pre-deployment checklist]" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
echo "Expected increase: +31% (+~155MB for 500MB database)" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] Post-migration total size: ___________________________________
- [ ] Baseline total size: ___________________________________
- [ ] Storage increase percentage: _____________%
- [ ] Storage increase < 40%: [ ] Yes [ ] No

**If storage increase > 50%, investigate but do NOT rollback unless other issues present.**

### Step 13: Test Query Performance

```bash
# Test small preview query (should use idx_chunks_search_small_preview)
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "EXPLAIN ANALYZE
   SELECT id, symbol_name, preview
   FROM maproom.chunks
   WHERE file_id = (SELECT id FROM maproom.files LIMIT 1)
     AND kind = 'function'
     AND LENGTH(preview) <= 2000
   LIMIT 10;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Test large preview query (should use idx_chunks_search_basic)
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "EXPLAIN ANALYZE
   SELECT id, symbol_name, LENGTH(preview) AS preview_len
   FROM maproom.chunks
   WHERE file_id = (SELECT id FROM maproom.files LIMIT 1)
     AND kind = 'function'
     AND LENGTH(preview) > 2000
   LIMIT 10;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

**Expected for small previews**:
- Index used: `idx_chunks_search_small_preview`
- Scan type: Index Only Scan
- Execution time: 5-10ms (or similar to baseline)

**Expected for large previews**:
- Index used: `idx_chunks_search_basic`
- Scan type: Index Scan
- Execution time: 15-30ms

- [ ] Small preview query uses correct index: [ ] Yes [ ] No
- [ ] Large preview query uses correct index: [ ] Yes [ ] No
- [ ] Small preview execution time: ___________________________________ ms
- [ ] Large preview execution time: ___________________________________ ms
- [ ] Performance within ±30% baseline: [ ] Yes [ ] No

### Step 14: Check PostgreSQL Logs

```bash
# Check for errors or warnings during migration
docker logs maproom-postgres --since "${MIGRATION_START}" 2>&1 | \
  grep -i "error\|warning\|fail" | \
  tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Count of errors found
ERROR_COUNT=$(docker logs maproom-postgres --since "${MIGRATION_START}" 2>&1 | grep -i "error" | wc -l)
echo "PostgreSQL errors found: ${ERROR_COUNT}" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] PostgreSQL logs checked: [ ] Yes [ ] No
- [ ] Error count: ___________________________________
- [ ] No migration-related errors: [ ] Yes [ ] No

**If errors found, review carefully. Some errors may be benign (e.g., client disconnects).**

---

## Success Criteria Evaluation

### Step 15: Evaluate Against Success Criteria

Review all verification results against defined success criteria:

#### MUST PASS (Blocking - Rollback if ANY fail)

- [ ] **Migration completed without errors**: Exit code = 0
- [ ] **Zero data loss**: Chunk count matches baseline exactly
- [ ] **Old index dropped**: idx_chunks_search_covering ABSENT
- [ ] **2 new indexes created**: idx_chunks_search_small_preview and idx_chunks_search_basic PRESENT
- [ ] **Large preview queries succeed**: Queries on preview >2704 bytes execute without errors
- [ ] **Query performance acceptable**: Within ±30% of baseline

**MUST PASS COUNT**: _____ / 6

**If ANY MUST PASS criteria fails, proceed to ROLLBACK section immediately.**

#### SHOULD PASS (Investigate but may not require rollback)

- [ ] **Storage increase < 40%**: Actual increase ____________% (expected: 31%)
- [ ] **Migration duration < 10 minutes**: Actual duration ____________ minutes
- [ ] **No PostgreSQL errors in logs**: Error count ____________
- [ ] **Correct index selection**: Query planner uses appropriate index for each query type

**SHOULD PASS COUNT**: _____ / 4

**If SHOULD PASS criteria fail, investigate but migration may still be successful.**

### Step 16: Overall Migration Status

Based on success criteria evaluation:

- [ ] **Migration Status**: [ ] SUCCESS [ ] FAILURE [ ] PARTIAL

**If SUCCESS**:
- All MUST PASS criteria met
- Proceed to Post-Deployment Tasks section

**If FAILURE or PARTIAL**:
- Proceed to Rollback Decision Tree section

---

## Rollback Decision Tree

**Use this decision tree ONLY if migration did not fully succeed.**

### Decision Point 1: Is there data loss?

```bash
# Compare chunk count to baseline
BASELINE_COUNT=[from pre-deployment checklist]
CURRENT_COUNT=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks;")

if [ "${CURRENT_COUNT}" -ne "${BASELINE_COUNT}" ]; then
    echo "❌ DATA LOSS DETECTED: Baseline ${BASELINE_COUNT}, Current ${CURRENT_COUNT}"
    echo "RECOMMENDATION: ROLLBACK IMMEDIATELY"
else
    echo "✅ Data integrity confirmed"
fi
```

**If data loss detected**: **ROLLBACK IMMEDIATELY** (see Step 17)

**If no data loss**: Proceed to Decision Point 2

### Decision Point 2: Does large preview query work?

```bash
# Test core fix - large preview query
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) FROM maproom.chunks WHERE LENGTH(preview) > 2704;"

if [ $? -eq 0 ]; then
    echo "✅ Large preview query works"
else
    echo "❌ Large preview query FAILED - core fix not working"
    echo "RECOMMENDATION: ROLLBACK"
fi
```

**If large preview query fails**: **ROLLBACK** (migration did not fix the issue)

**If large preview query works**: Proceed to Decision Point 3

### Decision Point 3: Is performance acceptable?

```bash
# Check if performance degraded significantly
# Compare to baseline from pre-deployment checklist
# If > 50% slower, consider rollback
# If 30-50% slower, investigate but may keep
# If < 30% difference, acceptable
```

**If performance >50% worse**: Consider rollback (consult with team)

**If performance 30-50% worse**: Investigate but likely acceptable (partial index coverage may explain)

**If performance <30% difference**: **KEEP** (acceptable variation)

### Decision Point 4: Are indexes correct?

```bash
# Verify index state
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname FROM pg_indexes WHERE tablename = 'chunks' AND schemaname = 'maproom' ORDER BY indexname;"

# Expected: idx_chunks_search_basic, idx_chunks_search_small_preview
# NOT expected: idx_chunks_search_covering
```

**If old index still present**: Migration did not complete properly - **ROLLBACK**

**If new indexes missing**: Migration failed - **ROLLBACK**

**If indexes correct**: Likely safe to keep (review other criteria)

### Rollback Recommendation Matrix

| Data Loss | Large Preview Works | Performance OK | Indexes Correct | Recommendation |
|-----------|---------------------|----------------|-----------------|----------------|
| YES       | -                   | -              | -               | **ROLLBACK**   |
| NO        | NO                  | -              | -               | **ROLLBACK**   |
| NO        | YES                 | NO (>50%)      | YES             | **CONSULT**    |
| NO        | YES                 | YES            | NO              | **ROLLBACK**   |
| NO        | YES                 | YES            | YES             | **KEEP**       |

---

## Rollback Procedure

**ONLY execute if rollback decision made in decision tree above.**

### Step 17: Execute Rollback

**WARNING**: Rollback will FAIL if database contains large previews (>2704 bytes). If rollback fails, database is in migrated state and cannot be reverted.

```bash
# Record rollback start time
ROLLBACK_START=$(date +%Y%m%d_%H%M%S)
echo "Rollback started: ${ROLLBACK_START}" | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Execute rollback script
echo "Executing rollback..."
time docker exec -i maproom-postgres psql -U maproom -d maproom \
  < /workspace/crates/maproom/migrations/rollback/0017_rollback.sql \
  2>&1 | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Save exit code
ROLLBACK_EXIT_CODE=${PIPESTATUS[0]}
echo "Rollback exit code: ${ROLLBACK_EXIT_CODE}" | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] Rollback executed: [ ] Yes [ ] No
- [ ] Rollback exit code: ___________________________________
- [ ] Exit code is 0 (success): [ ] Yes [ ] No

### Step 18: Verify Rollback Success

```bash
# Verify old index restored
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname FROM pg_indexes WHERE tablename = 'chunks' AND schemaname = 'maproom' ORDER BY indexname;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Expected: idx_chunks_search_covering (old index restored)
# NOT expected: idx_chunks_search_small_preview, idx_chunks_search_basic

# Verify data integrity
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) FROM maproom.chunks;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log

# Test basic query
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT id FROM maproom.chunks LIMIT 5;" \
  | tee -a /tmp/migration_0017_${MIGRATION_START}.log
```

- [ ] Old index (idx_chunks_search_covering) restored: [ ] Yes [ ] No
- [ ] New indexes removed: [ ] Yes [ ] No
- [ ] Chunk count matches baseline: [ ] Yes [ ] No
- [ ] Basic queries work: [ ] Yes [ ] No

### Step 19: Rollback Communication

```bash
# Send rollback notification to team
cat > /tmp/rollback_notification.txt <<EOF
Subject: [ROLLBACK] Maproom Migration 0017 Rolled Back - $(date)

Team,

The maproom database migration 0017 has been rolled back.

Reason: [DESCRIBE REASON FROM DECISION TREE]

Timeline:
  Migration started: ${MIGRATION_START}
  Migration completed: ${MIGRATION_END}
  Rollback executed: ${ROLLBACK_START}
  Rollback completed: $(date)

Database Status:
  - Original index restored (idx_chunks_search_covering)
  - New indexes removed
  - Data integrity verified: $(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks;") chunks
  - Database operational

Next Steps:
  - Root cause analysis scheduled
  - Migration retry timeline: TBD
  - Issue tracking: [TICKET/ISSUE NUMBER]

Contact: [MIGRATION LEAD] for questions
EOF

cat /tmp/rollback_notification.txt
```

- [ ] Rollback notification sent: [ ] Yes [ ] No
- [ ] Team informed: [ ] Yes [ ] No

---

## Post-Deployment Tasks

**Execute ONLY if migration succeeded (no rollback).**

### Step 20: Document Migration Results

```bash
# Create migration results summary
cat > /tmp/migration_0017_results_${MIGRATION_START}.txt <<EOF
Migration 0017 Execution Results
================================

Migration Details:
  Started: ${MIGRATION_START}
  Completed: ${MIGRATION_END}
  Duration: [CALCULATE FROM TIMESTAMPS] minutes
  Exit Code: ${MIGRATION_EXIT_CODE}

Pre-Migration State:
  Chunk Count: [FROM BASELINE]
  Database Size: [FROM BASELINE]
  Indexes: idx_chunks_search_covering

Post-Migration State:
  Chunk Count: $(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks;")
  Database Size: $(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT pg_size_pretty(pg_database_size('maproom'));")
  Indexes: idx_chunks_search_small_preview, idx_chunks_search_basic

Success Criteria:
  Migration completed without errors: [YES/NO]
  Zero data loss: [YES/NO]
  Old index dropped: [YES/NO]
  2 new indexes created: [YES/NO]
  Large preview queries succeed: [YES/NO]
  Query performance acceptable: [YES/NO]
  Storage increase < 40%: [YES/NO] ([ACTUAL]%)
  Migration duration < 10 minutes: [YES/NO] ([ACTUAL] minutes)
  No PostgreSQL errors: [YES/NO] ([ACTUAL] errors)

Overall Status: [SUCCESS/FAILURE/PARTIAL]

Notes:
  [ANY OBSERVATIONS OR ISSUES]

Approved by: ___________________________
Date: ___________________________
EOF

cat /tmp/migration_0017_results_${MIGRATION_START}.txt
```

- [ ] Migration results documented: [ ] Yes [ ] No
- [ ] Results file saved: [ ] Yes [ ] No

### Step 21: Success Notification

```bash
# Send success notification to team
cat > /tmp/success_notification.txt <<EOF
Subject: [SUCCESS] Maproom Migration 0017 Completed - $(date)

Team,

The maproom database migration 0017 has been successfully completed.

Timeline:
  Started: ${MIGRATION_START}
  Completed: ${MIGRATION_END}
  Duration: [DURATION] minutes

Changes Applied:
  - Dropped: idx_chunks_search_covering (problematic index)
  - Created: idx_chunks_search_small_preview (partial covering index)
  - Created: idx_chunks_search_basic (universal fallback index)

Results:
  - Data integrity: ✅ Zero data loss confirmed
  - Storage impact: +[ACTUAL]% (expected: +31%)
  - Performance: ✅ Within acceptable range
  - Large preview support: ✅ Now works for 100% of chunks

Impact:
  - Fixed: PostgreSQL B-tree index size limit errors
  - Benefit: Can now index codebases with large preview fields (>2704 bytes)
  - Performance: Small previews 5-10ms, large previews 15-30ms

Database is operational. No application changes required.

Full results: /tmp/migration_0017_results_${MIGRATION_START}.txt

Thank you for your support during this migration.

[MIGRATION LEAD NAME]
EOF

cat /tmp/success_notification.txt
```

- [ ] Success notification sent: [ ] Yes [ ] No
- [ ] Team informed: [ ] Yes [ ] No

### Step 22: Archive Migration Artifacts

```bash
# Create archive directory
mkdir -p /var/backups/maproom/migration-0017/artifacts

# Copy all migration artifacts to archive
cp /tmp/migration_0017_${MIGRATION_START}.log /var/backups/maproom/migration-0017/artifacts/
cp /tmp/migration_0017_results_${MIGRATION_START}.txt /var/backups/maproom/migration-0017/artifacts/
cp /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql /var/backups/maproom/migration-0017/artifacts/
cp /workspace/crates/maproom/migrations/rollback/0017_rollback.sql /var/backups/maproom/migration-0017/artifacts/

# Create archive summary
cat > /var/backups/maproom/migration-0017/artifacts/README.txt <<EOF
Migration 0017 Archive
======================

This directory contains all artifacts from migration 0017 execution.

Files:
  - migration_0017_${MIGRATION_START}.log: Full execution log
  - migration_0017_results_${MIGRATION_START}.txt: Results summary
  - 0017_fix_index_size_limits.sql: Migration SQL
  - 0017_rollback.sql: Rollback SQL (if needed)

Migration executed: ${MIGRATION_START}
Migration status: [SUCCESS/FAILURE]
EOF

echo "Migration artifacts archived to: /var/backups/maproom/migration-0017/artifacts/"
```

- [ ] Migration artifacts archived: [ ] Yes [ ] No
- [ ] Archive location: /var/backups/maproom/migration-0017/artifacts/

### Step 23: Update Project Tracking

- [ ] IDXSIZE-3002 ticket marked complete
- [ ] Migration log updated in `.agents/projects/IDXSIZE_index-size-limits/`
- [ ] Production deployment status documented
- [ ] Next ticket (IDXSIZE-3003: Post-deployment monitoring) ready to start

### Step 24: Schedule Post-Deployment Monitoring

- [ ] Monitoring task scheduled (IDXSIZE-3003)
- [ ] Monitoring period: 7 days post-migration
- [ ] Monitoring metrics defined:
  - Query performance trends
  - Storage growth rate
  - Index usage statistics
  - Error rates
- [ ] Monitoring owner assigned: ___________________________________

---

## Troubleshooting

### Issue: Migration hangs or takes > 15 minutes

**Symptoms**: Migration does not complete within expected 5-10 minute window

**Diagnosis**:
```bash
# Check PostgreSQL activity
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT pid, state, query_start, query FROM pg_stat_activity WHERE state = 'active';"

# Check for blocking locks
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT * FROM pg_locks WHERE NOT granted;"
```

**Resolution**:
- If CONCURRENTLY is waiting for locks, wait (expected with concurrent operations)
- If truly hung (no progress > 20 minutes), consider canceling and rollback
- Review PostgreSQL logs for specific errors

### Issue: Migration fails with "index already exists"

**Symptoms**: Error message "relation 'idx_chunks_search_small_preview' already exists"

**Diagnosis**:
```bash
# Check existing indexes
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname FROM pg_indexes WHERE tablename = 'chunks' AND schemaname = 'maproom';"
```

**Resolution**:
- If new indexes already exist, migration may have been partially applied
- Manually drop new indexes and re-run migration:
  ```bash
  docker exec maproom-postgres psql -U maproom -d maproom -c \
    "DROP INDEX IF EXISTS maproom.idx_chunks_search_small_preview;"
  docker exec maproom-postgres psql -U maproom -d maproom -c \
    "DROP INDEX IF EXISTS maproom.idx_chunks_search_basic;"
  # Then re-run migration
  ```

### Issue: Rollback fails with "index too large"

**Symptoms**: Rollback fails when trying to recreate idx_chunks_search_covering

**Diagnosis**:
```bash
# Check for large previews
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT COUNT(*) FROM maproom.chunks WHERE LENGTH(preview) > 2704;"
```

**Resolution**:
- If large previews exist, rollback is NOT POSSIBLE
- Database must remain in migrated state (forward-only migration)
- This is expected behavior - large previews are incompatible with old index
- Document that rollback was attempted but failed due to data constraints

### Issue: Query performance worse than expected

**Symptoms**: Queries slower than baseline by >30%

**Diagnosis**:
```bash
# Check index usage statistics
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexrelname, idx_scan, idx_tup_read, idx_tup_fetch
   FROM pg_stat_user_indexes
   WHERE schemaname = 'maproom' AND relname = 'chunks';"

# Verify ANALYZE was run
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT last_analyze, last_autoanalyze FROM pg_stat_user_tables WHERE schemaname = 'maproom' AND relname = 'chunks';"
```

**Resolution**:
- If ANALYZE not recent, run manually:
  ```bash
  docker exec maproom-postgres psql -U maproom -d maproom -c "ANALYZE maproom.chunks;"
  ```
- Allow query planner to warm up (statistics need time to stabilize)
- If performance does not improve after 1 hour, investigate query plans

### Issue: Storage increase > 50%

**Symptoms**: Database size increased more than expected

**Diagnosis**:
```bash
# Check index sizes individually
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT indexname, pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) AS size
   FROM pg_indexes
   WHERE tablename = 'chunks' AND schemaname = 'maproom'
   ORDER BY pg_relation_size(schemaname||'.'||indexname) DESC;"
```

**Resolution**:
- Expected: Two indexes larger than original single index (normal for multi-index strategy)
- If storage critical, consider:
  - VACUUM FULL to reclaim space
  - Increased monitoring to track growth
  - Storage provisioning if needed
- Do NOT rollback solely due to storage (unless critically constrained)

---

## Quick Reference

### Critical File Locations

- **Migration SQL**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Rollback SQL**: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`
- **Migration log**: `/tmp/migration_0017_${MIGRATION_START}.log`
- **Results summary**: `/tmp/migration_0017_results_${MIGRATION_START}.txt`
- **Archive location**: `/var/backups/maproom/migration-0017/artifacts/`

### Success Criteria Summary

**MUST PASS**:
1. Migration completes without errors (exit code 0)
2. Zero data loss (chunk count matches)
3. Old index dropped
4. 2 new indexes created
5. Large preview queries succeed (>2704 bytes)
6. Query performance within ±30% baseline

**SHOULD PASS**:
1. Storage increase < 40% (expected: +31%)
2. Migration duration < 10 minutes
3. No PostgreSQL errors in logs
4. Correct index selection by query planner

### Rollback Quick Command

```bash
# ONLY IF ROLLBACK NECESSARY
docker exec -i maproom-postgres psql -U maproom -d maproom < \
  /workspace/crates/maproom/migrations/rollback/0017_rollback.sql
```

**WARNING**: Rollback fails if large previews (>2704 bytes) exist in database.

---

## Document Information

**Document Version**: 1.0
**Last Updated**: 2025-11-09
**Maintained By**: IDXSIZE Project Team
**Related Documents**:
- `pre-deployment-checklist.md` (prerequisite verification)
- `/workspace/.agents/projects/IDXSIZE_index-size-limits/testing/production-clone-test-procedure.md`
- IDXSIZE-3002 ticket (production migration execution)
- IDXSIZE-3003 ticket (post-deployment monitoring)

**Support**: Contact IDXSIZE project lead for assistance during migration execution.
