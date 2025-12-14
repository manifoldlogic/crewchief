# Quality Strategy: Confidence Scoring

## Testing Philosophy

**Test for confidence, not coverage**. Focus on critical paths that prove confidence scoring works correctly and doesn't break existing functionality. Avoid ceremonial testing—every test must validate actual risk.

**Key Principles**:
1. **Prove correctness** - Confidence math is correct for all inputs
2. **Prevent regressions** - Backward compatibility maintained
3. **Validate integration** - Rust-TypeScript boundary works
4. **Performance assurance** - <5ms overhead maintained

**Not Testing**:
- Trivial getters/setters
- Generated code
- Serde serialization (covered by roundtrip tests)
- Debug logging

## Test Types

### Unit Tests (Rust)

**Scope**: Confidence computation logic in `confidence.rs`

**Tools**:
- `cargo test` - Rust test framework
- `assert_eq!`, `assert!` - Standard assertions
- Property-based testing for edge cases

**Coverage Target**: 100% for `confidence.rs` module (small, critical)

**Test Cases**:

1. **`compute_result_confidence` - Normal Cases**:
   ```rust
   #[test]
   fn test_confidence_multiple_sources() {
       // Result appears in 3 sources
       let result = FusedResult {
           source_scores: hashmap! {
               SearchSource::FTS => 0.9,
               SearchSource::Vector => 0.85,
               SearchSource::Graph => 0.7,
           },
           score: 2.45,
           chunk_id: 123,
       };

       let confidence = compute_result_confidence(&result, &all_results, 0, Some(3.0));

       assert_eq!(confidence.source_count, 3);
       assert!(confidence.is_exact_match); // exact_mult = 3.0
       assert_eq!(confidence.relative_score, 1.0); // This is top result
       assert_eq!(confidence.rank, 1);
   }
   ```

2. **`compute_result_confidence` - Edge Cases**:
   - Empty source_scores HashMap
   - Last result in list (no score gap)
   - Single result (top_score == this score)
   - No exact match multiplier (None)
   - Zero top score (graceful degradation)

3. **`compute_query_confidence` - Normal Cases**:
   - Multiple results with varied confidence
   - All results have exact matches
   - Mix of exact/non-exact matches

4. **`compute_query_confidence` - Edge Cases**:
   - Empty results array
   - Single result
   - Result saturation (len == limit)
   - No saturation (len < limit)
   - All active sources (4/4)
   - Single active source (1/4)

**Minimum Test Count**: 10 unit tests (reduced for 3-field MVP)

### Type Sync Tests (TypeScript)

**Scope**: Rust → JSON → TypeScript type compatibility

**Tools**:
- Jest - TypeScript test framework
- `packages/daemon-client/src/types.test.ts`

**Coverage Target**: All new types (ConfidenceSignals, SearchConfidenceSummary)

**Test Cases**:

1. **ConfidenceSignals Serialization**:
   ```typescript
   it('should serialize ConfidenceSignals correctly', () => {
       const rustJson = {
           source_count: 3,
           score_gap: 1.25,
           is_exact_match: true,
           relative_score: 0.95,
           rank: 2
       };

       const signals: ConfidenceSignals = rustJson;

       expect(signals.source_count).toBe(3);
       expect(signals.score_gap).toBeCloseTo(1.25);
       expect(signals.is_exact_match).toBe(true);
       expect(signals.relative_score).toBeCloseTo(0.95);
       expect(signals.rank).toBe(2);
   });
   ```

2. **SearchConfidenceSummary Serialization**:
   - Verify all fields deserialize correctly
   - Test boolean fields (result_saturation)
   - Test float fields (coverage_ratio, avg_source_count, exact_match_ratio)

3. **Optional Field Handling**:
   - Confidence present in JSON → deserializes
   - Confidence absent in JSON → field is undefined (backward compat)

**Minimum Test Count**: 3 type sync tests (reduced for 3-field MVP)

**Note**: Type sync tests moved to Phase 1 (run earlier for faster validation)

### Integration Tests (Rust)

**Scope**: Confidence integrated into search pipeline

**Tools**:
- `cargo test --test integration_tests`
- Test database with indexed sample data

**Coverage Target**: Critical integration points only

**Test Cases**:

