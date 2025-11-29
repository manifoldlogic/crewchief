# Implementation Plan: Content-Addressed Chunk Storage

## Project Overview

**Goal**: Implement content-addressed chunk storage using Git blob SHA to enable embedding deduplication

**Success Metrics**:
- Zero data loss during migration
- 70-90% deduplication rate (typical branch overlap)
- Query performance within 10% of baseline
- Cache hit rate measurable and accurate

## Execution Strategy

### Phase-Based Approach

This project follows a **strict sequential migration pattern**:

1. **Foundation** - Add blob SHA computation
2. **Separation** - Create code_embeddings table
3. **Integration** - Update application queries
4. **Cleanup** - Remove old embedding column

Each phase must complete and pass tests before moving to the next.

## Phase 1: Blob SHA Foundation (Week 1, Days 1-2)

### Objectives

- Implement blob SHA computation in Rust and PostgreSQL
- Add blob_sha column to chunks table
- Backfill existing chunks with blob SHA values

### Implementation Steps

#### Step 1.1: Implement Rust Blob SHA Function

**File**: `crates/maproom/src/content_hash.rs` (new file)

**Agent**: rust-indexer-engineer

**Tasks**:
1. Create content_hash module
2. Implement `compute_blob_sha()` function
3. Write comprehensive unit tests
4. Add to lib.rs exports

**Acceptance Criteria**:
- Function produces deterministic output
- Compatible with Git's blob SHA algorithm
- All unit tests pass (100% coverage)

#### Step 1.2: Implement PostgreSQL Blob SHA Function

**File**: `packages/maproom-mcp/migrations/001_add_blob_sha.sql` (new)

**Agent**: database-engineer

**Tasks**:
1. Create SQL function `compute_git_blob_sha()`
2. Test PostgreSQL function matches Rust output
3. Create migration script structure

**Acceptance Criteria**:
- SQL function produces identical output to Rust
- Function is IMMUTABLE (cacheable)
- Integration test verifies Rust/SQL compatibility

#### Step 1.3: Add blob_sha Column

**File**: `packages/maproom-mcp/migrations/001_add_blob_sha.sql`

**Agent**: database-engineer

**SQL**:
```sql
-- Add nullable column
ALTER TABLE chunks ADD COLUMN blob_sha TEXT;

-- Create index (CONCURRENTLY to avoid blocking)
CREATE INDEX CONCURRENTLY idx_chunks_blob_sha ON chunks(blob_sha);
```

**Acceptance Criteria**:
- Column added successfully
- Index created without blocking queries
- No downtime

#### Step 1.4: Backfill Blob SHA Values

**File**: `packages/maproom-mcp/migrations/001_add_blob_sha.sql`

**Agent**: database-engineer

**SQL**:
```sql
-- Backfill in batches
DO $$
DECLARE
  batch_size INT := 1000;
  rows_updated INT;
BEGIN
  LOOP
    UPDATE chunks
    SET blob_sha = compute_git_blob_sha(content)
    WHERE chunk_id IN (
      SELECT chunk_id FROM chunks
      WHERE blob_sha IS NULL
      LIMIT batch_size
    );

    GET DIAGNOSTICS rows_updated = ROW_COUNT;
    EXIT WHEN rows_updated = 0;

    RAISE NOTICE 'Updated % rows', rows_updated;
    COMMIT;
  END LOOP;
END $$;

-- Make NOT NULL after backfill
ALTER TABLE chunks ALTER COLUMN blob_sha SET NOT NULL;
```

**Acceptance Criteria**:
- All chunks have blob_sha values
- No NULL values remain
- Deduplication potential measured

**Validation Query**:
```sql
SELECT
  COUNT(*) AS total_chunks,
  COUNT(DISTINCT blob_sha) AS unique_blobs,
  ROUND(100.0 * (COUNT(*) - COUNT(DISTINCT blob_sha)) / COUNT(*), 2) AS dedup_pct
FROM chunks;
```

#### Step 1.5: Testing

**Agent**: unit-test-runner

**Tests**:
- Unit: `test_blob_sha_deterministic`
- Unit: `test_blob_sha_git_compatibility`
- Integration: `test_migration_001_success`
- Integration: `test_blob_sha_rust_sql_match`

**Deliverables**:
- All Phase 1 tests passing
- Migration script validated
- Metrics showing deduplication potential

