# Quality Strategy: Search Result Deduplication

## Testing Philosophy

This project adds a focused piece of functionality (deduplication) to an existing, well-tested search pipeline. Our testing strategy prioritizes:

1. **Correctness over coverage** - Ensure deduplication works as specified
2. **Integration over isolation** - Most value comes from E2E behavior
3. **Regression prevention** - Existing search behavior must not break
4. **Performance validation** - Latency must remain acceptable

## Test Levels

### Level 1: Unit Tests (dedup.rs)

**Focus:** Core deduplication logic in isolation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_key_generation() {
        // Same relpath, symbol, line → same identity
        // Different relpath → different identity
        // Different symbol_name → different identity
        // Different start_line → different identity
    }

    #[test]
    fn test_deduplicate_empty_results() {
        // Empty input → empty output
    }

    #[test]
    fn test_deduplicate_no_duplicates() {
        // Unique results → unchanged
    }

    #[test]
    fn test_deduplicate_all_duplicates() {
        // All same identity → one result
    }

    #[test]
    fn test_deduplicate_mixed() {
        // Some duplicates, some unique → correct count
    }

    #[test]
    fn test_deduplicate_preserves_order() {
        // Results remain sorted by score after dedup
    }

    #[test]
    fn test_highest_score_selection() {
        // Among duplicates, highest score wins
    }

    #[test]
    fn test_disabled_config() {
        // deduplicate=false → no deduplication
    }

    #[test]
    fn test_null_symbol_name_handling() {
        // symbol_name=None treated as empty string for grouping
    }
}
```

**Test Count:** ~10 unit tests
**Time Budget:** <100ms total

### Level 2: Integration Tests (pipeline integration)

**Focus:** Deduplication works correctly within search pipeline

```rust
#[tokio::test]
async fn test_search_with_deduplication_enabled() {
    // Setup: Insert same chunk in multiple worktrees
    // Action: Search with deduplicate=true
    // Verify: Only one result per unique chunk
}

#[tokio::test]
async fn test_search_with_deduplication_disabled() {
    // Setup: Insert same chunk in multiple worktrees
    // Action: Search with deduplicate=false
    // Verify: All duplicates returned
}

#[tokio::test]
async fn test_search_default_enables_deduplication() {
    // Setup: Insert duplicates
    // Action: Search with default options
    // Verify: Deduplication is applied
}

#[tokio::test]
async fn test_search_mode_compatibility() {
    // Verify dedup works with FTS, vector, and hybrid modes
}
```

**Test Count:** ~5 integration tests
**Time Budget:** <5s total (uses test database)

### Level 3: End-to-End Tests (MCP tool)

**Focus:** Deduplication behavior exposed correctly via MCP API

```typescript
describe('search tool deduplication', () => {
  it('deduplicates results by default', async () => {
    // Setup: Index same file in two worktrees
    // Action: Call search tool
    // Verify: Single result returned
  });

  it('respects deduplicate=false parameter', async () => {
    // Setup: Index same file in two worktrees
    // Action: Call search tool with deduplicate: false
    // Verify: Both results returned
  });
});
```

**Test Count:** ~3 E2E tests
**Time Budget:** <30s total

## Critical Test Scenarios

### Scenario 1: Duplicate Detection Accuracy

**Setup:**
- Same file `src/auth.rs` with function `validate` at line 10
- Indexed in worktrees: main, feature/auth, stale-snapshot

**Test:**
```rust
// Insert chunks with same (relpath, symbol, line) but different chunk_ids
let chunk1 = insert_chunk("main", "src/auth.rs", "validate", 10);
let chunk2 = insert_chunk("feature/auth", "src/auth.rs", "validate", 10);
let chunk3 = insert_chunk("stale-snapshot", "src/auth.rs", "validate", 10);

// Search
let results = search("validate", options_with_dedup()).await?;

