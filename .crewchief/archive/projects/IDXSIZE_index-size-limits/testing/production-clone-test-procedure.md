# Production Clone Test Procedure: Migration 0017

**Migration**: `0017_fix_index_size_limits.sql`
**Ticket**: IDXSIZE-2003
**Test Type**: Manual Production Clone Validation
**Purpose**: Verify migration safety and performance before production deployment

## Overview

This procedure tests migration 0017 on a complete clone of the production database in an isolated environment. The migration replaces the failing `idx_chunks_search_covering` index with two new specialized indexes that handle all chunk sizes without PostgreSQL's 2704-byte index entry limit.

**Migration Changes**:
- **Drops**: 1 index (`idx_chunks_search_covering`)
- **Creates**: 2 new indexes (`idx_chunks_search_small_preview`, `idx_chunks_search_basic`)
- **Expected Storage Impact**: +31% (+~155MB based on architecture analysis)
- **Expected Query Performance**: 5-10ms (small previews), 15-30ms (large previews)

## Prerequisites

### Required Tools
- Docker (for running isolated PostgreSQL instance)
- PostgreSQL client tools (`psql`, `pg_dump`)
- Access to production maproom database
- Sufficient disk space (2x current database size + 500MB buffer)

### Required Access
- Read access to production PostgreSQL instance
- Ability to run Docker containers on local/test machine
- Network connectivity to production database

### Migration File Location
```bash
/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql
```

## Test Environment Setup

### Step 1: Create Production Database Backup

**⚠️ CRITICAL**: Perform backup during low-traffic period to ensure consistency.

```bash
# Set production connection details
export PROD_HOST="maproom-postgres"
export PROD_PORT="5432"
export PROD_DB="maproom"
export PROD_USER="maproom"

# Create backup directory
mkdir -p /tmp/migration-test-0017
cd /tmp/migration-test-0017

# Create production backup with timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="maproom_backup_${TIMESTAMP}.sql"

echo "Creating production backup: ${BACKUP_FILE}"
docker exec maproom-postgres pg_dump \
  -U ${PROD_USER} \
  -d ${PROD_DB} \
  --verbose \
  --no-owner \
  --no-privileges \
  > ${BACKUP_FILE}

# Verify backup was created successfully
if [ -f "${BACKUP_FILE}" ] && [ -s "${BACKUP_FILE}" ]; then
    BACKUP_SIZE=$(ls -lh ${BACKUP_FILE} | awk '{print $5}')
    echo "✅ Backup created successfully: ${BACKUP_SIZE}"
else
    echo "❌ Backup failed - file empty or missing"
    exit 1
fi

# Quick backup validation
if grep -q "PostgreSQL database dump complete" ${BACKUP_FILE}; then
    echo "✅ Backup appears complete"
else
    echo "⚠️  Warning: Backup may be incomplete"
fi
```

**Success Criteria**:
- ✅ Backup file exists and is non-empty
- ✅ Backup file contains "PostgreSQL database dump complete" footer
- ✅ File size reasonable (typically 50MB-500MB depending on indexed data)

**Record in Test Results**:
```
Backup timestamp: _______________
Backup file size: _______________
Production chunk count (for verification): _______________
```

### Step 2: Create Isolated Test PostgreSQL Instance

```bash
# Stop any existing test instance
docker rm -f migration-test-pg 2>/dev/null || true

# Start isolated test PostgreSQL instance
# Using same version as production (pgvector/pgvector:pg15)
docker run -d \
  --name migration-test-pg \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=testpass \
  -e POSTGRES_DB=postgres \
  -p 5433:5432 \
  pgvector/pgvector:pg15

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to start..."
for i in {1..30}; do
    if docker exec migration-test-pg pg_isready -U postgres >/dev/null 2>&1; then
        echo "✅ PostgreSQL is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ PostgreSQL failed to start within 30 seconds"
        exit 1
    fi
    sleep 1
done

# Create test database
docker exec migration-test-pg psql -U postgres -c "CREATE DATABASE maproom_test;"

# Enable pgvector extension
docker exec migration-test-pg psql -U postgres -d maproom_test -c "CREATE EXTENSION IF NOT EXISTS vector;"

echo "✅ Test PostgreSQL instance ready on port 5433"
```

