# Implementation Plan: PostgreSQL Index Size Limit Fix

## Project Overview

**Goal**: Eliminate PostgreSQL B-tree index size limit errors when indexing real-world codebases

**Problem**: `idx_chunks_search_covering` fails when preview text exceeds 2704 bytes

**Solution**: Multi-index strategy with partial indexes and hash-based fallbacks

**Timeline**: 2-3 days (1 implementation, 1 testing, 0.5 buffer)

## Success Criteria

- ✅ Index any codebase without size errors (100% success rate)
- ✅ Query performance <20ms for 95th percentile
- ✅ Zero application changes required
- ✅ Migration completes in <10 minutes
- ✅ All automated tests pass
- ✅ Production monitoring shows stable performance

## Phase 1: Migration SQL Development (Day 1)

### Step 1.1: Create Migration File

**Agent**: database-engineer

**File**: `/workspace/crates/maproom/migrations/0013_fix_index_size_limits.sql`

**Content**:
```sql
-- Migration 0013: Fix index size limit errors
--
-- Problem: idx_chunks_search_covering fails when preview > 2704 bytes
-- Solution: Multi-index strategy (partial + hash + basic)
--
-- References:
-- - .crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md
-- - .crewchief/projects/IDXSIZE_index-size-limits/planning/analysis.md

SET statement_timeout = '10min';

BEGIN;

-- Drop the problematic covering index
DROP INDEX IF EXISTS maproom.idx_chunks_search_covering;

COMMIT;

-- Create new indexes concurrently (no lock)

-- Partial covering index for small previews (95% of data)
CREATE INDEX CONCURRENTLY idx_chunks_search_small_preview
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, preview)
  WHERE LENGTH(preview) <= 2000;

COMMENT ON INDEX maproom.idx_chunks_search_small_preview IS
  'Covering index for search queries with preview <= 2000 bytes. Enables index-only scans for 95%+ of chunks.';

-- Hash-based covering index for existence checks (100% of data)
CREATE INDEX CONCURRENTLY idx_chunks_search_hash
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, MD5(preview::bytea));

COMMENT ON INDEX maproom.idx_chunks_search_hash IS
  'Hash-based covering index for duplicate detection. Works for all chunk sizes.';

-- Basic non-covering index as universal fallback (100% of data)
CREATE INDEX CONCURRENTLY idx_chunks_search_basic
  ON maproom.chunks (file_id, kind, start_line);

COMMENT ON INDEX maproom.idx_chunks_search_basic IS
  'Basic index for chunks with large previews. Requires heap lookup but works for 100% of data.';

-- Update statistics for query planner
ANALYZE maproom.chunks;

RESET statement_timeout;
```

**Acceptance Criteria**:
- Migration SQL file created
- Syntax validated (`psql --dry-run`)
- Comments explain rationale
- Statement timeout set

### Step 1.2: Create Rollback Script

**Agent**: database-engineer

**File**: `/workspace/crates/maproom/migrations/rollback/0013_rollback.sql`

**Content**:
```sql
-- Rollback 0013: Restore original covering index
-- WARNING: Will fail if database has chunks with preview > 2704 bytes

BEGIN;

-- Drop new indexes
DROP INDEX IF EXISTS maproom.idx_chunks_search_small_preview;
DROP INDEX IF EXISTS maproom.idx_chunks_search_hash;
DROP INDEX IF EXISTS maproom.idx_chunks_search_basic;

-- Attempt to restore original index (may fail)
CREATE INDEX CONCURRENTLY idx_chunks_search_covering
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, preview);

COMMIT;
```

**Acceptance Criteria**:
- Rollback script created
- Warning documented
- Works on empty database

### Step 1.3: Update Documentation

**Agent**: general-purpose

**Files**:
- `/workspace/docs/DATABASE_INDICES.md` - Update index documentation
- `/workspace/CHANGELOG.md` - Add migration entry

**Changelog Entry**:
```markdown
## [Unreleased]

### Fixed
- **Index Size Limit Errors**: Fixed PostgreSQL B-tree index size limit errors when indexing code with large preview text
  - Replaced single covering index with multi-index strategy
  - Handles 100% of real-world codebases without errors
  - Maintains performance: 95%+ queries still use index-only scans
  - Migration 0013: See `crates/maproom/migrations/0013_fix_index_size_limits.sql`
```

**Acceptance Criteria**:
- Documentation updated
- CHANGELOG entry added
- Links to migration file

**Deliverable**: Migration ready for testing

---

## Phase 2: Testing and Validation (Day 2)