// Assert: Only 1 result
assert_eq!(results.len(), 1);
assert!(results[0].score >= chunk1.score.max(chunk2.score).max(chunk3.score));
```

### Scenario 2: Non-Duplicate Preservation

**Setup:**
- Different files with same symbol name: `src/auth.rs:validate` and `src/payment.rs:validate`

**Test:**
```rust
// Insert chunks with different relpaths
let chunk1 = insert_chunk("main", "src/auth.rs", "validate", 10);
let chunk2 = insert_chunk("main", "src/payment.rs", "validate", 25);

// Search
let results = search("validate", options_with_dedup()).await?;

// Assert: Both results preserved (different files)
assert_eq!(results.len(), 2);
```

### Scenario 3: Score-Based Selection

**Setup:**
- Same chunk in two worktrees with different scores (e.g., due to recency)

**Test:**
```rust
// Insert duplicates with different scores
let chunk1 = insert_chunk_with_score("main", "src/auth.rs", "validate", 10, 0.95);
let chunk2 = insert_chunk_with_score("old", "src/auth.rs", "validate", 10, 0.80);

// Search
let results = search("validate", options_with_dedup()).await?;

// Assert: Higher score selected
assert_eq!(results.len(), 1);
assert_eq!(results[0].score, 0.95);
```

### Scenario 4: Performance Under Load

**Setup:**
- 1000 search results with 50% duplicates (500 unique)

**Test:**
```rust
// Generate 1000 results, 500 unique identities
let results = generate_test_results(1000, 500);

// Time deduplication
let start = Instant::now();
let deduplicated = deduplicate(results, &config);
let elapsed = start.elapsed();

// Assert: Performance acceptable
assert!(elapsed.as_millis() < 50); // <50ms for 1000 results
assert_eq!(deduplicated.len(), 500);
```

## Regression Test Requirements

### Existing Tests Must Pass

All existing search tests must continue passing:
- `cargo test --lib search::` - All search module tests
- `cargo test --test search_integration` - Integration tests
- MCP search tool tests

### Backward Compatibility

**No API Breaking Changes:**
- `SearchOptions::new()` signature unchanged
- `FinalSearchResults` structure unchanged
- MCP search tool parameters unchanged (deduplicate is optional)

## Test Data Requirements

### Unit Test Data
- Hand-crafted `ChunkSearchResult` structs
- No database required
- Deterministic, reproducible

### Integration Test Data
- Test database with schema
- Fixture data: multiple worktrees, duplicate chunks
- Cleaned up after each test

### E2E Test Data
- Test repository indexed with daemon
- Known duplicate files across worktrees

## Test Fixture Creation

### Unit Test Fixtures

Create helper functions for building test data:

```rust
// In tests/fixtures/search_fixtures.rs or within dedup.rs tests

fn make_chunk_result(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
    score: f64,
) -> ChunkSearchResult {
    ChunkSearchResult {
        chunk_id,
        file_id: 1,
        relpath: relpath.to_string(),
        symbol_name: symbol_name.map(|s| s.to_string()),
        kind: "function".to_string(),
        start_line,
        end_line: start_line + 10,
        preview: "...".to_string(),
        score,
    }
}

fn make_duplicates(count: usize, relpath: &str, symbol: &str, line: i32) -> Vec<ChunkSearchResult> {
    (0..count)
        .map(|i| make_chunk_result(
            i as i64,
            relpath,
            Some(symbol),
            line,
            0.9 - (i as f64 * 0.05),  // Decreasing scores
        ))
        .collect()
}
```

### Integration Test Fixtures

For integration tests, insert duplicate chunks into the test database:

```rust
async fn setup_duplicate_chunks(db: &Pool) -> Result<()> {
    // Create two worktrees for the same repo
    let repo_id = insert_repo(db, "test-repo").await?;
    let wt_main = insert_worktree(db, repo_id, "main").await?;
    let wt_feature = insert_worktree(db, repo_id, "feature-x").await?;

    // Insert the same file in both worktrees
    let file_main = insert_file(db, wt_main, "src/auth.rs").await?;
    let file_feature = insert_file(db, wt_feature, "src/auth.rs").await?;

    // Insert identical chunks
    let chunk_content = "fn validate(token: &str) -> bool { ... }";
    insert_chunk(db, file_main, "validate", 10, 25, chunk_content).await?;
    insert_chunk(db, file_feature, "validate", 10, 25, chunk_content).await?;

    Ok(())
}
```

### MCP E2E Test Fixtures

For MCP tests, use a dedicated test repository with known duplicates:

```typescript
// In packages/maproom-mcp/tests/fixtures/setup-duplicates.ts