---

## Phase 2: Code Embeddings Table (Week 1, Days 3-4)

### Objectives

- Create code_embeddings table for deduplicated storage
- Migrate existing embeddings
- Establish foreign key relationship

### Implementation Steps

#### Step 2.1: Create code_embeddings Table

**File**: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql` (new)

**Agent**: database-engineer

**SQL**:
```sql
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);
```

**Acceptance Criteria**:
- Table created successfully
- Primary key constraint on blob_sha

#### Step 2.2: Migrate Embeddings

**File**: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`

**Agent**: database-engineer

**SQL**:
```sql
-- Extract unique embeddings (deduplicated)
INSERT INTO code_embeddings (blob_sha, embedding, model_version)
SELECT DISTINCT ON (blob_sha)
  blob_sha,
  embedding,
  'text-embedding-3-small'
FROM chunks
WHERE embedding IS NOT NULL
ORDER BY blob_sha, created_at ASC; -- Keep oldest
```

**Acceptance Criteria**:
- All unique embeddings migrated
- No data loss (all blob_sha values covered)
- Deduplication achieved

**Validation**:
```sql
-- Verify no orphaned chunks
SELECT COUNT(*)
FROM chunks c
LEFT JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE e.blob_sha IS NULL AND c.embedding IS NOT NULL;
-- Expected: 0
```

#### Step 2.3: Create Vector Index

**File**: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`

**Agent**: database-engineer

**SQL**:
```sql
-- HNSW index for vector similarity search
CREATE INDEX idx_embeddings_vector
ON code_embeddings
USING hnsw (embedding vector_cosine_ops);
```

**Acceptance Criteria**:
- Index created successfully
- Query planner uses HNSW index for similarity search

#### Step 2.4: Add Foreign Key Constraint

**File**: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`

**Agent**: database-engineer

**SQL**:
```sql
ALTER TABLE chunks
ADD CONSTRAINT fk_chunks_embedding
FOREIGN KEY (blob_sha) REFERENCES code_embeddings(blob_sha);
```

**Acceptance Criteria**:
- Constraint added successfully
- Referential integrity enforced

#### Step 2.5: Testing

**Agent**: unit-test-runner

**Tests**:
- Integration: `test_migration_002_deduplication`
- Integration: `test_no_embedding_loss`
- Integration: `test_foreign_key_constraint`

**Validation Queries**:
```sql
-- Verify storage savings
SELECT
  pg_size_pretty(pg_total_relation_size('chunks')) AS chunks_size,
  pg_size_pretty(pg_total_relation_size('code_embeddings')) AS embeddings_size;

-- Verify all embeddings accessible
SELECT
  (SELECT COUNT(*) FROM chunks WHERE embedding IS NOT NULL) AS chunks_with_embeddings,
  (SELECT COUNT(*) FROM code_embeddings) AS unique_embeddings;
```

**Deliverables**:
- All Phase 2 tests passing
- Migration verified on test database
- Storage savings measured

---

## Phase 3: Application Integration (Week 1, Day 5)

### Objectives

- Update all queries to use JOIN with code_embeddings
- Implement cache-aware upsert logic
- Verify query equivalence

### Implementation Steps

#### Step 3.1: Update Search Queries

**File**: `crates/maproom/src/search.rs`

**Agent**: rust-indexer-engineer

**Changes**:
```rust
// Before
let chunks = sqlx::query_as!(
    Chunk,
    "SELECT chunk_id, content, embedding FROM chunks WHERE ..."
)
.fetch_all(&pool)
.await?;

// After
let chunks = sqlx::query_as!(
    Chunk,
    r#"
    SELECT c.chunk_id, c.content, e.embedding
    FROM chunks c
    JOIN code_embeddings e ON c.blob_sha = e.blob_sha
    WHERE e.embedding <=> $1 < 0.5
    ORDER BY e.embedding <=> $1
    LIMIT 10
    "#,
    query_embedding
)
.fetch_all(&pool)
.await?;
```

**Acceptance Criteria**:
- All search queries updated
- EXPLAIN ANALYZE shows efficient query plan
- Results identical to old queries

#### Step 3.2: Implement Cache-Aware Upsert

**File**: `crates/maproom/src/upsert.rs`

**Agent**: rust-indexer-engineer