**Success Criteria**:
- ✅ Container running (`docker ps | grep migration-test-pg`)
- ✅ PostgreSQL accepting connections
- ✅ `maproom_test` database exists
- ✅ pgvector extension installed

**Verification Commands**:
```bash
# Verify container is running
docker ps | grep migration-test-pg

# Verify database exists
docker exec migration-test-pg psql -U postgres -l | grep maproom_test

# Verify pgvector extension
docker exec migration-test-pg psql -U postgres -d maproom_test -c "\dx" | grep vector
```

### Step 3: Restore Backup to Test Instance

```bash
# Restore backup to test instance
echo "Restoring backup to test instance..."
cat ${BACKUP_FILE} | docker exec -i migration-test-pg \
  psql -U postgres -d maproom_test

# Check for restore errors
if [ $? -eq 0 ]; then
    echo "✅ Backup restored successfully"
else
    echo "❌ Backup restore failed"
    exit 1
fi
```

**Success Criteria**:
- ✅ Restore completes without fatal errors
- ✅ All tables exist in test database

**Verification**:
```bash
# Verify schema restored
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT schemaname, tablename
  FROM pg_tables
  WHERE schemaname = 'maproom'
  ORDER BY tablename;
"

# Expected tables: repos, worktrees, files, chunks, chunk_relationships
```

## Pre-Migration Baseline Measurements

### Step 4: Capture Baseline Metrics

**⚠️ IMPORTANT**: These measurements establish the baseline for comparison after migration.

```bash
# Create results file
RESULTS_FILE="migration_test_results_${TIMESTAMP}.txt"
echo "Migration 0017 Test Results - $(date)" > ${RESULTS_FILE}
echo "=======================================" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}
```

#### 4.1: Chunk Count (Critical - Detects Data Loss)

```bash
echo "=== PRE-MIGRATION BASELINE ===" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}

# Total chunk count
CHUNK_COUNT=$(docker exec migration-test-pg psql -U postgres -d maproom_test -t -c "
  SELECT COUNT(*) FROM maproom.chunks;
")

echo "Total chunks: ${CHUNK_COUNT}" | tee -a ${RESULTS_FILE}

# Preview size distribution
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    CASE
      WHEN LENGTH(preview) <= 2000 THEN 'Small (<=2000 bytes)'
      WHEN LENGTH(preview) <= 5000 THEN 'Medium (2001-5000 bytes)'
      ELSE 'Large (>5000 bytes)'
    END as preview_size_category,
    COUNT(*) as chunk_count,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2) as percentage
  FROM maproom.chunks
  GROUP BY preview_size_category
  ORDER BY MIN(LENGTH(preview));
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Record in Test Results**:
```
Pre-migration chunk count: _______________
Small preview chunks (<= 2000 bytes): _______________
Medium preview chunks (2001-5000 bytes): _______________
Large preview chunks (> 5000 bytes): _______________
```

#### 4.2: Current Index Inventory

```bash
echo "=== CURRENT INDEXES ===" >> ${RESULTS_FILE}
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    indexname,
    pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) as index_size
  FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname LIKE 'idx_chunks_search%'
  ORDER BY indexname;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Expected Pre-Migration Indexes**:
- `idx_chunks_search_covering` (will be dropped)
- May also see other chunk indexes (not affected by migration)

**Record in Test Results**:
```
idx_chunks_search_covering size: _______________
Other chunk indexes: _______________
```

#### 4.3: Table and Total Index Size

