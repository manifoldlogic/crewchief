# Architecture: Content-Addressed Chunk Storage

## Design Principles

1. **Content is the key**: Use blob SHA as primary identifier for embeddings
2. **Separation of concerns**: Decouple content (chunks) from computed values (embeddings)
3. **Backward compatibility**: Existing queries must work during migration
4. **Zero data loss**: Migration must preserve all existing embeddings
5. **Performance first**: New architecture must be as fast or faster than current

## Database Schema Design

### New Table: code_embeddings

**Purpose**: Deduplicated storage of embeddings, keyed by content hash

```sql
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,           -- Git-compatible blob SHA (SHA-256)
  embedding vector(1536) NOT NULL,     -- OpenAI text-embedding-3-small
  model_version TEXT NOT NULL,         -- 'text-embedding-3-small'
  created_at TIMESTAMP DEFAULT NOW()
);

-- Index for vector similarity search
CREATE INDEX ON code_embeddings USING hnsw (embedding vector_cosine_ops);
```

**Key decisions**:
- `blob_sha` as PRIMARY KEY: Natural deduplication (one embedding per unique content)
- `model_version`: Enables model upgrades (invalidate old embeddings)
- `created_at`: Auditing and metrics (when was embedding first cached)

### Modified Table: code_chunks

**Changes**: Add blob_sha column, remove embedding column (later phase)

```sql
-- Phase 1: Add blob_sha
ALTER TABLE chunks ADD COLUMN blob_sha TEXT;
CREATE INDEX ON chunks(blob_sha);

-- Phase 2: Backfill blob_sha
UPDATE chunks
SET blob_sha = compute_git_blob_sha(content);

ALTER TABLE chunks ALTER COLUMN blob_sha SET NOT NULL;

-- Phase 3: Add foreign key
ALTER TABLE chunks
ADD CONSTRAINT fk_embedding
FOREIGN KEY (blob_sha) REFERENCES code_embeddings(blob_sha);

-- Phase 4: Drop embedding column (space savings)
-- ONLY after verifying queries work with JOIN
ALTER TABLE chunks DROP COLUMN embedding;
```