**Logic**:
```rust
async fn upsert_chunk_with_cache(
    pool: &PgPool,
    chunk: &ParsedChunk,
) -> Result<Uuid> {
    let blob_sha = compute_blob_sha(&chunk.content);

    // Check cache
    let embedding_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM code_embeddings WHERE blob_sha = $1)",
        blob_sha
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    if !embedding_exists {
        // Cache miss - generate embedding
        let embedding = generate_embedding(&chunk.content).await?;

        sqlx::query!(
            "INSERT INTO code_embeddings (blob_sha, embedding) VALUES ($1, $2)
             ON CONFLICT (blob_sha) DO NOTHING",
            blob_sha,
            embedding
        )
        .execute(pool)
        .await?;

        info!("Cache miss: generated embedding for {}", blob_sha);
    } else {
        info!("Cache hit: reusing embedding for {}", blob_sha);
    }

    // Insert/update chunk
    let chunk_id = sqlx::query_scalar!(
        "INSERT INTO chunks (blob_sha, file_path, content, ...)
         VALUES ($1, $2, $3, ...)
         ON CONFLICT (chunk_id) DO UPDATE SET ...
         RETURNING chunk_id",
        blob_sha,
        chunk.file_path,
        chunk.content,
    )
    .fetch_one(pool)
    .await?;

    Ok(chunk_id)
}
```

**Acceptance Criteria**:
- Cache hit detection works
- No duplicate embeddings generated
- Metrics track hits/misses

#### Step 3.3: Add Cache Metrics

**File**: `crates/maproom/src/metrics.rs` (new)

**Agent**: rust-indexer-engineer

**Implementation**:
```rust
pub struct CacheMetrics {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl CacheMetrics {
    pub fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }

    pub fn report(&self) {
        info!(
            "Cache metrics - Hits: {}, Misses: {}, Hit rate: {:.2}%",
            self.cache_hits.load(Ordering::Relaxed),
            self.cache_misses.load(Ordering::Relaxed),
            self.hit_rate() * 100.0
        );
    }
}
```

**Acceptance Criteria**:
- Metrics accurately track cache behavior
- Reported at end of scan operations

#### Step 3.4: Testing

**Agent**: unit-test-runner

**Tests**:
- Integration: `test_cache_hit_duplicate_content`
- Integration: `test_cache_miss_unique_content`
- Integration: `test_cache_metrics_accuracy`
- E2E: `test_search_query_equivalence`

**Deliverables**:
- All Phase 3 tests passing
- Cache metrics verified
- Query performance benchmarked

---

## Phase 4: Cleanup and Optimization (Week 2, Day 1)

### Objectives

- Remove old embedding column from chunks table
- Reclaim disk space
- Final performance validation

### Implementation Steps

#### Step 4.1: Drop Embedding Column

**File**: `packages/maproom-mcp/migrations/003_drop_old_embedding.sql` (new)

**Agent**: database-engineer

**Prerequisites**:
- All Phase 3 tests passing
- Manual verification of queries working
- Backup taken

**SQL**:
```sql
-- Drop old embedding column
ALTER TABLE chunks DROP COLUMN embedding;

-- Reclaim space
VACUUM FULL chunks;
```

**Acceptance Criteria**:
- Column dropped successfully
- Disk space reclaimed
- All queries still working

#### Step 4.2: Final Performance Benchmarks

**Agent**: unit-test-runner

**Benchmarks**:
```bash
# Run criterion benchmarks
cargo bench --bench search_performance

# Compare before/after
# - Search latency (should be within 10%)
# - Throughput (queries per second)
```

**Acceptance Criteria**:
- Performance within 10% of baseline
- No regressions

#### Step 4.3: Documentation

**Agent**: general-purpose

**Files to create/update**:
- `docs/architecture/content-addressed-storage.md` - Architecture overview
- `packages/maproom-mcp/README.md` - Update with new schema
- `CHANGELOG.md` - Document changes

**Deliverables**:
- Architecture documented
- Migration guide written
- Changelog updated

---

## Agent Assignments

### Primary Agents

1. **rust-indexer-engineer**
   - Blob SHA computation (Rust)
   - Upsert logic updates
   - Cache metrics
   - Search query updates

2. **database-engineer** (if exists, else general-purpose)
   - Migration scripts
   - PostgreSQL blob SHA function
   - Schema changes
   - Index creation