### Step 2.1: Automated Test Suite

**Agent**: database-engineer

**File**: `/workspace/crates/maproom/tests/test_index_migration.sh`

**Test Levels** (from quality-strategy.md):
1. **L1: SQL Syntax Validation**
2. **L2: Empty Database Test**
3. **L3: Data Population Test**
4. **L4: Production Clone Test** (manual)

**Test Script**:
```bash
#!/bin/bash
set -e

echo "🧪 Testing index migration 0013..."

# L1: Syntax validation
echo "✓ L1: Validating SQL syntax"
psql --dry-run < migrations/0013_fix_index_size_limits.sql

# L2: Empty database test
echo "✓ L2: Testing on empty database"
docker run -d --name test-pg-empty pgvector/pgvector:pg15
sleep 5
psql -h localhost -p 5432 -U postgres -c "CREATE DATABASE test_maproom;"
psql -h localhost -p 5432 -U postgres -d test_maproom < packages/maproom-mcp/config/init.sql
psql -h localhost -p 5432 -U postgres -d test_maproom < migrations/0013_fix_index_size_limits.sql

# Verify indexes
INDEXES=$(psql -h localhost -p 5432 -U postgres -d test_maproom -t -c "
  SELECT COUNT(*) FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname LIKE 'idx_chunks_search_%';
")

if [ "$INDEXES" != "3" ]; then
  echo "❌ Expected 3 indexes, found $INDEXES"
  exit 1
fi

echo "✓ All 3 indexes created"

# L3: Data population test
echo "✓ L3: Testing with data"
psql -h localhost -p 5432 -U postgres -d test_maproom <<SQL
-- Small previews (should work)
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT 1, 'function', generate_series(1, 100), generate_series(1, 100),
       'func_' || generate_series(1, 100), 'Short preview';

-- Large previews (should work now, failed before)
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT 2, 'function', generate_series(1, 10), generate_series(1, 10),
       'big_func_' || generate_series(1, 10), REPEAT('x', 3000);

-- Verify inserts
SELECT COUNT(*) FROM maproom.chunks;
SQL

echo "✓ Data insertion successful"

# Cleanup
docker rm -f test-pg-empty

echo "✅ All automated tests passed"
```

**Acceptance Criteria**:
- All L1-L3 tests pass
- Script runs in <5 minutes
- No errors or warnings

### Step 2.2: Query Performance Testing

**Agent**: database-engineer

**Test Queries**:
```sql
-- Query 1: Small preview (should use partial index)
EXPLAIN (ANALYZE, BUFFERS)
SELECT symbol_name, preview
FROM maproom.chunks
WHERE file_id = 1 AND kind = 'function'
LIMIT 10;
-- Expected: Index Only Scan using idx_chunks_search_small_preview
-- Expected: Execution time < 20ms

-- Query 2: Large preview (should use basic index)
EXPLAIN (ANALYZE, BUFFERS)
SELECT symbol_name, preview
FROM maproom.chunks
WHERE file_id = 2 AND kind = 'function'
LIMIT 10;
-- Expected: Index Scan using idx_chunks_search_basic + Heap Fetch
-- Expected: Execution time < 50ms
```

**Acceptance Criteria**:
- Small preview queries use partial index
- Large preview queries use basic index
- No sequential scans
- Performance within targets

### Step 2.3: Production Clone Test (Manual)

**Agent**: database-engineer (manual execution)

**Steps**:
```bash
# 1. Clone production database
docker exec maproom-postgres pg_dump -U maproom maproom > prod_backup.sql

# 2. Restore to test environment
docker run -d --name migration-test -p 5434:5432 pgvector/pgvector:pg15
psql -h localhost -p 5434 -U postgres -c "CREATE DATABASE maproom_test;"
psql -h localhost -p 5434 -U postgres -d maproom_test < prod_backup.sql

# 3. Baseline measurements
psql -h localhost -p 5434 -U postgres -d maproom_test -c "
  SELECT COUNT(*) FROM maproom.chunks;
  SELECT pg_size_pretty(pg_relation_size('maproom.chunks'));
"

# 4. Apply migration
time psql -h localhost -p 5434 -U postgres -d maproom_test < migrations/0013_fix_index_size_limits.sql

# 5. Verify results
psql -h localhost -p 5434 -U postgres -d maproom_test -c "
  \di maproom.idx_chunks_search_*
  SELECT COUNT(*) FROM maproom.chunks;
"

# 6. Test queries
psql -h localhost -p 5434 -U postgres -d maproom_test < test_queries.sql

# 7. Cleanup
docker rm -f migration-test
```