```bash
echo "=== STORAGE METRICS ===" >> ${RESULTS_FILE}

# Table size
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    'chunks table' as object,
    pg_size_pretty(pg_total_relation_size('maproom.chunks')) as total_size,
    pg_size_pretty(pg_relation_size('maproom.chunks')) as table_size,
    pg_size_pretty(pg_total_relation_size('maproom.chunks') - pg_relation_size('maproom.chunks')) as indexes_size
  ;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Record in Test Results**:
```
Pre-migration table size: _______________
Pre-migration total index size: _______________
Pre-migration total size (table + indexes): _______________
```

#### 4.4: Query Performance Baseline

```bash
echo "=== QUERY PERFORMANCE BASELINE ===" >> ${RESULTS_FILE}

# Enable timing
docker exec migration-test-pg psql -U postgres -d maproom_test -c "\timing on" > /dev/null

# Test Query 1: Small file search (most common query pattern)
echo "Query 1: File-based chunk search" >> ${RESULTS_FILE}
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT symbol_name, preview, kind
  FROM maproom.chunks
  WHERE file_id = (SELECT MIN(id) FROM maproom.files)
    AND kind IN ('function', 'class')
  ORDER BY start_line
  LIMIT 20;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Test Query 2: Large result set
echo "Query 2: Multi-file search" >> ${RESULTS_FILE}
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT COUNT(*), AVG(LENGTH(preview))
  FROM maproom.chunks
  WHERE file_id <= (SELECT MIN(id) + 5 FROM maproom.files)
    AND kind = 'function';
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Test Query 3: Symbol lookup
echo "Query 3: Symbol name search" >> ${RESULTS_FILE}
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT id, file_id, start_line, preview
  FROM maproom.chunks
  WHERE symbol_name LIKE 'create%'
  LIMIT 10;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Record in Test Results** (extract from EXPLAIN ANALYZE output):
```
Query 1 execution time: _______________ms
Query 1 index used: _______________

Query 2 execution time: _______________ms
Query 2 index used: _______________

Query 3 execution time: _______________ms
Query 3 index used: _______________
```

## Migration Execution

### Step 5: Apply Migration with Timing

```bash
echo "=== MIGRATION EXECUTION ===" >> ${RESULTS_FILE}
echo "Migration file: 0017_fix_index_size_limits.sql" >> ${RESULTS_FILE}
echo "Start time: $(date)" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}

# Copy migration file to container
docker cp /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql \
  migration-test-pg:/tmp/migration.sql

# Execute migration with timing
START_TIME=$(date +%s)

docker exec migration-test-pg psql -U postgres -d maproom_test \
  -f /tmp/migration.sql 2>&1 | tee -a ${RESULTS_FILE}

MIGRATION_EXIT_CODE=$?
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "" >> ${RESULTS_FILE}
echo "End time: $(date)" >> ${RESULTS_FILE}
echo "Duration: ${DURATION} seconds" >> ${RESULTS_FILE}
echo "Exit code: ${MIGRATION_EXIT_CODE}" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}

if [ ${MIGRATION_EXIT_CODE} -ne 0 ]; then
    echo "❌ Migration failed with exit code ${MIGRATION_EXIT_CODE}"
    echo "See ${RESULTS_FILE} for details"
    exit 1
else
    echo "✅ Migration completed in ${DURATION} seconds"
fi
```

**Success Criteria**:
- ✅ Migration completes without errors
- ✅ Duration < 600 seconds (10 minutes)
- ✅ No PostgreSQL errors in output

**Record in Test Results**:
```
Migration start time: _______________
Migration end time: _______________
Migration duration: _______________seconds
Exit code: _______________
```

## Post-Migration Validation

### Step 6: Verify Index Changes

#### 6.1: Verify New Indexes Created

