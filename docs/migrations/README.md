# Database Migration Log

This document tracks all database schema migrations for the Maproom indexer, documenting what changed, why, and the impact of each migration.

## Purpose

The migration log provides:
- Historical record of schema changes
- Problem/solution context for future troubleshooting
- Impact analysis (performance, storage, compatibility)
- Rollback procedures when applicable
- References to planning documentation

## Migration History

### Migration 0017: Fix Index Size Limits (2025-11-09)

**Migration File**: [`crates/maproom/migrations/0017_fix_index_size_limits.sql`](/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql)

**Problem**:
PostgreSQL B-tree indexes have a maximum entry size of 2704 bytes (1/3 of 8KB page size). The previous `idx_chunks_search_covering` index included the `preview` column in its INCLUDE clause, causing INSERT failures when chunk preview text exceeded 2704 bytes. Error message:
```
ERROR: index row size 2768 exceeds btree version 4 maximum 2704 for index "idx_chunks_search_covering"
```

This prevented indexing of code chunks with large preview text, particularly:
- Functions with extensive doc comments
- Markdown headings with long text
- Code blocks with detailed content
- Any symbol with preview > 2704 bytes

**Solution**:
Implemented a two-index strategy:

1. **idx_chunks_search_small_preview** (21 MB) - Partial covering index
   - Indexes: `file_id`, `kind`, `start_line`
   - Includes: `preview` (but only for chunks where `LENGTH(preview) <= 2000`)
   - Coverage: 95%+ of data (47,178 of 47,522 chunks)
   - Benefit: Index-only scans for most queries (no heap fetch required)

2. **idx_chunks_search_basic** (1.48 MB) - Universal fallback index
   - Indexes: `file_id`, `kind`, `start_line`
   - Includes: Nothing (heap fetch required for preview data)
   - Coverage: 100% of data (all 47,522 chunks)
   - Benefit: Works for all preview sizes, including > 2704 bytes

**Index Changes**:
- **Dropped**: `idx_chunks_search_covering` (old single index with size limit issue)
- **Created**: `idx_chunks_search_small_preview` (partial covering index for ≤2000 byte previews)
- **Created**: `idx_chunks_search_basic` (universal index with no size constraints)

**Impact Analysis**:

*Storage*:
- New indexes: 22.5 MB total (21 MB + 1.48 MB)
- Storage increase: Well within acceptable range for 47,522 chunks
- No unexpected growth or performance degradation

*Performance*:
- Small preview queries (95% of data): **0.025ms** execution time
  - Uses `idx_chunks_search_small_preview` with Index Only Scan
  - 800x faster than 20ms SLA threshold
- Large preview queries (5% of data): **0.120ms** execution time
  - Uses `idx_chunks_search_basic` with Index Scan + heap fetch
  - 416x faster than 50ms threshold
- Query planner correctly selects optimal index for each query pattern

*Compatibility*:
- ✅ **100% data coverage**: All 47,522 chunks indexed successfully
- ✅ **Critical fix validated**: 19 chunks with preview > 2704 bytes now queryable
  - Largest: 4,336 bytes (test_medium_batch_50_chunks)
  - Previously would have caused INSERT errors
- ✅ **Zero data loss**: All existing chunks maintained
- ✅ **No breaking changes**: Existing queries continue to work

**Deployment**:
- **Date**: 2025-11-09
- **Environment**: Development (maproom-postgres Docker container)
- **Migration Method**: Applied via psql
- **Duration**: Instantaneous (migration pre-applied)
- **Downtime**: None (CREATE INDEX CONCURRENTLY used)

**Monitoring Results**:
Post-deployment monitoring (IDXSIZE-3003) confirmed:
- ✅ Both indexes operational and actively used (scan counts > 0)
- ✅ Index sizes stable and within expected ranges
- ✅ Query performance excellent (sub-millisecond execution times)
- ✅ No PostgreSQL errors in logs
- ✅ Database health normal (136 MB total, 50 MB chunks table)
- ✅ All 19 large preview chunks queryable without errors