1. **Exact Match Detection Without Debug Mode**:
   ```rust
   #[tokio::test]
   async fn test_exact_match_detection_without_debug() {
       let options = SearchOptions::new(1, None, 10)
           .with_include_confidence(true)
           .with_debug(false); // Confidence should work without debug

       let results = search_pipeline.execute("exact_function_name", options).await?;

       assert!(results.results[0].confidence.is_some());
       let conf = results.results[0].confidence.unwrap();
       assert!(conf.is_exact_match); // Should detect exact match without debug mode
   }
   ```

2. **Search with Confidence Enabled**:
   ```rust
   #[tokio::test]
   async fn test_search_with_confidence() {
       let options = SearchOptions::new(1, None, 10)
           .with_include_confidence(true);

       let results = search_pipeline.execute("authenticate", options).await?;

       assert!(results.results[0].confidence.is_some());
       let conf = results.results[0].confidence.unwrap();
       assert!(conf.source_count >= 1);
       assert!(conf.score_gap >= 0.0);
       // is_exact_match tested separately above
   }
   ```

3. **Search with Confidence Disabled**:
   ```rust
   #[tokio::test]
   async fn test_search_without_confidence() {
       let options = SearchOptions::new(1, None, 10)
           .with_include_confidence(false);

       let results = search_pipeline.execute("authenticate", options).await?;

       assert!(results.results[0].confidence.is_none());
   }
   ```

4. **Default Parameter Behavior**:
   - Verify `include_confidence` defaults to `false` when not specified
   - Ensure backward compatibility for existing callers

**Minimum Test Count**: 4 integration tests (query summary deferred to Phase 2)

### End-to-End Tests (TypeScript/MCP)

**Scope**: MCP tool → daemon → search pipeline → confidence → response

**Tools**:
- Jest integration tests in `packages/maproom-mcp/tests/integration/`
- Real daemon process
- Test database

**Coverage Target**: Critical user paths only

**Test Cases**:

1. **MCP Search with Confidence**:
   ```typescript
   it('should return confidence when requested', async () => {
       const result = await search({
           query: 'authenticate',
           repo: 'test-repo',
           include_confidence: true
       });

       expect(result.hits[0].confidence).toBeDefined();
       expect(result.hits[0].confidence.source_count).toBeGreaterThan(0);
       expect(result.metadata.confidence_summary).toBeDefined();
   });
   ```

2. **Backward Compatibility**:
   ```typescript
   it('should work without include_confidence parameter', async () => {
       const result = await search({
           query: 'authenticate',
           repo: 'test-repo'
           // include_confidence omitted
       });

       expect(result.hits[0].confidence).toBeUndefined();
       expect(result.metadata.confidence_summary).toBeUndefined();
   });
   ```

3. **Confidence Signal Correctness**:
   - High confidence result (exact match, multiple sources)
   - Low confidence result (single source, no exact match)
   - Score gap decreases down result list

**Minimum Test Count**: 4 end-to-end tests (simplified for 3-field MVP)

## Critical Paths

The following paths MUST be tested before shipping:

### Critical Path 1: Confidence Computation Math
**Why**: Core value proposition—confidence signals must be accurate

**Tests**:
- `compute_result_confidence` with all edge cases
- `compute_query_confidence` with empty/full result sets
- Score gap calculation for first/middle/last results
- Relative score calculation with zero/non-zero top scores

**Pass Criteria**: All unit tests pass, no panics

### Critical Path 2: Rust-TypeScript Type Sync
**Why**: Breaking type sync breaks MCP consumers

**Tests**:
- ConfidenceSignals roundtrip (Rust → JSON → TypeScript)
- SearchConfidenceSummary roundtrip
- Optional field handling (presence/absence)

**Pass Criteria**: Type validation tests pass, no TypeScript compilation errors

### Critical Path 3: Backward Compatibility
**Why**: Cannot break existing MCP users

**Tests**:
- Search without `include_confidence` parameter works
- Confidence fields omitted from JSON when disabled
- Existing integration tests still pass

**Pass Criteria**: Zero regressions in existing test suite

### Critical Path 4: Performance
**Why**: >5ms overhead violates architecture requirement

**Tests**:
- Benchmark confidence computation overhead (moved to Phase 1)
- Measure p95 latency with confidence enabled
- Compare to baseline (without confidence)
- Measure on realistic corpus (1000+ files, 10,000+ chunks)

**Pass Criteria**: <5ms overhead, <50ms p95 total latency

