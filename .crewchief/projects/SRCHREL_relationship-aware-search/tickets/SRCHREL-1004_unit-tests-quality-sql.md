# Ticket: SRCHREL-1004 - Unit Tests for Quality SQL

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- test-engineer
- verify-ticket
- commit-ticket

## Summary

Create comprehensive unit tests for quality-weighted SQL query logic. Validate that production code edges score higher than test code edges, and that the feature flag toggle works correctly.

## Background

The quality-weighted SQL query implements the core algorithm: distinguishing production code from test code and applying appropriate weights. Unit tests must validate:
- Quality weights are applied correctly
- Test detection patterns work
- Logarithmic scaling is correct
- Feature flag toggle functions properly

## Acceptance Criteria

- [ ] Test: Production code edges score higher than test code edges
- [ ] Test: Logarithmic scaling applied correctly to quality-weighted sum
- [ ] Test: Feature flag=false uses legacy behavior
- [ ] Test: Feature flag=true uses quality-weighted behavior
- [ ] Test: Test detection patterns match file paths correctly
- [ ] Test: NULL edge counts handled gracefully (COALESCE)
- [ ] Test: Multiple edges from same source accumulate quality scores
- [ ] Test: Empty database returns empty results (no errors)
- [ ] All unit tests pass
- [ ] Code coverage >70% for modified database code

## Technical Requirements

**Test File Location:**
- `crates/maproom/tests/graph_quality_tests.rs` (new integration test file)
- OR unit tests in `crates/maproom/src/db/sqlite/mod.rs` test module

**Test Scenarios:**

**Test 1: Production vs Test Code Scoring**

```rust
#[tokio::test]
async fn test_production_code_scores_higher_than_test() {
    let store = setup_test_db().await;

    // Create two chunks with identical edge counts but different source types
    // Chunk A: 10 production code callers (from /src/ files)
    let chunk_a = create_chunk(&store, "ProductionHandler", "/src/handler.ts", 1).await;
    for i in 0..10 {
        let caller = create_chunk(&store, &format!("caller{}", i), "/src/caller.ts", 10 + i).await;
        create_edge(&store, caller, chunk_a, "calls").await;
    }

    // Chunk B: 10 test code callers (from /test/ files)
    let chunk_b = create_chunk(&store, "TestHelper", "/test/helper.ts", 2).await;
    for i in 0..10 {
        let test = create_chunk(&store, &format!("test{}", i), "/test/test.ts", 30 + i).await;
        create_edge(&store, test, chunk_b, "calls").await;
    }

    // Calculate graph importance with quality enabled
    let scores = store.calculate_graph_importance(1, None, 10, true).await.unwrap();

    // Find scores for our chunks
    let score_a = scores.iter().find(|(id, _)| *id == chunk_a).map(|(_, s)| *s);
    let score_b = scores.iter().find(|(id, _)| *id == chunk_b).map(|(_, s)| *s);

    assert!(score_a.is_some() && score_b.is_some());
    assert!(
        score_a.unwrap() > score_b.unwrap(),
        "Production code should score higher: {} vs {}",
        score_a.unwrap(),
        score_b.unwrap()
    );

    // Expected scores:
    // Chunk A: LOG(2 + 10*1.0) = LOG(12) ≈ 2.48
    // Chunk B: LOG(2 + 10*0.5) = LOG(7) ≈ 1.95
}
```

**Test 2: Logarithmic Scaling**

```rust
#[tokio::test]
async fn test_logarithmic_scaling() {
    let store = setup_test_db().await;

    // Chunk with known edge quality
    // 5 production callers → quality sum = 5.0 → LOG(2 + 5) = LOG(7) ≈ 1.946
    let chunk = create_chunk(&store, "Function", "/src/func.ts", 1).await;
    for i in 0..5 {
        let caller = create_chunk(&store, &format!("caller{}", i), "/src/caller.ts", 10 + i).await;
        create_edge(&store, caller, chunk, "calls").await;
    }

    let scores = store.calculate_graph_importance(1, None, 10, true).await.unwrap();
    let score = scores.iter().find(|(id, _)| *id == chunk).map(|(_, s)| *s).unwrap();

    // LOG(2 + 5.0) = LOG(7) = 1.9459...
    let expected = (7.0_f32).ln();
    assert!(
        (score - expected).abs() < 0.01,
        "Score {} should be close to {}",
        score,
        expected
    );
}
```

**Test 3: Feature Flag Toggle**