**Rollback Procedure**:
⚠️ **NOT RECOMMENDED** - Rollback will restore the index size limit bug.

If rollback is absolutely necessary:
1. Review rollback script: [`crates/maproom/migrations/rollback/0017_rollback.sql`](/workspace/crates/maproom/migrations/rollback/0017_rollback.sql)
2. **WARNING**: After rollback, chunks with preview > 2704 bytes will cause INSERT errors
3. Only rollback if critical issues discovered with new indexes
4. Coordinate with team before executing rollback

**Planning Documentation**:
- Project folder: [`.crewchief/projects/IDXSIZE_index-size-limits/`](/.crewchief/projects/IDXSIZE_index-size-limits/)
- Technical analysis: [`.crewchief/projects/IDXSIZE_index-size-limits/planning/analysis.md`](/.crewchief/projects/IDXSIZE_index-size-limits/planning/analysis.md)
- Architecture design: [`.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md`](/.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md)
- Implementation plan: [`.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md`](/.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md)
- Quality strategy: [`.crewchief/projects/IDXSIZE_index-size-limits/planning/quality-strategy.md`](/.crewchief/projects/IDXSIZE_index-size-limits/planning/quality-strategy.md)

**Testing**:
- Phase 1: Automated test suite (IDXSIZE-2001) - 30/30 tests passed
- Phase 2: Query performance tests (IDXSIZE-2002) - 17/17 tests passed
- Phase 2: Production clone test documentation (IDXSIZE-2003) - Complete
- Phase 2: Test execution validation (IDXSIZE-2004) - All tests passed
- Phase 3: Migration execution (IDXSIZE-3002) - Verified successful
- Phase 3: Post-deployment monitoring (IDXSIZE-3003) - System healthy

**Status**: ✅ **SUCCESSFUL** - Migration deployed, verified, and monitoring confirms system health

---

## Migration Template

Use this template for documenting future migrations:

```markdown
### Migration XXXX: [Title] (YYYY-MM-DD)

**Migration File**: `crates/maproom/migrations/XXXX_description.sql`

**Problem**:
[What issue prompted this migration? Include error messages if applicable]

**Solution**:
[How does this migration solve the problem? Include technical approach]

**Index/Schema Changes**:
- **Dropped**: [What was removed]
- **Created**: [What was added]
- **Modified**: [What was changed]

**Impact Analysis**:
- Storage: [Size impact, before/after]
- Performance: [Query performance changes, benchmarks]
- Compatibility: [Breaking changes, data loss risks]

**Deployment**:
- Date: [YYYY-MM-DD]
- Environment: [Development/Staging/Production]
- Duration: [How long it took]
- Downtime: [Any service interruption]

**Monitoring Results**:
[Key metrics from post-deployment monitoring]

**Rollback Procedure**:
[If rollback available, describe procedure. If not recommended, explain why]

**Planning Documentation**:
[Links to planning docs, tickets, architecture decisions]

**Testing**:
[Summary of test coverage and results]

**Status**: [In Progress / Successful / Issues Found]
```

## Guidelines

1. **Document immediately**: Add entries as soon as migration is deployed
2. **Include actual metrics**: Use real data from monitoring, not estimates
3. **Link to context**: Reference planning docs, tickets, and migration files
4. **Explain trade-offs**: Document why decisions were made
5. **Future-proof**: Write for developers who weren't involved in the project
6. **Be honest**: Document issues and learnings, not just successes

## References

- PostgreSQL migration files: [`crates/maproom/migrations/`](/workspace/crates/maproom/migrations/)
- Rollback scripts: [`crates/maproom/migrations/rollback/`](/workspace/crates/maproom/migrations/rollback/)
- Database architecture: [`docs/architecture/DATABASE_ARCHITECTURE.md`](/workspace/docs/architecture/DATABASE_ARCHITECTURE.md)
- Active projects: [`.crewchief/projects/`](/.crewchief/projects/)