async function setupDuplicateIndex() {
  // Create test repo with two worktrees
  const testRepo = await createTestRepo('dedup-test');

  // Create main worktree with auth.ts
  await createWorktree(testRepo, 'main', {
    'src/auth.ts': `
      export function validateToken(token: string): boolean {
        return token.startsWith('valid-');
      }
    `,
  });

  // Create feature worktree with same file
  await createWorktree(testRepo, 'feature-auth', {
    'src/auth.ts': `
      export function validateToken(token: string): boolean {
        return token.startsWith('valid-');
      }
    `,
  });

  // Index both worktrees
  await indexWorktree(testRepo, 'main');
  await indexWorktree(testRepo, 'feature-auth');

  return testRepo;
}
```

### Fixture Verification

Each test should verify fixtures are set up correctly:

```rust
#[tokio::test]
async fn test_fixture_creates_duplicates() {
    let db = setup_test_db().await;
    setup_duplicate_chunks(&db).await.unwrap();

    // Verify duplicates exist
    let results = search_raw("validate", &db).await.unwrap();
    assert!(results.len() >= 2, "Expected at least 2 duplicate chunks");

    // Verify they have same identity key
    let identities: HashSet<_> = results.iter()
        .map(|r| ChunkIdentity::from_result(r))
        .collect();
    assert_eq!(identities.len(), 1, "All results should have same identity");
}
```

## Performance Benchmarks

### Deduplication Benchmark

```rust
#[bench]
fn bench_deduplicate_100_results(b: &mut Bencher) {
    let results = generate_test_results(100, 50);
    b.iter(|| deduplicate(results.clone(), &DeduplicationConfig::default()));
}

#[bench]
fn bench_deduplicate_1000_results(b: &mut Bencher) {
    let results = generate_test_results(1000, 500);
    b.iter(|| deduplicate(results.clone(), &DeduplicationConfig::default()));
}

#[bench]
fn bench_deduplicate_10000_results(b: &mut Bencher) {
    let results = generate_test_results(10000, 5000);
    b.iter(|| deduplicate(results.clone(), &DeduplicationConfig::default()));
}
```

**Performance Targets:**
| Result Count | Target Latency |
|--------------|----------------|
| 100 | <1ms |
| 1000 | <10ms |
| 10000 | <100ms |

## Test Organization

```
crates/maproom/
├── src/search/
│   └── dedup.rs          # Unit tests in #[cfg(test)] module
├── tests/
│   └── search_dedup_integration.rs  # Integration tests
└── benches/
    └── dedup_bench.rs    # Performance benchmarks

packages/maproom-mcp/
└── tests/
    └── search-dedup.test.ts  # E2E tests
```

## Acceptance Criteria Verification

| Criterion | Test Type | Test File |
|-----------|-----------|-----------|
| Results deduplicated by (relpath, symbol, line) | Unit | dedup.rs |
| Highest score selected as representative | Unit | dedup.rs |
| Default behavior is deduplicated | Integration | search_dedup_integration.rs |
| Can disable via SearchOptions | Integration | search_dedup_integration.rs |
| MCP tool exposes deduplicate parameter | E2E | search-dedup.test.ts |
| Performance <10ms for 1000 results | Benchmark | dedup_bench.rs |

## Definition of Done

- [ ] All unit tests pass: `cargo test --lib search::dedup`
- [ ] All integration tests pass: `cargo test --test search_dedup_integration`
- [ ] All existing search tests pass (no regression)
- [ ] Performance benchmarks meet targets
- [ ] MCP E2E tests pass (if added)
- [ ] Manual verification: search for known duplicate returns 1 result