**Acceptance Criteria**:
- Migration completes in <10 minutes
- Zero data loss (row count matches)
- All indexes created successfully
- Queries return correct results

**Deliverable**: Migration validated, ready for production

---

## Phase 3: Production Deployment (Day 3)

### Step 3.1: Pre-Deployment Checklist

**Agent**: database-engineer

**Checklist**:
- [ ] All automated tests passed
- [ ] Production clone test successful
- [ ] Rollback script tested
- [ ] Migration window scheduled (low traffic)
- [ ] Database backup created
- [ ] Monitoring alerts configured
- [ ] Team notified of maintenance

**Verification**:
```bash
# Create production backup
docker exec maproom-postgres pg_dump -U maproom maproom | gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz

# Verify backup
gunzip -c backup_*.sql.gz | head -100

# Check current disk space
df -h | grep postgres
```

**Acceptance Criteria**:
- All checklist items completed
- Backup verified
- Team ready

### Step 3.2: Execute Migration

**Agent**: database-engineer (manual execution)

**Steps**:
```bash
# 1. Announce maintenance
echo "⚠️  Database maintenance starting..."

# 2. Run migration
time psql $DATABASE_URL < migrations/0013_fix_index_size_limits.sql

# 3. Verify success
psql $DATABASE_URL -c "
  SELECT indexname, pg_size_pretty(pg_relation_size(indexrelid))
  FROM pg_indexes
  JOIN pg_stat_user_indexes USING (indexrelid)
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
    AND indexname LIKE 'idx_chunks_search_%';
"

# 4. Test query
psql $DATABASE_URL -c "
  SELECT symbol_name, LENGTH(preview)
  FROM maproom.chunks
  WHERE file_id = (SELECT MIN(id) FROM maproom.files)
    AND kind = 'function'
  LIMIT 5;
"

# 5. Update statistics
psql $DATABASE_URL -c "ANALYZE maproom.chunks;"

# 6. Announce completion
echo "✅ Migration complete"
```

**Acceptance Criteria**:
- Migration completes without errors
- All 3 new indexes exist
- Old index removed
- Test query works

### Step 3.3: Post-Deployment Monitoring

**Agent**: database-engineer

**First Hour Checks**:
```sql
-- Check index sizes
SELECT
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as size,
  idx_scan as scans,
  idx_tup_read as tuples_read
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND tablename = 'chunks'
  AND indexname LIKE 'idx_chunks_search%'
ORDER BY indexname;
```

**Expected**:
- `idx_chunks_search_small_preview`: ~475MB, scans > 0
- `idx_chunks_search_hash`: ~100MB, scans = 0 (rarely used)
- `idx_chunks_search_basic`: ~80MB, scans > 0

**Alert Triggers**:
- Any index with 0 scans after 1 hour (investigate)
- Query errors >0.1% of total
- Any query >500ms

**Monitoring Duration**: 24 hours active monitoring, 1 week passive

**Acceptance Criteria**:
- No errors in PostgreSQL logs
- Query performance within SLA
- Index usage as expected

**Deliverable**: Migration deployed and stable

---

## Phase 4: Documentation and Cleanup (Day 3 afternoon)

### Step 4.1: Update Migration Log

**Agent**: general-purpose

**File**: `/workspace/docs/migrations/README.md`

**Entry**:
```markdown
## Migration 0013: Fix Index Size Limits (2025-11-XX)

**Problem**: B-tree index `idx_chunks_search_covering` failed when preview text exceeded 2704 bytes

**Solution**: Multi-index strategy with partial indexes

**Changes**:
- Dropped: `idx_chunks_search_covering`
- Added: `idx_chunks_search_small_preview` (partial, preview <= 2000 bytes)
- Added: `idx_chunks_search_hash` (hash-based, all sizes)
- Added: `idx_chunks_search_basic` (non-covering, all sizes)

**Impact**:
- Handles 100% of codebases (no more size errors)
- 95%+ queries still use index-only scans
- Storage increase: ~31% (+155MB typical)

**Rollback**: See `migrations/rollback/0013_rollback.sql` (not recommended)

**References**:
- Project: `.crewchief/projects/IDXSIZE_index-size-limits/`
- Analysis: `.crewchief/projects/IDXSIZE_index-size-limits/planning/analysis.md`
```

**Acceptance Criteria**:
- Migration logged
- Links to documentation
- Impact documented

### Step 4.2: Verify Indexing Works

