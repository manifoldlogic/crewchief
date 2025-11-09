# Quality Strategy: Schema Migration Safety

## Testing Philosophy

Schema migrations are **high-risk, low-reversibility** changes. Our strategy focuses on:

1. **Prevention** - Validate before applying
2. **Detection** - Test in production-like environment
3. **Recovery** - Ensure rollback is possible
4. **Confidence** - Measure, don't guess

**NOT chasing 100% coverage** - We need confidence the migration works, not ceremonial checkboxes.

## Test Pyramid

### L1: SQL Syntax Validation (Automated, Fast)

**Goal**: Ensure migration SQL is syntactically correct

**Tests**:
```bash
# Validate SQL syntax
psql --dry-run < migration.sql

# Check for common mistakes
grep -E "(DROP TABLE|TRUNCATE|DELETE FROM)" migration.sql && echo "❌ Destructive operations detected"
```

**Pass criteria**: SQL parses without errors

### L2: Migration Dry Run (Automated, Medium)

**Goal**: Test migration on empty database

```bash
# Create fresh test database
docker run -d --name test-pg pgvector/pgvector:pg15
psql -h test-pg -U postgres -c "CREATE DATABASE test_maproom;"

# Run full schema init
psql -h test-pg -U postgres -d test_maproom < packages/maproom-mcp/config/init.sql

# Apply migration
psql -h test-pg -U postgres -d test_maproom < migration.sql

# Validate indexes created
psql -h test-pg -U postgres -d test_maproom -c "
  SELECT indexname
  FROM pg_indexes
  WHERE schemaname = 'maproom'
    AND tablename = 'chunks'
  ORDER BY indexname;
"

# Expected output:
# idx_chunks_search_small_preview
# idx_chunks_search_hash
# idx_chunks_search_basic
```

**Pass criteria**:
- Migration completes without errors
- All 3 new indexes exist
- Old index (idx_chunks_search_covering) is gone

### L3: Data Population Test (Automated, Slow)

**Goal**: Test migration with realistic data

```bash
# Use test database from L2
# Insert test data with varying preview sizes

psql -h test-pg -U postgres -d test_maproom <<SQL
-- Insert chunks with small previews (should use partial index)
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  1,
  'function',
  generate_series(1, 1000),
  generate_series(1, 1000),
  'func_' || generate_series(1, 1000),
  'Short preview text';  -- ~20 bytes

-- Insert chunks with large previews (should use basic index)
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  2,
  'function',
  generate_series(1, 100),
  generate_series(1, 100),
  'big_func_' || generate_series(1, 100),
  REPEAT('x', 3000);  -- 3000 bytes, exceeds limit

-- Insert chunks with VERY large previews (stress test)
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  3,
  'function',
  generate_series(1, 10),
  generate_series(1, 10),
  'huge_func_' || generate_series(1, 10),
  REPEAT('y', 10000);  -- 10KB preview

-- Verify all inserts succeeded (this was failing before)
SELECT COUNT(*) FROM maproom.chunks;
-- Expected: 1110 rows

-- Check index usage
ANALYZE maproom.chunks;

-- Test query on small previews (should use partial covering index)
EXPLAIN (ANALYZE, BUFFERS) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id = 1 AND kind = 'function' LIMIT 10;
-- Expected plan: Index Only Scan using idx_chunks_search_small_preview

-- Test query on large previews (should use basic index)
EXPLAIN (ANALYZE, BUFFERS) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id = 2 AND kind = 'function' LIMIT 10;
-- Expected plan: Index Scan using idx_chunks_search_basic + Heap Fetch

SQL
```

**Pass criteria**:
- All INSERT statements succeed (no size errors)
- Query planner chooses appropriate indexes
- Performance <50ms for both query types

### L4: Real Data Migration Test (Manual, Production-Like)

**Goal**: Test on actual maproom database clone

