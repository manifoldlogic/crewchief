# IDXSIZE Testing Documentation

This directory contains testing procedures and results for the IDXSIZE (Index Size Limits) project.

## Documents

### production-clone-test-procedure.md

**Purpose**: Step-by-step manual testing procedure for validating migration 0017 on a production database clone.

**When to Use**: Before deploying migration 0017 to production. This is a **required** test before production deployment.

**What It Tests**:
- Migration executes successfully without errors
- Zero data loss (chunk count verification)
- Correct index changes (2 new indexes created, 1 old index dropped)
- Storage impact within acceptable range (+25-40%)
- Query performance maintained or improved
- Large preview chunks can be indexed and queried (core bug fix)

**Test Duration**: ~30-60 minutes (depending on database size)

**Prerequisites**:
- Production database backup access
- Docker environment for isolated PostgreSQL instance
- ~2GB free disk space (for typical database)

## Quick Start

```bash
# Navigate to testing documentation
cd /workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing

# Review test procedure
cat production-clone-test-procedure.md

# Follow step-by-step instructions in the document
```

## Test Workflow

1. **Read**: Review `production-clone-test-procedure.md` completely before starting
2. **Setup**: Create production backup and isolated test PostgreSQL instance
3. **Baseline**: Capture pre-migration metrics (chunk count, index sizes, query performance)
4. **Execute**: Run migration 0017 with timing
5. **Validate**: Verify index changes, data integrity, storage impact
6. **Test**: Run critical path queries and verify performance
7. **Document**: Complete test results template with all measurements
8. **Review**: Verify all success criteria met before production deployment
9. **Cleanup**: Remove test environment and archive results

## Success Criteria

### MUST PASS (Blocking)
- ✅ Migration completes without errors
- ✅ Zero data loss (chunk count exact match)
- ✅ Old index dropped, 2 new indexes created
- ✅ Large preview chunks can be queried successfully
- ✅ Query performance within ±30% of baseline

### SHOULD PASS (Investigate)
- Storage increase < 40%
- Migration duration < 10 minutes
- No PostgreSQL errors in logs

## Expected Results

Based on architecture analysis:

- **Storage increase**: ~31% (+~155MB for typical production database)
- **Migration duration**: 2-5 minutes
- **Query performance**:
  - Small previews (95% of queries): 5-10ms
  - Large previews (5% of queries): 15-30ms
  - Average: ~7ms

## Test Results

Test results will be stored in this directory after execution:

```
testing/
├── README.md                                    # This file
├── production-clone-test-procedure.md           # Test procedure
└── migration_test_results_YYYYMMDD_HHMMSS.txt  # Results (created during test)
```

## Related Documentation

- **Migration SQL**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Architecture**: `../planning/architecture.md`
- **Quality Strategy**: `../planning/quality-strategy.md`
- **Analysis**: `../planning/analysis.md`

## Migration Details

**Migration 0017**: Fix index size limit errors

**Problem**: The `idx_chunks_search_covering` index fails when preview > 2704 bytes due to PostgreSQL's index entry size limit (8KB page size / 3 ≈ 2704 bytes).

**Solution**: Multi-index strategy
1. **Partial covering index** (`idx_chunks_search_small_preview`): Handles 95% of chunks with preview <= 2000 bytes. Enables index-only scans.
2. **Basic fallback index** (`idx_chunks_search_basic`): Handles 100% of chunks including large previews. Requires heap lookup but works for all data.

**Impact**:
- ✅ Eliminates index size errors completely
- ✅ Maintains performance for common case (95% of queries)
- ✅ Enables indexing of large code blocks (current blocker)
- ✅ Zero application changes required

## Contact

For questions about testing procedures or results interpretation, see:
- Project documentation: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/`
- Ticket: `IDXSIZE-2003` in `../tickets/`