```bash
echo "=== POST-MIGRATION INDEX VERIFICATION ===" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    indexname,
    pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) as index_size,
    obj_description((schemaname||'.'||indexname)::regclass) as comment
  FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname LIKE 'idx_chunks_search%'
  ORDER BY indexname;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Verify specific indexes
NEW_INDEX_COUNT=$(docker exec migration-test-pg psql -U postgres -d maproom_test -t -c "
  SELECT COUNT(*)
  FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname IN ('idx_chunks_search_small_preview', 'idx_chunks_search_basic');
")

if [ "${NEW_INDEX_COUNT}" -eq 2 ]; then
    echo "✅ Both new indexes created successfully" | tee -a ${RESULTS_FILE}
else
    echo "❌ Expected 2 new indexes, found ${NEW_INDEX_COUNT}" | tee -a ${RESULTS_FILE}
fi
```

**Expected Post-Migration Indexes**:
- ✅ `idx_chunks_search_small_preview` (NEW)
- ✅ `idx_chunks_search_basic` (NEW)
- ❌ `idx_chunks_search_covering` (SHOULD BE GONE)

**Record in Test Results**:
```
idx_chunks_search_small_preview: [ ] Created  Size: _______________
idx_chunks_search_basic: [ ] Created  Size: _______________
idx_chunks_search_covering: [ ] Dropped (should not exist)
```

#### 6.2: Verify Old Index Dropped

```bash
OLD_INDEX_EXISTS=$(docker exec migration-test-pg psql -U postgres -d maproom_test -t -c "
  SELECT COUNT(*)
  FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND indexname = 'idx_chunks_search_covering';
")

if [ "${OLD_INDEX_EXISTS}" -eq 0 ]; then
    echo "✅ Old index successfully dropped" | tee -a ${RESULTS_FILE}
else
    echo "❌ Old index still exists!" | tee -a ${RESULTS_FILE}
fi

echo "" >> ${RESULTS_FILE}
```

### Step 7: Verify Zero Data Loss

```bash
echo "=== DATA INTEGRITY VERIFICATION ===" >> ${RESULTS_FILE}

# Verify chunk count unchanged
POST_CHUNK_COUNT=$(docker exec migration-test-pg psql -U postgres -d maproom_test -t -c "
  SELECT COUNT(*) FROM maproom.chunks;
")

echo "Pre-migration chunks:  ${CHUNK_COUNT}" | tee -a ${RESULTS_FILE}
echo "Post-migration chunks: ${POST_CHUNK_COUNT}" | tee -a ${RESULTS_FILE}

if [ "${CHUNK_COUNT}" -eq "${POST_CHUNK_COUNT}" ]; then
    echo "✅ Zero data loss - chunk count matches" | tee -a ${RESULTS_FILE}
else
    echo "❌ Data loss detected! Count mismatch!" | tee -a ${RESULTS_FILE}
fi

echo "" >> ${RESULTS_FILE}

# Verify large preview chunks still exist
LARGE_PREVIEW_COUNT=$(docker exec migration-test-pg psql -U postgres -d maproom_test -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE LENGTH(preview) > 2000;
")

echo "Chunks with large previews (>2000 bytes): ${LARGE_PREVIEW_COUNT}" | tee -a ${RESULTS_FILE}

if [ "${LARGE_PREVIEW_COUNT}" -gt 0 ]; then
    echo "✅ Large preview chunks preserved" | tee -a ${RESULTS_FILE}
else
    echo "ℹ️  No large preview chunks in this database" | tee -a ${RESULTS_FILE}
fi

echo "" >> ${RESULTS_FILE}
```

**Success Criteria**:
- ✅ Post-migration chunk count exactly matches pre-migration count
- ✅ Large preview chunks (if any) still exist

**Record in Test Results**:
```
Pre-migration chunk count: _______________
Post-migration chunk count: _______________
Data loss: [ ] None (counts match)  [ ] DETECTED (investigate!)
Large preview chunks preserved: _______________
```

### Step 8: Storage Impact Measurement