```bash
# Clone production database
docker exec maproom-postgres pg_dump -U maproom maproom > prod_backup.sql

# Restore to test environment
docker run -d --name migration-test pgvector/pgvector:pg15
psql -h migration-test -U postgres -c "CREATE DATABASE maproom_test;"
psql -h migration-test -U postgres -d maproom_test < prod_backup.sql

# Apply migration
time psql -h migration-test -U postgres -d maproom_test < migration.sql

# Verify:
# 1. Migration completed successfully
# 2. No data lost
SELECT COUNT(*) FROM maproom.chunks;  # Should match original

# 3. Old index gone, new indexes exist
\di maproom.idx_chunks_*

# 4. Query performance acceptable
EXPLAIN (ANALYZE) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id = (SELECT MIN(id) FROM maproom.files) AND kind = 'function' LIMIT 10;

# 5. Check for any failed rows
SELECT COUNT(*) FROM maproom.chunks WHERE LENGTH(preview) > 5000;
-- If this shows rows, verify queries still work for them
```

**Pass criteria**:
- Migration completes in <10 minutes
- Zero data loss (row count matches)
- Sample queries return correct results
- No errors in PostgreSQL logs

## Critical Path Testing

**These queries MUST work after migration:**

### Query 1: File-based search (most common)
```sql
SELECT id, symbol_name, kind, preview
FROM maproom.chunks
WHERE file_id = 42
  AND kind IN ('function', 'class')
ORDER BY start_line
LIMIT 20;
```

**Expected**: <20ms, uses idx_chunks_search_small_preview or idx_chunks_search_basic

### Query 2: Symbol lookup
```sql
SELECT id, file_id, start_line, preview
FROM maproom.chunks
WHERE symbol_name = 'main';
```

**Expected**: Uses existing idx_chunks_symbol_name (not affected by migration)

### Query 3: Vector search (hybrid)
```sql
SELECT c.id, c.symbol_name, c.preview
FROM maproom.chunks c
WHERE c.code_embedding <-> '[...]'::vector < 0.5
  AND c.file_id = 42
ORDER BY c.code_embedding <-> '[...]'::vector
LIMIT 10;
```

**Expected**: Uses idx_chunks_code_embedding + new search index

### Query 4: Large preview edge case
```sql
SELECT symbol_name, LENGTH(preview) as preview_len
FROM maproom.chunks
WHERE LENGTH(preview) > 2000
  AND kind = 'function'
LIMIT 10;
```

**Expected**: Works without errors, uses idx_chunks_search_basic

## Performance Benchmarks

### Pre-Migration Baseline

```sql
-- Benchmark existing queries (before migration)
\timing on

-- Query 1: Small file search
EXPLAIN (ANALYZE, BUFFERS) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id = 1 AND kind = 'function' LIMIT 10;
-- Record: Planning Time, Execution Time, Buffers

-- Query 2: Large result set
EXPLAIN (ANALYZE, BUFFERS) SELECT symbol_name FROM maproom.chunks WHERE file_id < 10 AND kind = 'class';
-- Record: Planning Time, Execution Time, Buffers
```

### Post-Migration Verification

```sql
-- Run same queries after migration
-- Compare timings:

-- Acceptable: ±30% variance
-- Red flag: >2x slower
-- Investigate: Any query >100ms
```

**Target**: 95% of queries within ±30% of baseline

## Rollback Strategy

### Rollback SQL

```sql
-- Rollback migration: IDXSIZE-001
-- Restores original covering index (may fail on large previews)

BEGIN;

-- Drop new indexes
DROP INDEX IF EXISTS maproom.idx_chunks_search_small_preview;
DROP INDEX IF EXISTS maproom.idx_chunks_search_hash;
DROP INDEX IF EXISTS maproom.idx_chunks_search_basic;

-- Restore original covering index (will fail if data has large previews)
-- This is why forward migration is one-way
CREATE INDEX CONCURRENTLY idx_chunks_search_covering
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, preview);

COMMIT;
```

**IMPORTANT**: Rollback only works if no large previews were indexed during migration. After migration, new large-preview chunks can be inserted, making rollback impossible.

**Recommendation**: **Don't rollback**. If migration fails, fix forward, not backward.

## Monitoring Post-Migration

### Immediate Checks (First Hour)

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
  AND indexname LIKE 'idx_chunks_search%';
```

**Expected**:
- idx_chunks_search_small_preview: ~475MB, scans increasing
- idx_chunks_search_hash: ~100MB, scans = 0 (rarely used)
- idx_chunks_search_basic: ~80MB, scans increasing (for large previews)

### Daily Checks (First Week)

```bash
# Check PostgreSQL logs for errors
docker logs maproom-postgres | grep -i error

