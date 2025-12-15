# Ticket: SRCHREL-3004 - Edge Case Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- test-engineer
- verify-ticket
- commit-ticket

## Summary

Test edge cases and unusual scenarios to ensure quality-weighted scoring handles corner cases gracefully without errors or unexpected behavior.

## Acceptance Criteria

- [ ] Test with empty database (no edges)
- [ ] Test with database containing only test code
- [ ] Test with database containing only production code
- [ ] Test with chunks having 0 edges (isolated code)
- [ ] Test with chunks having 1000+ edges (hub nodes)
- [ ] Test with malformed file paths (missing extensions, unusual characters)
- [ ] Test with NULL chunk kinds
- [ ] Test with extreme weight values (0.0, 10.0)
- [ ] Test with invalid configurations (negative weights)
- [ ] All edge case tests pass without errors
- [ ] Document expected behavior for each edge case

## Technical Requirements

**Edge Case Test Suite:**

```rust
#[tokio::test]
async fn test_empty_database() {
    let store = setup_empty_db().await;
    let config = SearchConfig::default_with_quality();

    let result = GraphExecutor::execute(&store, 1, None, 10, Some(&config)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().chunks.len(), 0);
}

#[tokio::test]
async fn test_only_test_code_in_database() {
    let store = setup_test_db().await;
    // Populate with only test files
    populate_with_test_code_only(&store).await;

    let result = GraphExecutor::execute(&store, 1, None, 10, Some(&config)).await;

    assert!(result.is_ok());
    // Should still return results, just with lower scores
}

#[tokio::test]
async fn test_isolated_chunk_no_edges() {
    let store = setup_test_db().await;
    let isolated = create_chunk_with_no_edges(&store).await;

    let scores = store.calculate_graph_importance(1, None, 100, true).await.unwrap();

    // Isolated chunk may or may not appear (depends on implementation)
    // If appears, score should be LOG(2) ≈ 0.693
}

#[tokio::test]
async fn test_hub_node_many_edges() {
    let store = setup_test_db().await;
    let hub = create_chunk_with_1000_edges(&store).await;

    let scores = store.calculate_graph_importance(1, None, 100, true).await.unwrap();

    // Hub should score high but not infinite
    let hub_score = scores.iter().find(|(id, _)| *id == hub).map(|(_, s)| *s);
    assert!(hub_score.is_some());
    assert!(hub_score.unwrap() < 10.0, "Score should not be extreme");
}

#[test]
fn test_malformed_file_paths() {
    // Test paths that might break regex/LIKE patterns
    let paths = vec![
        "/src/file with spaces.ts",
        "/src/file%20encoded.ts",
        "/src/файл.ts", // Unicode
        "/src/.hidden.ts",
        "relative/path.ts", // Relative path
        "/src/no_extension",
    ];

    for path in paths {
        let is_test = is_test_path(path);
        // Should not panic, should return boolean
        assert!(is_test == true || is_test == false);
    }
}

#[test]
fn test_invalid_config_rejected() {
    let invalid_yaml = r#"
graph_importance:
  edge_quality_weights:
    production_code: -1.0  # Invalid: negative
    "#;

    let result: Result<SearchConfig, _> = serde_yaml::from_str(invalid_yaml);
    assert!(result.is_err() || result.unwrap().graph_importance.edge_quality_weights.validate().is_err());
}

#[test]
fn test_extreme_weight_values() {
    let mut weights = EdgeQualityWeights::default();

    // Test boundary values
    weights.test_code = 0.0;
    assert!(weights.validate().is_ok());

    weights.test_code = 10.0;
    assert!(weights.validate().is_ok());

    weights.test_code = 10.1;
    assert!(weights.validate().is_err(), "Should reject weight >10");

    weights.test_code = -0.1;
    assert!(weights.validate().is_err(), "Should reject negative weight");
}
```

## Dependencies

**Prerequisites:**
- All Phase 1 and Phase 2 tickets complete

**Blocks:**
- None (final testing before rollout)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Phase 3, line 345)
- Quality Strategy: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/quality-strategy.md` (Edge case testing mentioned)
