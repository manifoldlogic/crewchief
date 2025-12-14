# Ticket: [SRCHREL-3002]: Comprehensive Edge Case Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- test-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive test coverage for edge cases including confidence gating, error handling, graph traversal edge cases, and empty result scenarios.

## Background
Robust software handles edge cases gracefully. This ticket ensures relationship expansion works correctly in unusual scenarios: no confidence data, graph traversal failures, cyclic graphs, all low-confidence results, empty graphs, and more.

This implements Phase 3 deliverables: confidence gating validation, error handling validation, edge case coverage.

## Acceptance Criteria
- [ ] Edge case test suite created with 10+ scenarios
- [ ] Confidence gating tests validate threshold (source_count >= 2 OR is_exact_match)
- [ ] Error handling tests validate graceful degradation
- [ ] Graph edge cases tested (cycles, empty, depth limits)
- [ ] Empty result semantics tested (None vs Some([]))
- [ ] All edge case tests pass
- [ ] Test coverage includes manual test checklist items from quality-strategy.md
- [ ] Documentation includes known edge cases and expected behavior

## Technical Requirements

### Edge Case Test Suite
Create `crates/maproom/tests/edge_cases_test.rs`:

```rust
#[tokio::test]
async fn test_no_confidence_data() {
    let store = setup_test_db().await;

    // Search with include_related=true but confidence disabled
    let params = SearchParams {
        query: "test",
        include_related: Some(true),
        include_confidence: Some(false),  // Explicitly disabled
        ..Default::default()
    };

    let results = search(&store, params).await.unwrap();

    // Without confidence, expansion should not run
    for result in results {
        assert!(result.related.is_none());
    }
}

#[tokio::test]
async fn test_all_low_confidence_results() {
    let store = setup_test_db().await;

    // Mock search results with all source_count < 2 and is_exact_match = false
    let mut results = vec![
        ChunkSearchResult {
            confidence: Some(ConfidenceSignals {
                source_count: 1,
                is_exact_match: false,
                ..Default::default()
            }),
            ..Default::default()
        },
        // ... 9 more low-confidence results
    ];

    apply_relationship_expansion(&store, &mut results).await;

    // No result should have related field
    let with_related = results.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(with_related, 0);
}

#[tokio::test]
async fn test_exact_match_triggers_expansion() {
    let store = setup_test_db().await;

    let mut results = vec![
        ChunkSearchResult {
            chunk_id: 123,
            confidence: Some(ConfidenceSignals {
                source_count: 1,  // Low count
                is_exact_match: true,  // But exact match
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    apply_relationship_expansion(&store, &mut results).await;

    // Exact match should trigger expansion despite low source_count
    assert!(results[0].related.is_some());
}

#[tokio::test]
async fn test_source_count_threshold() {
    let store = setup_test_db().await;

    let test_cases = vec![
        (1, false, false),  // Below threshold
        (2, false, true),   // At threshold
        (3, false, true),   // Above threshold
    ];

    for (source_count, is_exact_match, should_expand) in test_cases {
        let mut results = vec![
            ChunkSearchResult {
                chunk_id: 123,
                confidence: Some(ConfidenceSignals {
                    source_count,
                    is_exact_match,
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        apply_relationship_expansion(&store, &mut results).await;

        let has_related = results[0].related.is_some();
        assert_eq!(
            has_related, should_expand,
            "source_count={}, is_exact_match={}, expected expand={}",
            source_count, is_exact_match, should_expand
        );
    }
}

#[tokio::test]
async fn test_graph_traversal_failure() {
    let store = setup_test_db().await;

    // Simulate database error (corrupt table, connection failure, etc.)
    simulate_graph_error(&store).await;

    let mut results = vec![
        ChunkSearchResult {
            chunk_id: 123,
            confidence: Some(ConfidenceSignals {
                source_count: 3,
                is_exact_match: false,
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    // Should not panic or fail
    apply_relationship_expansion(&store, &mut results).await;

    // Result should have related=None (graceful degradation)
    assert!(results[0].related.is_none());
}

#[tokio::test]
async fn test_cyclic_graph() {
    let store = setup_test_db().await;

    // Create cycle: A → B → C → A
    insert_edge(&store, chunk_a, chunk_b, "import").await;
    insert_edge(&store, chunk_b, chunk_c, "call").await;
    insert_edge(&store, chunk_c, chunk_a, "import").await;

    let related = find_top_related_chunks(&store, chunk_a, 5).await.unwrap();

    // Should not infinite loop, should detect cycle
    assert!(related.len() > 0);
    assert!(related.len() <= 5);

    // Should not include source chunk (chunk_a) in results
    assert!(!related.iter().any(|r| r.chunk_id == chunk_a));
}

#[tokio::test]
async fn test_empty_graph() {
    let store = setup_test_db().await;

    // Chunk with no edges
    insert_chunk(&store, isolated_chunk).await;

    let related = find_top_related_chunks(&store, isolated_chunk.id, 5).await.unwrap();

    // Should return empty array (not error)
    assert_eq!(related.len(), 0);
}

#[tokio::test]
async fn test_depth_limit_enforced() {
    let store = setup_test_db().await;

    // Create chain: A → B → C → D → E (depth 4)
    insert_edge(&store, chunk_a, chunk_b, "import").await;
    insert_edge(&store, chunk_b, chunk_c, "call").await;
    insert_edge(&store, chunk_c, chunk_d, "import").await;
    insert_edge(&store, chunk_d, chunk_e, "call").await;

    let related = find_top_related_chunks(&store, chunk_a, 5).await.unwrap();

    // Should only traverse to depth 2 (B and C, not D or E)
    let max_depth = related.iter().map(|r| r.depth).max().unwrap_or(0);
    assert!(max_depth <= 2);

    // Should not include D or E
    assert!(!related.iter().any(|r| r.chunk_id == chunk_d.id));
    assert!(!related.iter().any(|r| r.chunk_id == chunk_e.id));
}

#[tokio::test]
async fn test_more_than_limit() {
    let store = setup_test_db().await;

    // Create chunk with 10 edges
    for i in 0..10 {
        insert_edge(&store, source_chunk, related_chunk[i], "import").await;
    }

    let related = find_top_related_chunks(&store, source_chunk.id, 5).await.unwrap();

    // Should return exactly 5 (top by relevance)
    assert_eq!(related.len(), 5);

    // Should be sorted by relevance descending
    for i in 0..related.len()-1 {
        assert!(related[i].relevance >= related[i+1].relevance);
    }
}

#[tokio::test]
async fn test_empty_result_semantics() {
    let store = setup_test_db().await;

    // High-confidence result with no relationships
    let mut results = vec![
        ChunkSearchResult {
            chunk_id: isolated_chunk.id,
            confidence: Some(ConfidenceSignals {
                source_count: 3,
                is_exact_match: false,
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    apply_relationship_expansion(&store, &mut results).await;

    // Should have Some([]), not None
    assert!(results[0].related.is_some());
    assert_eq!(results[0].related.as_ref().unwrap().len(), 0);
}

#[tokio::test]
async fn test_max_concurrent_expansions_cap() {
    let store = setup_test_db().await;

    // 5 high-confidence results
    let mut results: Vec<ChunkSearchResult> = (0..5).map(|i| {
        ChunkSearchResult {
            chunk_id: i,
            confidence: Some(ConfidenceSignals {
                source_count: 3,
                is_exact_match: false,
                ..Default::default()
            }),
            ..Default::default()
        }
    }).collect();

    apply_relationship_expansion(&store, &mut results).await;

    // Only first 3 should be expanded (MAX_CONCURRENT_EXPANSIONS)
    let with_related = results.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(with_related, 3);

    // First 3 results should have related field
    assert!(results[0].related.is_some());
    assert!(results[1].related.is_some());
    assert!(results[2].related.is_some());

    // Last 2 results should not
    assert!(results[3].related.is_none());
    assert!(results[4].related.is_none());
}
```