**Note**: Benchmarking moved to Phase 1 (before integration) to catch performance issues early.

## Test Data Strategy

**Rust Unit Tests**: In-memory test data (FusedResult structs)
```rust
let test_results = vec![
    FusedResult { chunk_id: 1, score: 3.5, source_scores: hashmap!{ FTS => 0.9 } },
    FusedResult { chunk_id: 2, score: 2.2, source_scores: hashmap!{ FTS => 0.7, Vector => 0.8 } },
];
```

**Integration Tests**: Shared test database with indexed sample code
- Small corpus: ~50 chunks across 5 files
- Representative code: functions, classes, tests, docs
- Known expected results for confidence signals

**E2E Tests**: Test database created in test setup
- `beforeAll()` hook indexes test repository
- `afterAll()` hook cleans up
- Deterministic test data for reproducibility

**No Fixtures**: Generate test data programmatically (easier to maintain)

## Quality Gates

### Before Ticket Verification

**Automated Checks** (must pass):
- [ ] `cargo test -p crewchief-maproom` - All Rust unit tests pass
- [ ] `cargo clippy -p crewchief-maproom` - Zero clippy warnings
- [ ] `cargo fmt --check -p crewchief-maproom` - Code formatted
- [ ] `pnpm test packages/daemon-client` - Type sync tests pass
- [ ] `pnpm test packages/maproom-mcp` - MCP integration tests pass
- [ ] `pnpm typecheck` - Zero TypeScript errors

**Manual Checks** (verify-ticket agent):
- [ ] Confidence signals match expected values for test queries
- [ ] Documentation updated (if Phase 3)
- [ ] No performance regression detected

### Before Commit

**Required** (commit-ticket agent):
- [ ] All quality gates passed
- [ ] Acceptance criteria met (per ticket)
- [ ] Git commit message follows convention
- [ ] No debug logging or commented code

### Before Merge to Main

**CI Pipeline** (automated):
- [ ] All tests pass on CI
- [ ] Build succeeds for all platforms
- [ ] No security vulnerabilities detected
- [ ] Test coverage meets threshold (>80% for new code)

## Performance Testing Strategy

**Benchmark Approach**:
1. Baseline: Measure search latency without confidence (10 queries, p50/p95)
2. With Confidence: Measure search latency with confidence enabled (same 10 queries)
3. Overhead: Calculate difference (should be <5ms)

**Sample Queries**:
- Simple exact match: "authenticate"
- Multi-word concept: "user authentication"
- Code pattern: "async function"
- Low matches: "obscure_function_xyz"
- High matches: "test"

**Measurement**:
```rust
#[bench]
fn bench_confidence_overhead(b: &mut Bencher) {
    b.iter(|| {
        let start = Instant::now();
        compute_result_confidence(&test_result, &all_results, 0, Some(3.0));
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(1)); // <1ms per result
    });
}
```

**Pass Criteria**:
- Per-result computation: <1ms
- Total overhead for 20 results: <5ms
- P95 total search latency: <50ms

## Test Exclusions (Pragmatic Decisions)

**Not Testing**:
1. **Serde Serialization** - Covered by roundtrip tests, framework battle-tested
2. **HashMap/Vec Operations** - Stdlib, already tested by Rust team
3. **Trivial Accessors** - `confidence.source_count` getter, no logic
4. **Debug Mode Integration** - Already tested in existing debug mode tests
5. **Logging** - Non-functional, tested manually

**Why**: Testing stdlib or framework code wastes time without adding confidence

## Regression Prevention

**Strategy**:
1. **Pin exact match behavior**: Test that 3.0× multiplier → is_exact_match=true
2. **Pin score gap math**: Test gap = result[i].score - result[i+1].score
3. **Pin backward compat**: Integration test without include_confidence parameter
4. **Pin performance**: Benchmark overhead, fail CI if >5ms

**How**:
- Snapshot tests for complex outputs (Jest snapshots for JSON)
- Golden file tests for Rust (if needed)
- Performance regression tests in CI

## Quality Metrics (Post-Deployment)

**Track** (optional, Phase 2+):
- Test execution time trends (detect slow tests)
- Test flakiness rate (should be 0%)
- Coverage trends (should stay >80%)
- Bug density (bugs per 1000 lines of new code)

**Don't Track**:
- 100% coverage (vanity metric)
- Lines of test code (quantity ≠ quality)
- Test execution speed (unless critically slow)