3. **unit-test-runner**
   - Execute all tests after each phase
   - Report test results
   - No code modifications

4. **verify-ticket**
   - Final verification against acceptance criteria
   - Validate all success metrics

5. **commit-ticket**
   - Create Conventional Commit for completed work

### Agent Workflow Per Phase

```
Phase N:
  1. Implementation agent (rust-indexer-engineer or database-engineer)
     - Write code/migrations
     - Write tests
  2. unit-test-runner
     - Execute tests
     - Report results
  3. If tests fail → return to step 1
  4. If tests pass → proceed to next phase

Final Phase:
  1. verify-ticket
     - Check all acceptance criteria
  2. If any criteria fail → identify failing phase, return to implementation
  3. If all pass → commit-ticket
```

## Testing Strategy

### Per-Phase Testing

**Phase 1**:
- Unit tests for blob SHA
- Migration test (add column)
- Backfill validation

**Phase 2**:
- Migration test (create table)
- Deduplication verification
- Foreign key test

**Phase 3**:
- Cache behavior tests
- Query equivalence tests
- Performance benchmarks

**Phase 4**:
- Final integration test
- Performance validation
- Manual smoke tests

### Critical Path Tests

Run on every commit:
1. `test_blob_sha_deterministic`
2. `test_cache_hit_duplicate_content`
3. `test_search_query_equivalence`
4. `test_no_data_loss`

## Risk Mitigation

### Backup Strategy

**Before Phase 1**:
```bash
pg_dump maproom > backup_before_blobsha_$(date +%Y%m%d).sql
```

**Before Phase 4** (dropping column):
```bash
pg_dump maproom > backup_before_drop_embedding_$(date +%Y%m%d).sql
```

### Rollback Plans

**Phase 1 rollback**:
```sql
ALTER TABLE chunks DROP COLUMN blob_sha;
```

**Phase 2 rollback**:
```sql
ALTER TABLE chunks DROP CONSTRAINT fk_chunks_embedding;
DROP TABLE code_embeddings;
```

**Phase 4 rollback**:
Restore from backup (column can't be easily re-added)

### Monitoring

**During migration**:
- Watch query latency (Grafana/monitoring)
- Monitor disk space
- Track error rates

**After deployment**:
- Verify cache hit rates (should be 70-90% for branch overlaps)
- Monitor storage usage (should decrease)
- Track embedding API costs (should decrease proportionally)

## Success Criteria

### Functional Requirements

- [x] Blob SHA computed for all chunks
- [x] Embeddings deduplicated in code_embeddings table
- [x] Cache hit/miss detection working
- [x] All queries return correct results
- [x] Foreign key integrity enforced

### Performance Requirements

- [x] Query latency within 10% of baseline
- [x] Cache hit rate 70-90% for typical branch overlap
- [x] Migration completes in reasonable time (<1 hour for 1M chunks)

### Quality Requirements

- [x] All unit tests passing
- [x] All integration tests passing
- [x] E2E search equivalence test passing
- [x] Zero data loss verified

### Documentation Requirements

- [x] Architecture documented
- [x] Migration guide written
- [x] Changelog updated

## Timeline

**Week 1**:
- Day 1-2: Phase 1 (Blob SHA foundation)
- Day 3-4: Phase 2 (Code embeddings table)
- Day 5: Phase 3 (Application integration)

**Week 2**:
- Day 1: Phase 4 (Cleanup)
- Day 2: Buffer for issues

**Total**: 6-7 working days

## Next Project Dependencies

This project is a **prerequisite** for:

- **BRANCHX**: Branch-aware indexing (needs blob SHA for deduplication)
- **BRWATCH**: Branch switch detection (needs cache for fast switching)

**Do not start BRANCHX or BRWATCH until BLOBSHA is complete and verified.**

## Acceptance Checklist

Before marking this project complete:

- [ ] All phases complete (1-4)
- [ ] All tests passing (unit + integration + E2E)
- [ ] Performance benchmarks within targets
- [ ] Documentation updated
- [ ] Manual smoke test successful
- [ ] Cache metrics showing expected behavior
- [ ] Deduplication working (embedding count < chunk count)
- [ ] Zero data loss verified
- [ ] Backup taken and tested
- [ ] Rollback procedure documented

**Only after ALL items checked** → Run verify-ticket → commit-ticket