**Key decisions**:
- Keep `embedding` column during migration: Backward compatibility
- Add index on `blob_sha`: Fast JOIN performance
- Foreign key constraint: Referential integrity (can't delete embedding still in use)

## Blob SHA Computation

### Algorithm Implementation

**Rust implementation** (in `crates/maproom/src/content_hash.rs`):

```rust
use sha2::{Sha256, Digest};

/// Compute Git-compatible blob SHA for content
///
/// Format: SHA256("blob <size>\0<content>")
/// Compatible with: git hash-object
pub fn compute_blob_sha(content: &str) -> String {
    let mut hasher = Sha256::new();

    // Git blob header: "blob <size>\0"
    hasher.update(b"blob ");
    hasher.update(content.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());

    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_sha_deterministic() {
        let content = "function foo() { return 1; }";
        let sha1 = compute_blob_sha(content);
        let sha2 = compute_blob_sha(content);
        assert_eq!(sha1, sha2);
    }

    #[test]
    fn test_blob_sha_different_content() {
        let content1 = "function foo() { return 1; }";
        let content2 = "function bar() { return 2; }";
        assert_ne!(compute_blob_sha(content1), compute_blob_sha(content2));
    }

    #[test]
    fn test_blob_sha_whitespace_sensitive() {
        // Content addressing is bit-for-bit identical
        let content1 = "function foo() { return 1; }";
        let content2 = "function foo() { return 1;  }"; // Extra space
        assert_ne!(compute_blob_sha(content1), compute_blob_sha(content2));
    }
}
```

### PostgreSQL Function

**SQL implementation** (for migration):

```sql
CREATE OR REPLACE FUNCTION compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(
    digest(
      'blob ' || length(content) || E'\0' || content,
      'sha256'
    ),
    'hex'
  );
$$ LANGUAGE SQL IMMUTABLE;
```

**Usage**:
```sql
-- Compute blob SHA for existing chunks
UPDATE chunks
SET blob_sha = compute_git_blob_sha(content);
```

## Migration Strategy

### Phase 1: Add Blob SHA Column (Week 1, Day 1-2)

**Goal**: Add blob_sha to chunks without disrupting service

```sql
-- 1. Add nullable column
ALTER TABLE chunks ADD COLUMN blob_sha TEXT;

-- 2. Create index (may take time for large tables)
CREATE INDEX CONCURRENTLY idx_chunks_blob_sha ON chunks(blob_sha);

-- 3. Backfill in batches (avoid long-running transaction)
DO $$
DECLARE
  batch_size INT := 1000;
  total INT;
  processed INT := 0;
BEGIN
  SELECT COUNT(*) INTO total FROM chunks WHERE blob_sha IS NULL;

  WHILE processed < total LOOP
    UPDATE chunks
    SET blob_sha = compute_git_blob_sha(content)
    WHERE chunk_id IN (
      SELECT chunk_id FROM chunks
      WHERE blob_sha IS NULL
      LIMIT batch_size
    );

    processed := processed + batch_size;
    RAISE NOTICE 'Processed % of % chunks', processed, total;
    COMMIT;
  END LOOP;
END $$;

-- 4. Make it NOT NULL after backfill
ALTER TABLE chunks ALTER COLUMN blob_sha SET NOT NULL;
```

**Validation**:
```sql
-- Ensure no NULLs
SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL;
-- Expected: 0

-- Check deduplication potential
SELECT
  COUNT(*) AS total_chunks,
  COUNT(DISTINCT blob_sha) AS unique_chunks,
  COUNT(*) - COUNT(DISTINCT blob_sha) AS duplicates,
  ROUND(100.0 * (COUNT(*) - COUNT(DISTINCT blob_sha)) / COUNT(*), 2) AS dedup_percentage
FROM chunks;
```

### Phase 2: Create Embedding Table (Week 1, Day 3-4)

**Goal**: Extract embeddings into deduplicated table

```sql
-- 1. Create table
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

-- 2. Migrate existing embeddings (deduplicated)
INSERT INTO code_embeddings (blob_sha, embedding, model_version)
SELECT DISTINCT ON (blob_sha)
  blob_sha,
  embedding,
  'text-embedding-3-small' -- Current model
FROM chunks
WHERE embedding IS NOT NULL
ORDER BY blob_sha, created_at ASC; -- Keep oldest embedding

-- 3. Create HNSW index for vector search
CREATE INDEX idx_embeddings_vector
ON code_embeddings
USING hnsw (embedding vector_cosine_ops);

-- 4. Add foreign key
ALTER TABLE chunks
ADD CONSTRAINT fk_chunks_embedding
FOREIGN KEY (blob_sha) REFERENCES code_embeddings(blob_sha);
```

**Validation**:
```sql
-- Verify all chunks have embeddings
SELECT COUNT(*)
FROM chunks c
LEFT JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE e.blob_sha IS NULL;
-- Expected: 0

-- Verify deduplication worked
SELECT
  (SELECT COUNT(*) FROM chunks) AS total_chunks,
  (SELECT COUNT(*) FROM code_embeddings) AS unique_embeddings,
  ROUND(100.0 * (SELECT COUNT(*) FROM code_embeddings) / (SELECT COUNT(*) FROM chunks), 2) AS cache_efficiency
FROM dual;
```

### Phase 3: Update Application Queries (Week 1, Day 5)

**Goal**: Modify queries to use JOIN instead of direct embedding access

**Before**:
```rust
// Query embeddings directly from chunks
let chunks = sqlx::query_as!(
    Chunk,
    r#"
    SELECT chunk_id, content, embedding
    FROM chunks
    WHERE embedding <=> $1 < 0.5
    ORDER BY embedding <=> $1
    LIMIT 10
    "#,
    query_embedding
)
.fetch_all(&pool)
.await?;
```

**After**:
```rust
// Join with code_embeddings
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

**Files to update**:
- `crates/maproom/src/search.rs` - Vector search queries
- `crates/maproom/src/upsert.rs` - Chunk insertion logic
- `packages/maproom-mcp/src/search.ts` - MCP search handler

### Phase 4: Drop Old Embedding Column (Week 2, Day 1)

**Goal**: Reclaim storage space

```sql
-- 1. Verify queries work without embedding column
-- Run test suite, manual testing

-- 2. Drop the column
ALTER TABLE chunks DROP COLUMN embedding;

-- 3. VACUUM to reclaim space
VACUUM FULL chunks;
```

**Expected savings**:
- Embedding size: 1536 floats × 4 bytes = 6KB per embedding
- If 50% deduplication: 50% × 6KB × chunk_count = massive savings

## Query Performance

### Indexing Strategy

**Before** (embedding in chunks table):
```sql
-- HNSW index on chunks.embedding
CREATE INDEX ON chunks USING hnsw (embedding vector_cosine_ops);
```

**After** (embedding in code_embeddings table):
```sql
-- HNSW index on code_embeddings.embedding
CREATE INDEX ON code_embeddings USING hnsw (embedding vector_cosine_ops);

-- B-tree index for JOIN
CREATE INDEX ON chunks(blob_sha);
```

**Performance considerations**:
- HNSW index is smaller (fewer unique embeddings)
- JOIN overhead is minimal (indexed primary key → foreign key)
- Overall performance should be equal or better

### Benchmark Queries

**Test query** (find similar code):
```sql
EXPLAIN ANALYZE
SELECT c.chunk_id, c.content, c.symbol_name, e.embedding <=> $1 AS distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE e.embedding <=> $1 < 0.5
ORDER BY distance
LIMIT 10;
```

**Expected plan**:
1. Index scan on `code_embeddings` (HNSW) to find candidates
2. Index nested loop join on `chunks.blob_sha`
3. Sort and limit

**Success criteria**: Query time within 10% of baseline

## Embedding Cache Logic

### Upsert Workflow

**New chunk insertion logic**:

```rust
async fn upsert_chunk(
    pool: &PgPool,
    chunk: &ParsedChunk,
) -> Result<Uuid> {
    // 1. Compute blob SHA
    let blob_sha = compute_blob_sha(&chunk.content);

    // 2. Check if embedding exists
    let embedding_exists = sqlx::query!(
        "SELECT 1 FROM code_embeddings WHERE blob_sha = $1",
        blob_sha
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    // 3. Generate embedding if needed
    if !embedding_exists {
        let embedding = generate_embedding(&chunk.content).await?;

        sqlx::query!(
            r#"
            INSERT INTO code_embeddings (blob_sha, embedding, model_version)
            VALUES ($1, $2, 'text-embedding-3-small')
            ON CONFLICT (blob_sha) DO NOTHING
            "#,
            blob_sha,
            embedding
        )
        .execute(pool)
        .await?;

        info!("Generated new embedding for blob_sha: {}", blob_sha);
    } else {
        info!("Cache hit for blob_sha: {}", blob_sha);
    }

    // 4. Insert or update chunk
    let chunk_id = sqlx::query_scalar!(
        r#"
        INSERT INTO chunks (blob_sha, file_path, symbol_name, content, ...)
        VALUES ($1, $2, $3, $4, ...)
        ON CONFLICT (chunk_id) DO UPDATE
        SET blob_sha = EXCLUDED.blob_sha, updated_at = NOW()
        RETURNING chunk_id
        "#,
        blob_sha,
        chunk.file_path,
        chunk.symbol_name,
        chunk.content,
        // ...
    )
    .fetch_one(pool)
    .await?;

    Ok(chunk_id)
}
```

### Cache Metrics

**Track cache effectiveness**:

```rust
struct CacheMetrics {
    cache_hits: u64,
    cache_misses: u64,
    hit_rate: f64,
}

impl CacheMetrics {
    fn record_hit(&mut self) {
        self.cache_hits += 1;
        self.update_hit_rate();
    }

    fn record_miss(&mut self) {
        self.cache_misses += 1;
        self.update_hit_rate();
    }

    fn update_hit_rate(&mut self) {
        let total = self.cache_hits + self.cache_misses;
        self.hit_rate = if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        };
    }
}
```

**Log at end of scan**:
```
[INFO] Indexing complete:
  - Chunks processed: 10,000
  - Cache hits: 8,000 (80%)
  - Cache misses: 2,000 (20%)
  - Embeddings generated: 2,000
  - Estimated cost: $0.04
```

## Model Versioning

**When upgrading embedding model**:

```sql
-- Option 1: Invalidate all old embeddings
DELETE FROM code_embeddings WHERE model_version != 'text-embedding-3-small-v2';

-- Option 2: Lazy regeneration
-- Keep old embeddings, generate new ones on demand
-- Track version in code_embeddings.model_version
```

**For this MVP**: Don't implement version migration yet. Assume single model version.

## Rollback Plan

If migration fails:

```sql
-- 1. Drop foreign key
ALTER TABLE chunks DROP CONSTRAINT fk_chunks_embedding;

-- 2. Drop new table
DROP TABLE code_embeddings;

-- 3. Drop blob_sha column
ALTER TABLE chunks DROP COLUMN blob_sha;

-- 4. Restore from backup if needed
```

**Critical**: Take database backup before Phase 1

## Technology Choices

### Why SHA-256?

- **Cryptographically secure**: Collision probability negligible (2^-256)
- **Git compatibility**: Standard blob hash algorithm
- **Performance**: Fast computation (~1μs per chunk)
- **Standard**: Supported by PostgreSQL, Rust, everywhere

### Why Separate Table?

**Alternatives considered**:

1. **Keep embeddings in chunks table**
   - Con: No deduplication
   - Con: Redundant storage

2. **Materialized view**
   - Con: Complex refresh logic
   - Con: Still stores duplicates

3. **Separate table** ✅
   - Pro: Natural deduplication via PRIMARY KEY
   - Pro: Clean separation of concerns
   - Pro: Easy to version (model_version column)

## Success Metrics

### Performance Targets

- Query latency: Within 10% of baseline
- Index scan time: < 50ms for top 10 results
- JOIN overhead: < 5ms

### Storage Targets

- Deduplication rate: 70-90% (based on typical branch overlap)
- Disk space savings: 50%+ after VACUUM

### Cost Targets

- Embedding API calls: Reduced by dedup percentage
- Example: 80% overlap → 80% cost savings

## Next Steps

1. Implement blob SHA computation (Rust + SQL)
2. Create migration scripts
3. Write comprehensive tests (quality-strategy.md)
4. Plan rollout strategy (plan.md)