```rust
#[tokio::test]
async fn test_feature_flag_toggle() {
    let store = setup_test_db().await;

    // Create mixed test/production scenario
    let chunk = create_chunk(&store, "Function", "/src/func.ts", 1).await;
    create_chunk_and_edge(&store, "ProdCaller", "/src/caller.ts", chunk, "calls").await;
    create_chunk_and_edge(&store, "TestCaller", "/test/test.ts", chunk, "calls").await;

    // Get scores with flag=false (legacy)
    let legacy_scores = store.calculate_graph_importance(1, None, 10, false).await.unwrap();
    let legacy_score = legacy_scores.iter().find(|(id, _)| *id == chunk).map(|(_, s)| *s);

    // Get scores with flag=true (quality)
    let quality_scores = store.calculate_graph_importance(1, None, 10, true).await.unwrap();
    let quality_score = quality_scores.iter().find(|(id, _)| *id == chunk).map(|(_, s)| *s);

    // Scores should differ
    assert_ne!(legacy_score, quality_score);

    // Legacy: LOG(2 + 2) = LOG(4) ≈ 1.386
    // Quality: LOG(2 + 1.0 + 0.5) = LOG(3.5) ≈ 1.253
    assert!(quality_score.unwrap() < legacy_score.unwrap());
}
```

**Test 4: Test Detection Patterns**

```rust
#[test]
fn test_file_path_patterns() {
    let test_paths = vec![
        "/src/test/helper.ts",
        "/src/tests/utils.ts",
        "/src/__tests__/component.test.tsx",
        "/src/auth.test.ts",
        "/src/validator.spec.js",
        "/crates/maproom/tests/integration_test.rs",
        "/lib/utils_test.py",
    ];

    let production_paths = vec![
        "/src/handler.ts",
        "/lib/utils.ts",
        "/crates/maproom/src/db.rs",
        "/packages/cli/src/index.ts",
    ];

    // Test that all test paths are detected
    for path in test_paths {
        assert!(is_test_path(path), "Should detect test path: {}", path);
    }

    // Test that production paths are not misidentified
    for path in production_paths {
        assert!(!is_test_path(path), "Should not detect test path: {}", path);
    }
}

fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.js")
        || lower.ends_with(".test.tsx")
        || lower.ends_with(".test.jsx")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.js")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_test.py")
}
```

**Test 5: NULL Handling**

```rust
#[tokio::test]
async fn test_null_edge_count_handling() {
    let store = setup_test_db().await;

    // Chunk with no edges
    let isolated_chunk = create_chunk(&store, "Isolated", "/src/isolated.ts", 1).await;

    let scores = store.calculate_graph_importance(1, None, 10, true).await.unwrap();

    // Isolated chunk may or may not appear in results (depending on implementation)
    // If it appears, score should be LOG(2 + 0) = LOG(2) ≈ 0.693
    if let Some((_, score)) = scores.iter().find(|(id, _)| *id == isolated_chunk) {
        let expected = 2.0_f32.ln();
        assert!(
            (score - expected).abs() < 0.01,
            "Isolated chunk score should be LOG(2): {}",
            score
        );
    }
}
```

**Test Helpers:**

```rust
async fn setup_test_db() -> SqliteStore {
    // Create in-memory SQLite database for testing
    let store = SqliteStore::new(":memory:").await.unwrap();
    // Run schema migrations
    store.run_migrations().await.unwrap();
    store
}

async fn create_chunk(
    store: &SqliteStore,
    name: &str,
    file_path: &str,
    id: i64,
) -> i64 {
    // Insert file and chunk into test database
    // Return chunk ID
    // ...
}

async fn create_edge(
    store: &SqliteStore,
    src_chunk_id: i64,
    dst_chunk_id: i64,
    edge_type: &str,
) {
    // Insert edge into chunk_edges table
    // ...
}
```

## Implementation Notes

**Test Database Setup:**
- Use in-memory SQLite database (`:memory:`)
- Run schema migrations to create tables
- Populate with synthetic test data
- Clean up after each test (automatic with in-memory DB)

**Coverage Target:**
- Edge quality computation: 100% (critical logic)
- Feature flag toggle: 100% (safety critical)
- Overall modified code: >70%

**Parameterized Tests:**
Consider using `rstest` for parameterized tests:
```rust
use rstest::rstest;

#[rstest]
#[case("/src/test/file.ts", true)]
#[case("/src/production.ts", false)]
#[case("/tests/unit.rs", true)]
fn test_path_detection(#[case] path: &str, #[case] expected: bool) {
    assert_eq!(is_test_path(path), expected);
}
```

## Dependencies

**Prerequisites:**
- SRCHREL-1001 (database query implementation)
- SRCHREL-1002 (feature flag exists)
- SRCHREL-1003 (executor updated)

**Blocks:**
- SRCHREL-1005 (integration tests build on unit tests)

## Risk Assessment

**Risk:** Test database setup too complex
**Mitigation:** Use helper functions, consider test fixtures

**Risk:** Tests flaky due to timing or database state
**Mitigation:** Use in-memory database, ensure tests are isolated

**Risk:** Coverage metrics misleading
**Mitigation:** Focus on testing behavior, not just lines of code

## Files/Packages Affected

**New Files:**
- `crates/maproom/tests/graph_quality_tests.rs` (new test file)

**Test Helpers:**
- `crates/maproom/tests/helpers/mod.rs` (test database setup utilities)

**Dependencies:**
- `rstest` (optional, for parameterized tests)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 1.4, lines 205-214)
- Quality Strategy: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/quality-strategy.md` (Unit tests, lines 13-46)
