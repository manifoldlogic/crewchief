# Quality Strategy: Relationship-Aware Search

## Testing Philosophy

**Confidence over coverage.** Focus testing effort on critical paths that provide confidence in relationship expansion correctness, performance, and backward compatibility. Ship pragmatically tested code, not ceremonially tested code.

**Key Principles**:
1. Test for confidence, not coverage percentages
2. Focus on edge cases where bugs are likely (graph cycles, empty results, missing confidence)
3. Performance testing is non-negotiable (latency budget is a hard constraint)
4. Type synchronization must be validated (Rust ↔ TypeScript divergence breaks clients)
5. Backward compatibility is critical (existing users must not break)

## Test Types

### Unit Tests

**Scope**: Core relationship expansion logic in isolation

**Tools**:
- Rust: `cargo test` with `#[cfg(test)]` modules
- TypeScript: Jest for type validation tests

**Rust Unit Tests** (`crates/maproom/src/search/relationships.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_edge_weight_computation() {
        // Extends/implements edges get 1.1× boost
        assert_eq!(compute_edge_weight("extends", "class"), 1.1);
        assert_eq!(compute_edge_weight("implements", "interface"), 1.1);

        // Test edges get 0.5× penalty
        assert_eq!(compute_edge_weight("import", "test"), 0.5);
        assert_eq!(compute_edge_weight("call", "test_helper"), 0.5);

        // Normal edges get 1.0×
        assert_eq!(compute_edge_weight("import", "function"), 1.0);
        assert_eq!(compute_edge_weight("call", "method"), 1.0);
    }

    #[test]
    fn test_module_proximity_boost() {
        // Same directory should match
        let source = "src/auth/handler.ts";
        let related_same = "src/auth/validator.ts";
        let related_diff = "src/utils/logger.ts";

        assert_eq!(extract_parent_dir(source), "src/auth");
        assert_eq!(extract_parent_dir(related_same), "src/auth");
        assert_eq!(extract_parent_dir(related_diff), "src/utils");
    }

    #[test]
    fn test_relevance_sorting() {
        // Higher relevance should rank first
        let chunks = vec![
            (chunk_a, 0.4),  // Lower
            (chunk_b, 0.8),  // Higher
            (chunk_c, 0.6),  // Middle
        ];

        let sorted = sort_by_relevance(chunks);
        assert_eq!(sorted[0].1, 0.8);  // chunk_b first
        assert_eq!(sorted[1].1, 0.6);  // chunk_c second
        assert_eq!(sorted[2].1, 0.4);  // chunk_a last
    }

    #[test]
    fn test_preview_truncation() {
        let content = "a".repeat(150);
        let preview = truncate_preview(&content, 100);
        assert_eq!(preview.len(), 103);  // 100 chars + "..."
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn test_empty_related_chunks() {
        // Graph traversal returns no chunks
        let store = setup_test_store().await;
        let related = find_top_related_chunks(&store, isolated_chunk_id, 5).await.unwrap();
        assert_eq!(related.len(), 0);
    }

    #[test]
    fn test_fewer_than_limit() {
        // Graph has 2 related chunks, limit is 5
        let store = setup_test_store().await;
        let related = find_top_related_chunks(&store, chunk_with_2_edges, 5).await.unwrap();
        assert_eq!(related.len(), 2);
    }
}
```

**TypeScript Unit Tests** (`packages/daemon-client/src/types.test.ts`):
```typescript
describe('RelatedChunkResult type sync', () => {
  it('matches Rust struct fields', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 123,
      relpath: 'src/foo.ts',
      symbol_name: 'myFunction',
      kind: 'function',
      start_line: 10,
      end_line: 20,
      preview: 'function myFunction() {...',
      depth: 2,
      relevance: 0.7,
      relationship_type: 'call',
    };

    // If Rust changes a field name, this fails
    expect(sample.chunk_id).toBe(123);
    expect(sample.relationship_type).toBe('call');
  });
});
```

**Coverage Target**: 80% for relationship expansion module (pragmatic, not exhaustive).

### Integration Tests

**Scope**: End-to-end search with relationship expansion, confidence gating, backward compatibility

**Approach**: Black-box testing of search pipeline with mocked database

**Rust Integration Tests** (`crates/maproom/tests/search_relationships_test.rs`):
```rust
#[tokio::test]
async fn test_search_with_relationships() {
    let store = setup_test_database().await;

    // Insert test data: chunk A imports chunk B, B calls C
    insert_test_chunks(&store).await;
    insert_test_edges(&store, vec![
        (chunk_a, chunk_b, "import"),
        (chunk_b, chunk_c, "call"),
    ]).await;

    // Search with include_related=true, include_confidence=true
    let params = SearchParams {
        query: "test query",
        include_related: Some(true),
        include_confidence: Some(true),
        ..Default::default()
    };

    let results = search(&store, params).await.unwrap();

    // Verify result with high confidence has related field
    let high_conf_result = results.iter()
        .find(|r| r.confidence.as_ref().unwrap().source_count >= 2)
        .unwrap();

    assert!(high_conf_result.related.is_some());
    let related = high_conf_result.related.as_ref().unwrap();
    assert!(related.len() > 0);
    assert!(related.len() <= 5);
}

#[tokio::test]
async fn test_confidence_gating() {
    let store = setup_test_database().await;

    // Mock results: 2 high-confidence, 8 low-confidence
    let results = mock_search_results(vec![
        (chunk_1, ConfidenceSignals { source_count: 3, is_exact_match: false, .. }),  // High
        (chunk_2, ConfidenceSignals { source_count: 1, is_exact_match: false, .. }),  // Low
        (chunk_3, ConfidenceSignals { source_count: 1, is_exact_match: true, .. }),   // High (exact match)
        // ... 7 more low-confidence
    ]);

    let expanded = apply_relationship_expansion(&store, results).await.unwrap();

    let with_related = expanded.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(with_related, 2);  // Only 2 high-confidence results
}

#[tokio::test]
async fn test_backward_compatibility() {
    let store = setup_test_database().await;

    // Search without include_related parameter
    let params = SearchParams {
        query: "test query",
        include_related: None,
        ..Default::default()
    };

    let results = search(&store, params).await.unwrap();

    // No result should have related field
    for result in results {
        assert!(result.related.is_none());
    }
}
```

**MCP Integration Tests** (`packages/maproom-mcp/tests/search-relationships.test.ts`):
```typescript
describe('MCP search with relationships', () => {
  it('returns related chunks when requested', async () => {
    const result = await mcpClient.call('search', {
      query: 'authentication',
      repo: 'test-repo',
      include_related: true,
      include_confidence: true,
    });

    const highConfResults = result.results.filter(r =>
      r.confidence?.source_count >= 2 || r.confidence?.is_exact_match
    );

    // High-confidence results should have related field
    for (const r of highConfResults) {
      expect(r.related).toBeDefined();
      expect(Array.isArray(r.related)).toBe(true);
      expect(r.related.length).toBeLessThanOrEqual(5);
    }
  });
});
```

### End-to-End Tests

**Scope**: Critical paths only (high-confidence result expansion, error handling)

**Approach**: Real database, real search queries, measure actual latency

**Critical Path 1: High-Confidence Result Expansion**
```rust
#[tokio::test]
async fn e2e_high_confidence_expansion() {
    // Real database with production-like data
    let db_path = setup_e2e_database().await;
    let store = SqliteStore::new(&db_path).await.unwrap();

    // Real search query
    let results = execute_search(&store, "authentication handler", true, true).await;

    // Verify at least one high-confidence result has relationships
    let expanded = results.iter()
        .filter(|r| r.confidence.as_ref().map_or(false, |c| c.source_count >= 2))
        .filter(|r| r.related.is_some())
        .count();

    assert!(expanded > 0, "Expected at least one expanded result");
}
```

**Critical Path 2: Performance Budget**
```rust
#[tokio::test]
async fn e2e_performance_budget() {
    let store = setup_e2e_database().await;

    // Baseline: search without relationships
    let start = Instant::now();
    execute_search(&store, "test query", false, true).await;
    let baseline = start.elapsed();

    // With relationships
    let start = Instant::now();
    execute_search(&store, "test query", true, true).await;
    let with_relationships = start.elapsed();

    let overhead = with_relationships - baseline;
    assert!(overhead < Duration::from_millis(20),
        "Overhead {} ms exceeds 20ms budget", overhead.as_millis());
}
```

**Critical Path 3: Graceful Degradation**
```rust
#[tokio::test]
async fn e2e_graph_traversal_failure() {
    let store = setup_e2e_database().await;

    // Corrupt chunk_edges table or simulate database error
    simulate_graph_error(&store).await;

    // Search should still succeed
    let results = execute_search(&store, "test query", true, true).await;

    // Results without related field (graceful degradation)
    assert!(results.len() > 0);
    assert!(results.iter().all(|r| r.related.is_none()));
}
```

## Critical Paths

The following paths MUST be tested with high confidence:

### 1. High-Confidence Result Expansion
- **What**: Results with `source_count >= 2` OR `is_exact_match` get `related` field populated
- **Why Critical**: Core feature value depends on correct confidence gating
- **Tests**: Unit (confidence threshold), Integration (mock confidence), E2E (real search)

### 2. Performance Budget Compliance
- **What**: Relationship expansion overhead <20ms p95
- **Why Critical**: Exceeding budget breaks initiative latency constraint
- **Tests**: Benchmark suite, E2E performance tests

### 3. Backward Compatibility
- **What**: Searches without `include_related` parameter work unchanged
- **Why Critical**: Breaking existing clients is unacceptable
- **Tests**: Integration tests (without parameter), E2E tests (production clients)

### 4. Type Synchronization
- **What**: Rust `RelatedChunkResult` matches TypeScript interface
- **Why Critical**: Divergence breaks MCP clients with runtime errors
- **Tests**: Type validation tests, JSON serialization round-trip

### 5. Graph Traversal Correctness
- **What**: Depth-2 traversal returns correct related chunks with correct relevance
- **Why Critical**: Incorrect relationships erode user trust
- **Tests**: Unit tests (graph queries), Integration tests (edge cases), E2E tests (real graphs)

### 6. Error Handling (Graceful Degradation)
- **What**: Graph errors don't fail entire search
- **Why Critical**: Robustness requirement (search must always succeed)
- **Tests**: Integration tests (simulated errors), E2E tests (corrupted data)

## Test Data Strategy

### Unit Test Data

**Approach**: Minimal synthetic data in-memory

- Mock `RelatedChunk` structs with known relevance scores
- Edge weight test cases covering all relationship types
- Module proximity test cases with various directory structures

**Why**: Fast, isolated, deterministic.

### Integration Test Data

**Approach**: SQLite in-memory database with fixture data

```rust
async fn setup_test_database() -> SqliteStore {
    let store = SqliteStore::new(":memory:").await.unwrap();

    // Insert fixture chunks
    store.insert_chunk(chunk_a).await.unwrap();
    store.insert_chunk(chunk_b).await.unwrap();
    store.insert_chunk(chunk_c).await.unwrap();

    // Insert fixture edges
    store.insert_edge(chunk_a, chunk_b, "import").await.unwrap();
    store.insert_edge(chunk_b, chunk_c, "call").await.unwrap();

    store
}
```

**Why**: Controlled, repeatable, covers edge cases (cycles, empty graphs).

### E2E Test Data

**Approach**: Real database snapshot from production-like repository

- Copy of actual maproom database (anonymized if needed)
- Real chunk_edges table with production relationships
- Real search queries from usage logs

**Why**: Validates real-world performance and relationship quality.

### Golden Test Sets

**Maintained Test Queries**:
1. "authentication handler" → Should find auth code with related imports
2. "error handling" → Should find error handling code with related callers
3. "database query" → Should find database code with related tests (weighted lower)

**Review**: Quarterly review of golden set relevance.

## Quality Gates

Before ticket verification:

### Phase 1: Rust Core

- [ ] Unit tests pass (cargo test)
- [ ] Edge weight computation correct for all relationship types
- [ ] Module proximity boost applied correctly
- [ ] Relevance sorting works correctly
- [ ] Empty result edge case handled
- [ ] Performance benchmark: Single result traversal <8ms p95
- [ ] No linting errors (cargo clippy)

### Phase 2: TypeScript Integration

- [ ] Type sync validation tests pass
- [ ] MCP search tool accepts `include_related` parameter
- [ ] JSON serialization round-trip works (Rust → JSON → TypeScript)
- [ ] Integration tests pass (search with relationships)
- [ ] Backward compatibility tests pass (without parameter)
- [ ] No TypeScript errors (tsc --noEmit)
- [ ] No linting errors (eslint)

### Phase 3: Testing & Documentation

- [ ] E2E tests pass (high-confidence expansion, performance, error handling)
- [ ] Performance tests validate <20ms overhead at p95
- [ ] Confidence gating tests pass (only high-confidence expanded)
- [ ] Response size <10KB for typical queries
- [ ] Documentation complete (usage examples, architecture)
- [ ] All previous phase tests still passing

## Performance Testing

### Benchmark Suite

**Required Benchmarks**:

1. **Baseline Search Latency** (without relationships)
   - Measure p50, p95, p99 for typical queries
   - Establish baseline for overhead calculation

2. **Relationship Expansion Overhead**
   - Measure delta between baseline and with `include_related=true`
   - Target: <20ms p95 overhead
   - Fail CI if exceeds budget

3. **Graph Traversal Scaling**
   - Measure latency vs number of edges (10, 100, 1000 edges per chunk)
   - Ensure depth-2 traversal remains bounded

4. **Parallel vs Sequential Traversal**
   - If sequential exceeds budget, benchmark parallel traversal
   - Measure database concurrency impact

**Benchmark Execution**:
```bash
cargo bench --bench search_relationships
```

**CI Integration**: Performance regression tests run on every commit. Fail build if overhead exceeds 20ms.

## Continuous Integration

**GitHub Actions Workflow** (`.github/workflows/test.yml`):

```yaml
jobs:
  test-relationships:
    runs-on: ubuntu-latest
    steps:
      - name: Rust unit tests
        run: cargo test --package crewchief-maproom relationships

      - name: TypeScript type sync tests
        run: cd packages/daemon-client && npm test types.test.ts

      - name: Performance benchmarks
        run: cargo bench --bench search_relationships

      - name: E2E integration tests
        run: cargo test --test search_relationships_e2e
```

**Success Criteria**: All tests green before merge.

## Manual Testing Checklist

Before final sign-off:

- [ ] Run search with `include_related=true` on production database copy
- [ ] Verify related chunks are architecturally meaningful
- [ ] Check response payload size in Chrome DevTools
- [ ] Test edge case: Search with no results
- [ ] Test edge case: All results low-confidence (no relationships)
- [ ] Test edge case: Graph with cycles (verify cycle detection works)
- [ ] Test error case: Database connection failure (verify graceful degradation)

## Acceptance Criteria Validation

**Checklist for Ticket Verification**:

1. **Functionality**:
   - [ ] High-confidence results have `related` field
   - [ ] Related chunks count ≤ 5 per result
   - [ ] Relevance scores correct (decay × edge_weight × module_boost)

2. **Performance**:
   - [ ] Overhead <20ms p95 (measured via benchmarks)
   - [ ] Response size <10KB (measured via E2E tests)

3. **Compatibility**:
   - [ ] Searches without parameter unchanged (backward compatibility tests pass)
   - [ ] Type sync validated (Rust ↔ TypeScript tests pass)

4. **Robustness**:
   - [ ] Graceful degradation on errors (E2E error tests pass)
   - [ ] Empty result edge case handled (unit tests pass)

5. **Code Quality**:
   - [ ] No linting errors
   - [ ] Inline documentation complete
   - [ ] TYPE_SYNC comments present
