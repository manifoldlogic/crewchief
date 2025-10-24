# Ticket: HYBRID_SEARCH-1002: Database Vector Preparation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Prepare the PostgreSQL database for hybrid search by ensuring vector columns are properly configured, optimizing ivfflat indices, and running database statistics updates. While the base schema already includes vector columns and indices, this ticket focuses on verifying configuration, optimizing index parameters, and establishing best practices for vector operations.

## Background
The HYBRID_SEARCH project implements a state-of-the-art hybrid retrieval system combining full-text search, vector similarity, and graph signals. The database layer must be optimized for efficient vector operations to support <50ms p95 latency requirements and >80% recall targets.

The existing schema (from `0001_init.sql`) already includes:
- `code_embedding VECTOR(1536)` and `text_embedding VECTOR(1536)` columns on `maproom.chunks` table
- ivfflat indices with lists=200 on both embedding columns

This ticket ensures these configurations are production-ready and optimized according to the architecture specifications.

**Planning Context:**
- Architecture: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`
- Plan: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_PLAN.md` (Phase 1, Week 1, Task 2)

## Acceptance Criteria
- [ ] Vector columns verified to exist on chunks table with correct dimensions (1536)
- [ ] ivfflat indices created with optimal parameters (lists=200, probes=10)
- [ ] Index configuration verified using `EXPLAIN ANALYZE` on sample queries
- [ ] ANALYZE run on chunks table to update query planner statistics
- [ ] Partial indices created for performance optimization (high recency_score, high churn_score)
- [ ] Database configuration documented with recommended settings
- [ ] Performance baseline established for vector similarity queries

## Technical Requirements
- PostgreSQL with pgvector extension installed and enabled
- Vector dimensions: 1536 (matching text-embedding-3-small model from HYBRID_SEARCH-1001)
- ivfflat index configuration:
  - `lists`: 200 (number of clusters for index)
  - `probes`: 10 (runtime parameter for search accuracy/speed tradeoff)
- Distance metric: Cosine similarity (`vector_cosine_ops`)
- Partial indices for common filter patterns:
  - Recent chunks: `recency_score > 0.5`
  - High churn chunks: `churn_score > 10`
- Query optimization:
  - Ensure vector operations use indices (verify with EXPLAIN)
  - Set appropriate `ivfflat.probes` parameter at session or database level

## Implementation Notes

### Current Schema State
The base schema already includes:
```sql
-- From 0001_init.sql
code_embedding VECTOR(1536)
text_embedding VECTOR(1536)

CREATE INDEX IF NOT EXISTS idx_chunks_code_vec
  ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX IF NOT EXISTS idx_chunks_text_vec
  ON maproom.chunks USING ivfflat (text_embedding vector_cosine_ops)
  WITH (lists = 200);
```

### Required Enhancements

1. **Verify pgvector Extension**
   - Confirm extension is installed: `CREATE EXTENSION IF NOT EXISTS vector;`
   - Check version compatibility

2. **Configure Runtime Parameters**
   - Set `ivfflat.probes = 10` for optimal accuracy/speed balance
   - Can be set at database, session, or query level
   - Document recommended settings

3. **Create Partial Indices**
   According to architecture document (lines 386-392):
   ```sql
   CREATE INDEX idx_chunks_recent
     ON maproom.chunks (recency_score)
     WHERE recency_score > 0.5;

   CREATE INDEX idx_chunks_high_churn
     ON maproom.chunks (churn_score)
     WHERE churn_score > 10;
   ```

4. **Update Statistics**
   ```sql
   ANALYZE maproom.chunks;
   ANALYZE maproom.files;
   ANALYZE maproom.chunk_edges;
   ```

5. **Verify Index Usage**
   - Test sample vector queries with EXPLAIN ANALYZE
   - Ensure indices are being used (not sequential scans)
   - Document query patterns that trigger index usage

6. **Performance Baseline**
   - Measure query latency for vector similarity searches
   - Test with different `probes` values (1, 5, 10, 20)
   - Document accuracy vs. speed tradeoffs

### Migration Strategy
Create new migration file: `0004_optimize_vector_indices.sql`
- Add partial indices
- Set recommended database parameters
- Update statistics
- Include verification queries

### Testing Approach
- Create sample embedding vectors for testing
- Run vector similarity queries: `code_embedding <=> $1::vector`
- Verify EXPLAIN plans show index usage
- Measure query latency with different probes settings
- Test with NULL and non-NULL embedding filters

### Architecture References
From HYBRID_SEARCH_ARCHITECTURE.md:
- Vector Search section (lines 150-180): Shows query patterns using `<=>` operator
- Database Optimizations section (lines 381-425): Partial indices and ANALYZE commands
- Configuration section (lines 290-293): ivfflat_lists=200, ivfflat_probes=10

## Dependencies
- **Prerequisite**: HYBRID_SEARCH-1001 (Embedding Service Setup) should be completed or in progress to understand embedding dimensions
- **External**: PostgreSQL with pgvector extension must be installed
- **External**: Database must have sufficient resources for index creation (CPU, memory)

## Risk Assessment
- **Risk**: ivfflat index creation may be slow on large datasets
  - **Mitigation**: Indices already exist; this ticket focuses on optimization and validation

- **Risk**: Incorrect probes setting could impact query performance
  - **Mitigation**: Test multiple probes values and document tradeoffs; default=10 is recommended balance

- **Risk**: Vector index may not be used if queries don't match expected patterns
  - **Mitigation**: Use EXPLAIN ANALYZE to verify index usage; document proper query syntax

- **Risk**: Statistics may be outdated after bulk embedding generation
  - **Mitigation**: Run ANALYZE after any bulk data operations; consider automatic ANALYZE triggers

## Files/Packages Affected
- `crates/maproom/migrations/0004_optimize_vector_indices.sql` (NEW) - Optimization migration
- `crates/maproom/migrations/0001_init.sql` (REFERENCE ONLY) - Existing schema with vector columns
- `crates/maproom/src/db/` - May need query adjustments to leverage indices
- Documentation of database configuration and tuning parameters