# Check slow query log
docker exec maproom-postgres psql -U maproom -d maproom -c "
  SELECT query, calls, mean_exec_time, max_exec_time
  FROM pg_stat_statements
  WHERE query LIKE '%chunks%'
  ORDER BY mean_exec_time DESC
  LIMIT 10;
"
```

**Red flags**:
- Any query >200ms average
- Error rate >0.1% of queries
- Index not being used (idx_scan = 0 after 1 day)

### Weekly Checks (First Month)

```sql
-- Index bloat check
SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as size,
  idx_scan,
  ROUND(100.0 * idx_scan / NULLIF(idx_tup_read, 0), 2) as hit_ratio
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND tablename = 'chunks';
```

**Action items**:
- If hit_ratio < 10%: Index not useful, consider dropping
- If size grows >2x in 1 week: Investigate data growth
- If idx_scan = 0 after 1 week: Index not needed

## Test Data Generation

### Synthetic Data Sets

**Small preview dataset** (95% of data):
```sql
-- Generate 10,000 chunks with preview 50-500 bytes
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  (random() * 100)::int + 1,
  (ARRAY['function', 'class', 'interface'])[floor(random() * 3 + 1)],
  generate_series(1, 10000),
  generate_series(1, 10000),
  'symbol_' || generate_series(1, 10000),
  REPEAT('x', (random() * 450 + 50)::int);
```

**Large preview dataset** (5% edge case):
```sql
-- Generate 500 chunks with preview 2000-5000 bytes
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  (random() * 100)::int + 1,
  'function',
  generate_series(10001, 10500),
  generate_series(10001, 10500),
  'large_symbol_' || generate_series(10001, 10500),
  REPEAT('y', (random() * 3000 + 2000)::int);
```

**Extreme edge case** (verify no failures):
```sql
-- Generate 10 chunks with preview 10KB-50KB
INSERT INTO maproom.chunks (file_id, kind, start_line, end_line, symbol_name, preview)
SELECT
  (random() * 100)::int + 1,
  'function',
  generate_series(10501, 10510),
  generate_series(10501, 10510),
  'extreme_symbol_' || generate_series(10501, 10510),
  REPEAT('z', (random() * 40000 + 10000)::int);
```

## Success Criteria

### Must-Pass (Blocking)

- ✅ Migration SQL executes without errors
- ✅ All 3 new indexes created successfully
- ✅ Old index dropped
- ✅ INSERT of 10KB preview succeeds (no size error)
- ✅ Critical path queries return correct results
- ✅ Query performance within ±30% of baseline

### Should-Pass (Investigate if Fail)

- Zero PostgreSQL errors in logs during migration
- Index usage stats show small_preview index used most
- Storage increase <40% (expected: ~31%)
- Migration completes in <10 minutes

### Nice-to-Have (Not Blocking)

- Hash-based index used for duplicate detection queries
- Index-only scan rate >90% for small previews
- Query planning time <5ms

## Risk-Based Testing Prioritization

**High Risk** (must test):
- Large preview INSERT/UPDATE (prevents size errors)
- Query correctness (prevents wrong results)
- Index selection (prevents performance regression)

**Medium Risk** (should test):
- Concurrent operations during migration
- Storage growth rate
- Rollback procedure

**Low Risk** (can skip for MVP):
- Hash collision scenarios (MD5 is good enough)
- Extreme concurrency (100+ simultaneous queries)
- Multi-TB database scale

## Automated Test Suite

Create `test-index-migration.sh`:
```bash
#!/bin/bash
set -e

echo "🔧 Testing index migration..."

# L1: Syntax validation
echo "✓ L1: SQL syntax validation"
psql --dry-run < migration.sql

# L2: Empty database test
echo "✓ L2: Migration on empty database"
# ... (test code from above)

# L3: Data population test
echo "✓ L3: Data population test"
# ... (test code from above)

# L4: Production clone test (manual)
echo "⚠️  L4: Manual testing required on production clone"

echo "✅ All automated tests passed"
```

Run before applying migration to production.

## Conclusion

This testing strategy provides **confidence without ceremony**:
- Automated tests catch obvious errors
- Manual testing validates production-like scenarios
- Monitoring ensures post-migration success
- Rollback plan exists (but forward migration preferred)

**Philosophy**: Test the critical path thoroughly, monitor everything, fix issues as they arise.