```bash
echo "=== POST-MIGRATION STORAGE METRICS ===" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    'chunks table' as object,
    pg_size_pretty(pg_total_relation_size('maproom.chunks')) as total_size,
    pg_size_pretty(pg_relation_size('maproom.chunks')) as table_size,
    pg_size_pretty(pg_total_relation_size('maproom.chunks') - pg_relation_size('maproom.chunks')) as indexes_size
  ;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Calculate storage increase
echo "Storage increase calculation:" >> ${RESULTS_FILE}
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  WITH pre_size AS (
    -- This is approximate - using post-migration table size as baseline
    SELECT pg_relation_size('maproom.chunks') as table_bytes
  ),
  post_size AS (
    SELECT pg_total_relation_size('maproom.chunks') - pg_relation_size('maproom.chunks') as index_bytes
  )
  SELECT
    pg_size_pretty(pre_size.table_bytes) as table_size,
    pg_size_pretty(post_size.index_bytes) as total_index_size,
    pg_size_pretty(post_size.index_bytes::bigint) as index_increase,
    ROUND(100.0 * post_size.index_bytes::numeric / pre_size.table_bytes::numeric, 2) as increase_percentage
  FROM pre_size, post_size;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Expected Storage Impact**:
- Total index size increase: ~31% (~155MB based on architecture analysis)
- Acceptable range: 25-40%

**Record in Test Results**:
```
Post-migration total index size: _______________
Storage increase: _______________MB
Storage increase percentage: _______________%
Within expected range (25-40%): [ ] Yes  [ ] No
```

## Critical Path Testing

### Step 9: Test Critical Queries

#### 9.1: Query 1 - File-Based Search (Most Common)

```bash
echo "=== CRITICAL PATH QUERY TESTING ===" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}
echo "Query 1: File-based chunk search (most common pattern)" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT symbol_name, preview, kind
  FROM maproom.chunks
  WHERE file_id = (SELECT MIN(id) FROM maproom.files)
    AND kind IN ('function', 'class')
  ORDER BY start_line
  LIMIT 20;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Expected Behavior**:
- Uses `idx_chunks_search_small_preview` (if previews small) OR `idx_chunks_search_basic` (if previews large)
- Execution time: 5-20ms
- Returns correct results

**Record in Test Results**:
```
Query 1 execution time: _______________ms
Query 1 index used: _______________
Performance: [ ] Good (<20ms)  [ ] Acceptable (20-50ms)  [ ] Slow (>50ms)
```

#### 9.2: Query 2 - Large Result Set

```bash
echo "Query 2: Multi-file aggregation" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT COUNT(*), AVG(LENGTH(preview))
  FROM maproom.chunks
  WHERE file_id <= (SELECT MIN(id) + 5 FROM maproom.files)
    AND kind = 'function';
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Record in Test Results**:
```
Query 2 execution time: _______________ms
Query 2 index used: _______________
Performance: [ ] Good (<20ms)  [ ] Acceptable (20-50ms)  [ ] Slow (>50ms)
```

#### 9.3: Query 3 - Large Preview Edge Case

```bash
echo "Query 3: Large preview handling (edge case)" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT id, symbol_name, LENGTH(preview) as preview_len
  FROM maproom.chunks
  WHERE LENGTH(preview) > 2000
    AND kind = 'function'
  LIMIT 10;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Verify this query succeeds (it would fail with old index on insert)
LARGE_PREVIEW_QUERY_SUCCESS=$?
if [ ${LARGE_PREVIEW_QUERY_SUCCESS} -eq 0 ]; then
    echo "✅ Large preview query succeeded" | tee -a ${RESULTS_FILE}
else
    echo "❌ Large preview query failed!" | tee -a ${RESULTS_FILE}
fi