**Agent**: rust-indexer-engineer

**Test**:
```bash
# Try indexing a real codebase that previously failed
export DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"
maproom scan /workspace --force

# Should complete without errors
echo $?  # Exit code 0 = success
```

**Acceptance Criteria**:
- Scan completes successfully
- No "index row size exceeds" errors
- Large-preview chunks indexed

### Step 4.3: Update Project README

**Agent**: general-purpose

**File**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/README.md`

**Add section**:
```markdown
## Implementation Status

✅ **COMPLETED** - Deployed to production on 2025-11-XX

**Migration**: `crates/maproom/migrations/0013_fix_index_size_limits.sql`

**Results**:
- 100% of codebases index successfully (was ~50% before)
- Query performance maintained (95%+ index-only scans)
- Zero rollbacks or issues
- Storage impact: +31% as expected

**Lessons Learned**:
- Always validate index size constraints during schema design
- Partial indexes are powerful for filtering large values
- PostgreSQL query planner handles multiple indexes intelligently
- Concurrent index creation enables zero-downtime migrations
```

**Acceptance Criteria**:
- Project marked complete
- Results documented
- Lessons learned captured

**Deliverable**: Project complete and documented

---

## Agent Assignments

### Phase 1: Migration Development
- **database-engineer** - Migration SQL, rollback script
- **general-purpose** - Documentation updates

### Phase 2: Testing
- **database-engineer** - Test suite, query testing, production clone test

### Phase 3: Deployment
- **database-engineer** - Execute migration, monitoring

### Phase 4: Documentation
- **general-purpose** - Migration log, project status
- **rust-indexer-engineer** - Verify indexing works

## Risk Mitigation

### Risk 1: Migration Takes Too Long

**Mitigation**:
- Use `CREATE INDEX CONCURRENTLY` (no table lock)
- Schedule during low-traffic window
- Can be killed mid-flight if needed

**Fallback**: Pause migration, resume during next window

### Risk 2: Query Performance Degrades

**Mitigation**:
- Baseline measurements before migration
- EXPLAIN ANALYZE on test queries
- Monitor query timing post-migration

**Fallback**: If >30% slower, add expression indexes or optimize queries

### Risk 3: Storage Exceeds Capacity

**Mitigation**:
- Calculate expected size increase (~31%)
- Verify disk space before migration
- Monitor storage growth

**Fallback**: Drop idx_chunks_search_hash if not used (saves ~100MB)

## Timeline

### Day 1: Migration Development
- **Morning**: Steps 1.1-1.2 (Migration SQL, rollback)
- **Afternoon**: Step 1.3 (Documentation)
- **End of Day**: Migration ready for testing

### Day 2: Testing
- **Morning**: Steps 2.1-2.2 (Automated tests, query testing)
- **Afternoon**: Step 2.3 (Production clone test)
- **End of Day**: Migration validated

### Day 3: Deployment and Documentation
- **Morning**: Steps 3.1-3.3 (Deploy, monitor)
- **Afternoon**: Steps 4.1-4.3 (Document, verify)
- **End of Day**: Project complete

**Buffer**: 0.5 days for unexpected issues

**Total**: 2-3 days wall time

## Success Metrics

### Must-Have (Blocking)

- ✅ Migration completes without errors
- ✅ All 3 new indexes created
- ✅ Can index large-preview chunks without errors
- ✅ Query performance within ±30% of baseline
- ✅ Zero data loss

### Should-Have (Investigate if Missing)

- 95%+ queries use index-only scans
- Migration completes in <10 minutes
- Storage increase <40%
- No PostgreSQL errors in logs

### Nice-to-Have (Optimize Later)

- Index-only scan rate >98%
- Average query time <10ms
- Zero monitoring alerts

## Acceptance Checklist

Final sign-off requires all of the following:

- [ ] Migration 0013 deployed to production
- [ ] All automated tests passed
- [ ] Production clone test successful
- [ ] Post-deployment monitoring complete (24 hours)
- [ ] No errors or performance regressions
- [ ] Documentation updated (CHANGELOG, DATABASE_INDICES, migration log)
- [ ] Team trained on new index strategy
- [ ] Rollback procedure documented (even if not recommended)

**Sign-off**: Database Engineer, Tech Lead

---

## Appendix: Quick Reference Commands

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

**Timeline**: 2-3 days (1 implementation, 1 testing, 0.5 buffer)
**Risk Level**: Low (schema changes are well-tested, rollback available)
**Impact**: High (fixes critical blocking issue for 50%+ of users)
