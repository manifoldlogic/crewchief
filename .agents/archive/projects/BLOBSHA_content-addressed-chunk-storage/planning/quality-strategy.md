# Quality Strategy: Content-Addressed Chunk Storage

## Testing Philosophy

**Core principle**: Prevent data loss and ensure cache correctness.

This is a **migration-heavy project** touching core data structures. Testing must verify:

1. **Data integrity**: No embeddings lost during migration
2. **Cache correctness**: Identical content → identical blob SHA → cache hit
3. **Query equivalence**: New queries return same results as old queries
4. **Performance**: No regression in search speed

**Not testing**:
- Embedding quality (out of scope, OpenAI's responsibility)
- Tree-sitter parsing (tested elsewhere)
- Branch tracking (BRANCHX project)

## Test Pyramid

```
        🔺
       E2E (5%)
      -------
     Integration (25%)
    -------
   Unit Tests (70%)
```

### Unit Tests (70%)

**Focus**: Pure functions, deterministic behavior

#### 1. Blob SHA Computation Tests

**File**: `crates/maproom/src/content_hash.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_sha_deterministic() {
        let content = "function foo() { return 1; }";
        let sha1 = compute_blob_sha(content);
        let sha2 = compute_blob_sha(content);
        assert_eq!(sha1, sha2, "Same content must produce same SHA");
    }

    #[test]
    fn test_blob_sha_different_content() {
        let content1 = "function foo() { return 1; }";
        let content2 = "function bar() { return 2; }";
        assert_ne!(
            compute_blob_sha(content1),
            compute_blob_sha(content2),
            "Different content must produce different SHA"
        );
    }

    #[test]
    fn test_blob_sha_whitespace_sensitivity() {
        // Blob SHA is bit-for-bit identical check
        let content1 = "function foo() { return 1; }";
        let content2 = "function foo() { return 1;  }"; // Extra space
        assert_ne!(
            compute_blob_sha(content1),
            compute_blob_sha(content2),
            "Even whitespace changes must produce different SHA"
        );
    }

    #[test]
    fn test_blob_sha_empty_content() {
        let empty = "";
        let sha = compute_blob_sha(empty);
        assert!(!sha.is_empty(), "Empty content should have valid SHA");
    }

    #[test]
    fn test_blob_sha_unicode() {
        let unicode = "function émoji() { return '🎉'; }";
        let sha = compute_blob_sha(unicode);
        assert_eq!(sha.len(), 64, "SHA should be 64 hex chars (256 bits)");
    }

    #[test]
    fn test_blob_sha_git_compatibility() {
        // Verify compatibility with `git hash-object`
        // Can manually test with: echo -n "test" | git hash-object --stdin
        let content = "test";
        let our_sha = compute_blob_sha(content);
        let git_sha = "9daeafb9864cf43055ae93beb0afd6c7d144bfa4"; // From git hash-object
        assert_eq!(our_sha, git_sha, "Must match Git's blob SHA");
    }
}
```

**Coverage goal**: 100% of `compute_blob_sha` function

#### 2. PostgreSQL Function Tests

**File**: `packages/maproom-mcp/tests/blob-sha.test.ts`

```typescript
import { describe, it, expect, beforeAll } from 'vitest';
import { getPool } from '../src/db';

describe('PostgreSQL blob SHA function', () => {
  let pool: any;

  beforeAll(async () => {
    pool = await getPool();
  });

  it('computes blob SHA for simple content', async () => {
    const result = await pool.query(
      "SELECT compute_git_blob_sha('test') AS sha"
    );
    expect(result.rows[0].sha).toBe(
      '9daeafb9864cf43055ae93beb0afd6c7d144bfa4'
    );
  });

  it('matches Rust implementation', async () => {
    const content = 'function foo() { return 1; }';
    const sqlResult = await pool.query(
      'SELECT compute_git_blob_sha($1) AS sha',
      [content]
    );
    const rustResult = computeBlobSha(content); // Call Rust via FFI or CLI

    expect(sqlResult.rows[0].sha).toBe(rustResult);
  });

  it('handles empty content', async () => {
    const result = await pool.query(
      "SELECT compute_git_blob_sha('') AS sha"
    );
    expect(result.rows[0].sha).toHaveLength(64);
  });

  it('handles unicode content', async () => {
    const content = '🎉';
    const result = await pool.query(
      'SELECT compute_git_blob_sha($1) AS sha',
      [content]
    );
    expect(result.rows[0].sha).toHaveLength(64);
  });
});
```

**Coverage goal**: PostgreSQL function works identically to Rust

### Integration Tests (25%)

**Focus**: Database operations, cache behavior

#### 3. Migration Tests

**File**: `packages/maproom-mcp/tests/migration.test.ts`

```typescript
describe('Migration: Add blob_sha column', () => {
  it('adds blob_sha to all existing chunks', async () => {
    // Setup: Create test chunks
    await createTestChunk({
      content: 'function foo() { return 1; }',
    });

    // Run migration
    await runMigration('001_add_blob_sha');

    // Verify
    const result = await pool.query(
      'SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL'
    );
    expect(result.rows[0].count).toBe('0');
  });

  it('computes correct blob SHA during backfill', async () => {
    const content = 'test content';
    const expectedSha = computeBlobSha(content);

    await createTestChunk({ content });
    await runMigration('001_add_blob_sha');

    const result = await pool.query(
      'SELECT blob_sha FROM chunks WHERE content = $1',
      [content]
    );
    expect(result.rows[0].blob_sha).toBe(expectedSha);
  });

  it('handles large batch migration', async () => {
    // Create 10,000 test chunks
    for (let i = 0; i < 10000; i++) {
      await createTestChunk({ content: `function f${i}() {}` });
    }

    // Run migration
    const start = Date.now();
    await runMigration('001_add_blob_sha');
    const duration = Date.now() - start;

    // Should complete in reasonable time
    expect(duration).toBeLessThan(60000); // < 1 minute

    // Verify all migrated
    const result = await pool.query(
      'SELECT COUNT(*) FROM chunks WHERE blob_sha IS NOT NULL'
    );
    expect(result.rows[0].count).toBe('10000');
  });
});

describe('Migration: Create code_embeddings table', () => {
  it('extracts unique embeddings from chunks', async () => {
    // Setup: Create chunks with duplicate content
    const content = 'function foo() {}';
    await createTestChunk({ content, embedding: [0.1, 0.2, 0.3] });
    await createTestChunk({ content, embedding: [0.1, 0.2, 0.3] }); // Duplicate

    // Run migration
    await runMigration('002_create_code_embeddings');

    // Verify: Only 1 embedding stored
    const result = await pool.query('SELECT COUNT(*) FROM code_embeddings');
    expect(result.rows[0].count).toBe('1');
  });

  it('preserves all embeddings during migration', async () => {
    // Setup: Create N unique chunks
    const n = 100;
    for (let i = 0; i < n; i++) {
      await createTestChunk({
        content: `function f${i}() {}`,
        embedding: Array(1536).fill(i / n),
      });
    }

    // Run migration
    await runMigration('002_create_code_embeddings');

    // Verify: All N embeddings present
    const result = await pool.query('SELECT COUNT(*) FROM code_embeddings');
    expect(result.rows[0].count).toBe(n.toString());
  });

  it('creates foreign key constraint', async () => {
    await runMigration('002_create_code_embeddings');

    // Verify constraint exists
    const result = await pool.query(`
      SELECT constraint_name
      FROM information_schema.table_constraints
      WHERE table_name = 'chunks'
        AND constraint_type = 'FOREIGN KEY'
        AND constraint_name = 'fk_chunks_embedding'
    `);

    expect(result.rows).toHaveLength(1);
  });
});
```

**Coverage goal**: All migration phases tested with realistic data

#### 4. Cache Behavior Tests

**File**: `crates/maproom/tests/cache_tests.rs`

```rust
#[tokio::test]
async fn test_cache_hit_duplicate_content() {
    let pool = get_test_pool().await;

    let chunk1 = ParsedChunk {
        content: "function foo() { return 1; }".to_string(),
        symbol_name: "foo".to_string(),
        file_path: "file1.ts".to_string(),
        // ...
    };

    let chunk2 = ParsedChunk {
        content: "function foo() { return 1; }".to_string(), // Same content
        symbol_name: "foo".to_string(),
        file_path: "file2.ts".to_string(), // Different file
        // ...
    };

    // Insert first chunk (cache miss)
    upsert_chunk(&pool, &chunk1).await.unwrap();
    let embedding_count_1 = count_embeddings(&pool).await.unwrap();

    // Insert second chunk (should be cache hit)
    upsert_chunk(&pool, &chunk2).await.unwrap();
    let embedding_count_2 = count_embeddings(&pool).await.unwrap();

    // Should not generate new embedding
    assert_eq!(embedding_count_1, embedding_count_2);
}

#[tokio::test]
async fn test_cache_miss_different_content() {
    let pool = get_test_pool().await;

    let chunk1 = ParsedChunk {
        content: "function foo() { return 1; }".to_string(),
        // ...
    };

    let chunk2 = ParsedChunk {
        content: "function bar() { return 2; }".to_string(), // Different
        // ...
    };

    upsert_chunk(&pool, &chunk1).await.unwrap();
    let count1 = count_embeddings(&pool).await.unwrap();

    upsert_chunk(&pool, &chunk2).await.unwrap();
    let count2 = count_embeddings(&pool).await.unwrap();

    // Should generate new embedding
    assert_eq!(count2, count1 + 1);
}

#[tokio::test]
async fn test_cache_metrics() {
    let pool = get_test_pool().await;
    let mut metrics = CacheMetrics::new();

    // Insert unique chunks
    for i in 0..10 {
        let chunk = create_test_chunk(&format!("function f{i}() {{}}")!);
        upsert_chunk_with_metrics(&pool, &chunk, &mut metrics).await.unwrap();
    }

    // Insert duplicates
    for i in 0..10 {
        let chunk = create_test_chunk(&format!("function f{i}() {{}}")!);
        upsert_chunk_with_metrics(&pool, &chunk, &mut metrics).await.unwrap();
    }

    // 10 misses, 10 hits = 50% hit rate
    assert_eq!(metrics.cache_hits, 10);
    assert_eq!(metrics.cache_misses, 10);
    assert_eq!(metrics.hit_rate, 0.5);
}
```

**Coverage goal**: All cache scenarios covered (hit, miss, metrics)

### E2E Tests (5%)

**Focus**: End-to-end workflows with realistic data

#### 5. Search Query Equivalence Test

**File**: `packages/maproom-mcp/tests/e2e/search-equivalence.test.ts`

```typescript
describe('Search query equivalence', () => {
  it('returns same results before and after migration', async () => {
    const pool = await getPool();

    // Setup: Index test repository
    await indexRepository('./fixtures/test-repo');

    // Query before migration (direct embedding)
    const beforeResults = await pool.query(`
      SELECT chunk_id, symbol_name, content
      FROM chunks
      WHERE embedding <=> $1 < 0.5
      ORDER BY embedding <=> $1
      LIMIT 10
    `, [queryEmbedding]);

    // Run migration
    await runAllMigrations();

    // Query after migration (JOIN with code_embeddings)
    const afterResults = await pool.query(`
      SELECT c.chunk_id, c.symbol_name, c.content
      FROM chunks c
      JOIN code_embeddings e ON c.blob_sha = e.blob_sha
      WHERE e.embedding <=> $1 < 0.5
      ORDER BY e.embedding <=> $1
      LIMIT 10
    `, [queryEmbedding]);

    // Results should be identical
    expect(afterResults.rows).toEqual(beforeResults.rows);
  });
});
```

#### 6. Performance Regression Test

**File**: `crates/maproom/benches/search_performance.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_search_before_migration(c: &mut Criterion) {
    c.bench_function("search_before_migration", |b| {
        b.iter(|| {
            // Query chunks table directly
            search_chunks_direct(black_box(&query_embedding))
        });
    });
}

fn benchmark_search_after_migration(c: &mut Criterion) {
    c.bench_function("search_after_migration", |b| {
        b.iter(|| {
            // Query with JOIN to code_embeddings
            search_chunks_with_join(black_box(&query_embedding))
        });
    });
}

criterion_group!(
    benches,
    benchmark_search_before_migration,
    benchmark_search_after_migration
);
criterion_main!(benches);
```

**Success criteria**: After migration query time within 10% of before

## Critical Path Testing

**Most important tests** (run on every commit):

1. ✅ `test_blob_sha_deterministic` - Cache relies on this
2. ✅ `test_cache_hit_duplicate_content` - Core deduplication behavior
3. ✅ `test_migration_no_data_loss` - Zero data loss requirement
4. ✅ `test_search_equivalence` - Backward compatibility

**If any of these fail**: Stop and fix before proceeding

## Test Data Strategy

### Fixtures

**Directory**: `packages/maproom-mcp/tests/fixtures/`

```
fixtures/
├── simple-repo/          # Minimal test case
│   ├── file1.ts          # 1 function
│   └── file2.ts          # Same function (dedup test)
├── realistic-repo/       # Production-like
│   ├── src/
│   │   ├── auth.ts      # 10 functions
│   │   ├── db.ts        # 5 functions
│   │   └── utils.ts     # 20 functions
│   └── tests/
└── edge-cases/
    ├── empty.ts          # Empty file
    ├── unicode.ts        # Unicode content
    └── large.ts          # 1000+ functions
```

### Synthetic Data Generation

**For stress testing**:

```typescript
function generateSyntheticChunks(count: number, overlapPercent: number) {
  const unique = Math.floor(count * (1 - overlapPercent));
  const chunks = [];

  // Generate unique chunks
  for (let i = 0; i < unique; i++) {
    chunks.push({
      content: `function unique_${i}() { return ${i}; }`,
    });
  }

  // Generate duplicate chunks
  for (let i = unique; i < count; i++) {
    const original = chunks[i % unique];
    chunks.push({ ...original }); // Duplicate
  }

  return chunks;
}

// Test with 80% overlap (realistic branch scenario)
const chunks = generateSyntheticChunks(10000, 0.8);
```

## Test Environment

### Database Setup

**Use Docker for isolated testing**:

```yaml
# docker-compose.test.yml
services:
  postgres-test:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_DB: maproom_test
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
    ports:
      - "5433:5432"
```

**Before each test suite**:
```bash
docker-compose -f docker-compose.test.yml up -d
npm run migrate:test
```

**After tests**:
```bash
docker-compose -f docker-compose.test.yml down -v
```

### CI Pipeline

**GitHub Actions** (`.github/workflows/test-blobsha.yml`):

```yaml
name: Test BLOBSHA Project

on:
  push:
    paths:
      - 'crates/maproom/src/content_hash.rs'
      - 'packages/maproom-mcp/migrations/**'
      - 'packages/maproom-mcp/tests/**'

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_DB: maproom_test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Run Rust tests
        run: cargo test --package maproom content_hash

      - name: Run migration tests
        run: npm test -- migration.test.ts

      - name: Run E2E tests
        run: npm test -- e2e/
```

## Manual Testing Checklist

**Before marking project complete**:

- [ ] Run full test suite: `npm test && cargo test`
- [ ] Run migrations on staging database
- [ ] Verify deduplication: Check `code_embeddings` count vs `chunks` count
- [ ] Measure cache hit rate: Index branch twice, second time should be 100% hits
- [ ] Performance test: Search query time before/after migration
- [ ] Spot check: Manually verify a few chunks have correct blob SHA
- [ ] Rollback test: Verify rollback script restores original state

## Acceptance Criteria

**Project is complete when**:

1. ✅ All unit tests pass (100% of `compute_blob_sha`)
2. ✅ All integration tests pass (migrations, cache behavior)
3. ✅ E2E search equivalence test passes
4. ✅ Performance within 10% of baseline
5. ✅ Manual testing checklist complete
6. ✅ Zero data loss verified (chunk count unchanged)
7. ✅ Deduplication working (embedding count < chunk count)

**Any failure** → Return to implementation, do not proceed to next phase

## Risk Mitigation

**Backup before migration**:
```bash
pg_dump maproom > backup_before_blobsha_$(date +%Y%m%d).sql
```

**Gradual rollout**:
1. Test on development database
2. Test on staging with production-size data
3. Deploy to production with monitoring

**Monitoring during migration**:
- Watch query latency (should not spike)
- Monitor disk usage (will increase during migration, decrease after VACUUM)
- Track error rates (should remain zero)

## Next Steps

Detailed implementation plan in `plan.md`