echo "" >> ${RESULTS_FILE}
```

**Expected Behavior**:
- Uses `idx_chunks_search_basic` (non-covering index with heap lookup)
- Execution time: 15-30ms
- **CRITICAL**: No errors (old index would fail on INSERT for large previews)

**Record in Test Results**:
```
Query 3 execution time: _______________ms
Query 3 index used: _______________
Query succeeded: [ ] Yes  [ ] No (CRITICAL FAILURE)
Performance: [ ] Good (<30ms)  [ ] Acceptable (30-50ms)  [ ] Slow (>50ms)
```

#### 9.4: Query 4 - Symbol Name Search

```bash
echo "Query 4: Symbol name search" >> ${RESULTS_FILE}

docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, TIMING)
  SELECT id, file_id, start_line, preview
  FROM maproom.chunks
  WHERE symbol_name LIKE 'create%'
  LIMIT 10;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Record in Test Results**:
```
Query 4 execution time: _______________ms
Query 4 index used: _______________
```

### Step 10: Index Usage Verification

```bash
echo "=== INDEX USAGE STATISTICS ===" >> ${RESULTS_FILE}

# Update statistics first
docker exec migration-test-pg psql -U postgres -d maproom_test -c "ANALYZE maproom.chunks;" > /dev/null

# Check index usage
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    indexname,
    idx_scan as scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched,
    CASE
      WHEN idx_scan = 0 THEN 'Not used in test queries'
      ELSE 'Used ' || idx_scan || ' times'
    END as usage_summary
  FROM pg_stat_user_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname LIKE 'idx_chunks_search%'
  ORDER BY indexname;
" | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Expected Results**:
- `idx_chunks_search_small_preview`: Used for queries with small previews
- `idx_chunks_search_basic`: Used as fallback for large previews or when covering index not beneficial
- Both indexes should show some usage if test data includes both small and large previews

**Record in Test Results**:
```
idx_chunks_search_small_preview scans: _______________
idx_chunks_search_basic scans: _______________
Both indexes used: [ ] Yes  [ ] Only one used (investigate)
```

## PostgreSQL Log Verification

### Step 11: Check PostgreSQL Logs

```bash
echo "=== POSTGRESQL LOG REVIEW ===" >> ${RESULTS_FILE}
echo "Checking for errors, warnings, and performance issues..." >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}

# Check for errors during migration
docker logs migration-test-pg 2>&1 | grep -i error | tail -20 | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Check for warnings
docker logs migration-test-pg 2>&1 | grep -i warning | tail -20 | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}

# Check for slow queries
docker logs migration-test-pg 2>&1 | grep -E "duration.*ms" | tail -20 | tee -a ${RESULTS_FILE}

echo "" >> ${RESULTS_FILE}
```

**Success Criteria**:
- ✅ No ERROR messages related to migration
- ✅ No "index row size exceeds maximum" errors
- ✅ No unexpected WARNINGS

**Record in Test Results**:
```
Errors found: [ ] None  [ ] Found (details: _______________)
Warnings found: [ ] None  [ ] Found (details: _______________)
Slow queries (>100ms): [ ] None  [ ] Found (details: _______________)
```

## Test Results Summary

### Step 12: Document Test Results

Complete the following test results template:

```bash
echo "" >> ${RESULTS_FILE}
echo "=== TEST RESULTS SUMMARY ===" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}
echo "Test Date: $(date)" >> ${RESULTS_FILE}
echo "Tester: _______________" >> ${RESULTS_FILE}
echo "Database Size: _______________" >> ${RESULTS_FILE}
echo "Total Chunks: _______________" >> ${RESULTS_FILE}
echo "" >> ${RESULTS_FILE}

cat >> ${RESULTS_FILE} << 'EOF'
VALIDATION CHECKLIST:

[ ] Pre-migration baseline captured
[ ] Migration completed without errors
[ ] Migration duration < 10 minutes
[ ] Zero data loss (chunk count matches)
[ ] Old index (idx_chunks_search_covering) dropped
[ ] New index (idx_chunks_search_small_preview) created
[ ] New index (idx_chunks_search_basic) created
[ ] Storage increase within acceptable range (25-40%)
[ ] Query 1 (file search) performance acceptable
[ ] Query 2 (aggregation) performance acceptable
[ ] Query 3 (large preview) succeeds without errors
[ ] Query 4 (symbol search) performance acceptable
[ ] Both new indexes used by query planner
[ ] No PostgreSQL errors in logs
[ ] No index size errors in logs

SUCCESS CRITERIA:

MUST PASS (Blocking):
[ ] Migration completes without errors
[ ] Zero data loss (count matches)
[ ] All 3 index changes applied correctly
[ ] Large preview INSERT/query succeeds (critical fix validation)
[ ] All critical path queries return correct results
[ ] Query performance within ±30% of baseline

SHOULD PASS (Investigate if failed):
[ ] Storage increase < 40%
[ ] Migration duration < 10 minutes
[ ] No PostgreSQL errors in logs
[ ] Both indexes show usage statistics

OVERALL RESULT: [ ] PASS  [ ] FAIL  [ ] CONDITIONAL PASS

Notes/Observations:
_______________________________________________________________________
_______________________________________________________________________
_______________________________________________________________________

Issues Found:
_______________________________________________________________________
_______________________________________________________________________
_______________________________________________________________________

Follow-up Actions Required:
_______________________________________________________________________
_______________________________________________________________________
_______________________________________________________________________
EOF

echo "" >> ${RESULTS_FILE}
echo "Test results saved to: ${RESULTS_FILE}"
cat ${RESULTS_FILE}
```

## Rollback Procedure (If Migration Fails)

**⚠️ WARNING**: Rollback is only safe if NO large preview chunks exist in the database. After migration, the old covering index cannot be recreated if large previews were added.

### Rollback Steps

If migration fails during test and needs to be rolled back:

```bash
echo "=== ROLLBACK PROCEDURE ===" | tee -a ${RESULTS_FILE}
echo "WARNING: This rollback only works if no large preview chunks exist" | tee -a ${RESULTS_FILE}
echo "" | tee -a ${RESULTS_FILE}

# Drop new indexes
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  DROP INDEX IF EXISTS maproom.idx_chunks_search_small_preview;
  DROP INDEX IF EXISTS maproom.idx_chunks_search_basic;
"

# Attempt to recreate old index (may fail if large previews exist)
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  CREATE INDEX CONCURRENTLY idx_chunks_search_covering
    ON maproom.chunks (file_id, kind, start_line)
    INCLUDE (symbol_name, preview);
"

ROLLBACK_EXIT_CODE=$?

if [ ${ROLLBACK_EXIT_CODE} -eq 0 ]; then
    echo "✅ Rollback successful" | tee -a ${RESULTS_FILE}
else
    echo "❌ Rollback failed - old index cannot be recreated" | tee -a ${RESULTS_FILE}
    echo "This is expected if large preview chunks exist" | tee -a ${RESULTS_FILE}
    echo "Recommendation: Fix migration forward, not backward" | tee -a ${RESULTS_FILE}
fi
```

**Recommendation**: **Do NOT rollback in production**. If migration fails, investigate and fix forward. The multi-index strategy is designed to be the permanent solution.

## Cleanup

### Step 13: Cleanup Test Environment

**⚠️ Only cleanup after test results are documented and reviewed!**

```bash
# Save test results before cleanup
cp ${RESULTS_FILE} /workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/

echo "Test results saved to project directory"

# Stop and remove test container
docker stop migration-test-pg
docker rm migration-test-pg

# Remove backup files (optional - may want to keep for reference)
# rm -rf /tmp/migration-test-0017