### TypeScript Edge Case Tests
Add to `packages/daemon-client/src/types.test.ts`:

```typescript
describe('RelatedChunkResult edge cases', () => {
  it('handles missing optional fields gracefully', () => {
    const minimal: Partial<RelatedChunkResult> = {
      chunk_id: 123,
      relpath: 'src/foo.ts',
      // Other fields intentionally missing
    };

    // TypeScript should catch this at compile time
    // This validates type definitions are strict
  });

  it('validates null vs undefined semantics', () => {
    const result: ChunkSearchResult = {
      chunk_id: 1,
      // ... other fields ...
      related: undefined,  // Expansion didn't run
    };

    expect(result.related).toBeUndefined();

    const resultWithEmpty: ChunkSearchResult = {
      chunk_id: 2,
      // ... other fields ...
      related: [],  // Expansion ran, no results
    };

    expect(resultWithEmpty.related).toEqual([]);
  });
});
```

## Implementation Notes

Test organization:
- Group related edge cases in describe blocks
- Use descriptive test names
- Document expected behavior in comments

Error simulation:
- Create helper function `simulate_graph_error()`
- Mock database failures or corrupt data
- Validate graceful degradation (log warning, don't throw)

Confidence gating tests:
- Test all combinations of source_count and is_exact_match
- Validate threshold boundary (source_count = 2)
- Ensure exact match bypasses source_count requirement

Graph edge cases:
- Cycles (prevent infinite loops)
- Empty graphs (no edges)
- Depth limits (max_depth = 2)
- More results than limit (top-N selection)

## Dependencies
- All previous Phase 1 and Phase 2 tickets (complete feature must exist)

## Risk Assessment
- **Risk**: Edge case not covered causes production failure
  - **Mitigation**: Comprehensive test suite + manual testing checklist
- **Risk**: Test expectations incorrect (test passes but behavior is wrong)
  - **Mitigation**: Review test logic; validate against architecture.md

## Files/Packages Affected
- `crates/maproom/tests/edge_cases_test.rs` (new file)
- `packages/daemon-client/src/types.test.ts` (add edge case tests)

## Verification Notes
The verify-ticket agent should check:
- All edge case tests pass: `cargo test edge_cases_test`
- At least 10 distinct edge cases covered
- Confidence gating tests validate threshold correctly
- Error handling tests validate graceful degradation
- Graph edge cases covered (cycles, empty, depth, limit)
- Empty result semantics tested (None vs Some([]))
- TypeScript edge case tests pass
- Manual test checklist from quality-strategy.md completed