echo "✅ Cleanup complete"
```

## Expected Results Reference

### Storage Impact

Based on architecture analysis:

- **Storage increase**: ~31% (+~155MB)
- **Acceptable range**: 25-40%
- **Three indexes total** (down from 1, but includes partial + fallback):
  - `idx_chunks_search_small_preview`: ~475MB (95% of rows)
  - `idx_chunks_search_basic`: ~80MB (100% of rows, minimal)
  - **Total**: ~555MB (was ~500MB with old failing index)

### Query Performance Targets

From architecture.md:

| Query Type | Target Performance | Index Used |
|------------|-------------------|------------|
| Small preview (95%) | 5-10ms | idx_chunks_search_small_preview (index-only scan) |
| Large preview (5%) | 15-30ms | idx_chunks_search_basic (index + heap lookup) |
| Average (weighted) | ~7ms | Query planner chooses automatically |

### Migration Duration

- **Target**: < 10 minutes
- **Typical**: 2-5 minutes depending on database size
- **Concurrent index creation**: Minimal blocking, production can continue serving queries

## Success Criteria Summary

### MUST PASS (Blocking for Production)

- ✅ Migration completes without errors
- ✅ Zero data loss (chunk count exact match)
- ✅ Old index dropped, 2 new indexes created
- ✅ Large preview chunks can be queried (critical fix validation)
- ✅ All critical path queries return correct results
- ✅ Query performance within ±30% of baseline

### SHOULD PASS (Investigate but not blocking)

- Storage increase < 40%
- Migration duration < 10 minutes
- No PostgreSQL errors/warnings in logs
- Both new indexes showing usage in statistics

### NICE TO HAVE (Not blocking)

- Index-only scan rate > 90% for small previews
- Query planning time < 5ms
- Storage increase closer to 31% estimate

## Troubleshooting

### Migration Fails with "index row size exceeds maximum"

**Cause**: Migration is trying to create covering index on large preview chunks

**Solution**: This should NOT happen with migration 0017 (it's what we're fixing!). If it does:
1. Verify you're running the correct migration file (0017_fix_index_size_limits.sql)
2. Check migration SQL syntax - partial index should have `WHERE LENGTH(preview) <= 2000`

### Old Index Still Exists After Migration

**Cause**: DROP INDEX statement failed or was commented out

**Solution**:
```bash
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  DROP INDEX IF EXISTS maproom.idx_chunks_search_covering;
"
```

### New Indexes Not Being Used

**Cause**: Query planner statistics out of date

**Solution**:
```bash
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  ANALYZE maproom.chunks;
"
```

### Query Performance Worse Than Baseline

**Possible Causes**:
1. Statistics out of date (run ANALYZE)
2. Test instance has different PostgreSQL settings than production
3. Test instance has limited resources (CPU/memory)

**Investigation**:
```bash
# Check query planner choices
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
  [your slow query here]
"

# Check index bloat
docker exec migration-test-pg psql -U postgres -d maproom_test -c "
  SELECT
    indexname,
    pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) as size
  FROM pg_indexes
  WHERE schemaname = 'maproom' AND tablename = 'chunks'
  ORDER BY pg_relation_size(schemaname||'.'||indexname) DESC;
"
```

## Additional Resources

### Related Documentation

- **Architecture**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md`
- **Migration SQL**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Quality Strategy**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/planning/quality-strategy.md`

### PostgreSQL Documentation

- [CREATE INDEX CONCURRENTLY](https://www.postgresql.org/docs/15/sql-createindex.html#SQL-CREATEINDEX-CONCURRENTLY)
- [Partial Indexes](https://www.postgresql.org/docs/15/indexes-partial.html)
- [Index-Only Scans](https://www.postgresql.org/docs/15/indexes-index-only-scans.html)
- [pg_stat_user_indexes](https://www.postgresql.org/docs/15/monitoring-stats.html#MONITORING-PG-STAT-ALL-INDEXES-VIEW)

---

**Document Version**: 1.0
**Last Updated**: 2025-11-09
**Maintained By**: IDXSIZE Project Team
