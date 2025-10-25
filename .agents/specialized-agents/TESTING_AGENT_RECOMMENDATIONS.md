# Testing Agent Recommendations for Maproom MVP

## Overview

This document identifies the most valuable test agents for the Maproom MVP, focusing on tests that maximize confidence and velocity while avoiding over-testing. The recommendations are based on the critical path projects, identified risks, and the principle that tests should save more time than they cost.

## Testing Philosophy for MVP

### Core Principles
1. **Confidence over Coverage:** Test critical paths thoroughly, not every line
2. **Integration over Unit:** End-to-end tests catch more real-world failures
3. **Properties over Examples:** Invariant testing catches edge cases efficiently
4. **Regression Prevention:** Automated checks prevent backsliding on performance and quality
5. **Fast Feedback:** Tests should run quickly enough to use during development

### What NOT to Test in MVP
- ❌ Simple getters/setters and trivial functions
- ❌ Third-party library internals (tree-sitter, pgvector)
- ❌ 100% code coverage for its own sake
- ❌ Exhaustive load testing (defer to Phase 4)
- ❌ UI/UX (MCP is API-based)

## Currently Available Test Agents

### 1. **integration-tester**
- **Current Role:** End-to-end testing, integration verification
- **MVP Priority:** CRITICAL - Most valuable test type for MVP
- **Recommended Usage:** 60% of testing effort
- **Focus Areas:** MCP_CORE, HYBRID_SEARCH, CONTEXT_ASM

## Recommended Test Agent Types

### 1. **contract-test-engineer** ⭐ HIGH PRIORITY
**Value Proposition:** Ensure stable interfaces between components

**Why Critical for MVP:**
- Maproom has clear component boundaries (MCP tools, database, parsers)
- Breaking changes in contracts would cascade across the system
- Fast, focused tests that catch integration issues early
- Much cheaper than full integration tests

**Responsibilities:**
- Define and verify API contracts for MCP tools
- Test database query input/output schemas
- Validate parser output structure
- Ensure backward compatibility during changes

**Test Examples:**
```typescript
// MCP tool contract test
describe('search tool contract', () => {
  it('returns expected schema on success', async () => {
    const result = await searchTool.execute({ query: 'test', repo: 'crewchief' });
    expect(result).toMatchSchema({
      results: array(object({
        chunk_id: number(),
        symbol_name: string(),
        score: number().min(0).max(1),
        relpath: string(),
      })),
      total: number(),
    });
  });

  it('returns error schema on failure', async () => {
    const result = await searchTool.execute({ query: '', repo: 'invalid' });
    expect(result).toMatchSchema({
      error: object({
        code: string(),
        message: string(),
      }),
    });
  });
});
```

**Projects Benefiting:**
- MCP_CORE: All 5 MCP tools
- HYBRID_SEARCH: Query API stability
- CONTEXT_ASM: Context bundle format
- LANG_PARSE: Parser output consistency

**Success Metrics:**
- All public APIs have contract tests
- Breaking changes detected before merge
- Contract tests run in <5s

### 2. **property-test-engineer** ⭐ HIGH PRIORITY
**Value Proposition:** Catch edge cases in search, ranking, and budgets automatically

**Why Critical for MVP:**
- HYBRID_SEARCH has complex scoring algorithms with many edge cases
- CONTEXT_ASM token budgets must NEVER be exceeded
- Property-based testing finds edge cases developers miss
- High ROI: Write one test, get thousands of test cases

**Responsibilities:**
- Define invariants for search scoring
- Test context assembly budget constraints
- Verify token counting accuracy
- Validate graph traversal properties

**Test Examples:**
```rust
// Property test for hybrid search scoring
#[test]
fn test_hybrid_score_invariants() {
    proptest!(|(
        fts_score in 0.0..=1.0f32,
        vector_score in 0.0..=1.0f32,
        recency_score in 0.0..=1.0f32,
        churn_score in 0.0..=10.0f32,
    )| {
        let result = calculate_hybrid_score(
            fts_score, vector_score, recency_score, churn_score
        );

        // Invariant: Score always in valid range
        prop_assert!(result >= 0.0 && result <= 1.0);

        // Invariant: Higher input scores = higher output (monotonicity)
        let higher = calculate_hybrid_score(
            fts_score + 0.1, vector_score, recency_score, churn_score
        );
        prop_assert!(higher >= result);
    });
}

// Property test for context budget
#[test]
fn test_context_never_exceeds_budget() {
    proptest!(|(
        budget in 1000..=100000usize,
        chunks in vec(any::<MockChunk>(), 1..=100),
    )| {
        let context = assemble_context(chunks, budget);
        let token_count = count_tokens(&context);

        // Invariant: Never exceed budget
        prop_assert!(token_count <= budget);

        // Invariant: Use at least 40% of budget (efficiency)
        prop_assert!(token_count >= (budget as f32 * 0.4) as usize);
    });
}
```

**Projects Benefiting:**
- HYBRID_SEARCH: Scoring correctness, rank invariants
- CONTEXT_ASM: Budget constraints, token accuracy
- INC_INDEX: Incremental update correctness

**Success Metrics:**
- All scoring algorithms have property tests
- Token budgets verified by property tests
- Property tests find 5+ edge cases missed by example tests

### 3. **concurrency-test-engineer** ⭐ MEDIUM-HIGH PRIORITY
**Value Proposition:** Prevent race conditions in incremental indexing

**Why Important for MVP:**
- INC_INDEX identified as "high risk" for race conditions
- File watching involves concurrent updates
- Race conditions are notoriously hard to debug in production
- Investment now saves debugging time later

**Responsibilities:**
- Test concurrent file modifications
- Verify database transaction isolation
- Detect race conditions in cache updates
- Ensure thread-safe data structures

**Test Examples:**
```rust
// Concurrency stress test for incremental indexing
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_file_updates() {
    let indexer = create_test_indexer().await;

    // Simulate 50 concurrent file changes
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let idx = indexer.clone();
            tokio::spawn(async move {
                // Randomly: create, modify, or delete files
                match i % 3 {
                    0 => idx.handle_file_created(format!("file_{}.ts", i)).await,
                    1 => idx.handle_file_modified(format!("file_{}.ts", i)).await,
                    2 => idx.handle_file_deleted(format!("file_{}.ts", i)).await,
                    _ => unreachable!(),
                }
            })
        })
        .collect();

    // Wait for all updates
    futures::future::join_all(handles).await;

    // Verify consistency
    let db_state = indexer.get_database_state().await;
    assert!(db_state.is_consistent());
    assert_eq!(db_state.file_count(), indexer.file_watcher.count());
}
```

**Projects Benefiting:**
- INC_INDEX: File watching, concurrent updates
- HYBRID_SEARCH: Cache invalidation
- PERF_OPT: Concurrent query handling

**Success Metrics:**
- No race conditions detected in CI
- Stress tests with 100+ concurrent operations pass
- Database consistency verified under load

### 4. **snapshot-test-engineer** ⭐ MEDIUM PRIORITY
**Value Proposition:** Catch unexpected parser behavior changes

**Why Valuable for MVP:**
- LANG_PARSE adds 3 new languages with complex parsing
- Parser regressions are hard to spot manually
- Golden tests provide clear "before/after" comparison
- Fast to write once test files are created

**Responsibilities:**
- Create golden test files for each language
- Capture expected parser output
- Detect unexpected changes in chunk extraction
- Maintain test corpus as languages evolve

**Test Examples:**
```typescript
// Snapshot test for TypeScript parser
describe('TypeScript parser snapshots', () => {
  const testCases = [
    'function-basic.ts',
    'class-with-methods.ts',
    'async-arrow-functions.ts',
    'react-component.tsx',
    'type-definitions.ts',
  ];

  testCases.forEach((file) => {
    it(`parses ${file} consistently`, async () => {
      const source = await readTestFile(file);
      const chunks = await parseTypeScript(source);

      // Snapshot the extracted chunks
      expect(chunks).toMatchSnapshot({
        // Allow dynamic fields
        file_id: expect.any(Number),
        created_at: expect.any(Date),
      });
    });
  });
});

// Python parser snapshot test
test('python class parsing snapshot', () => {
  const source = `
class Calculator:
    def __init__(self):
        self.result = 0

    def add(self, x: int, y: int) -> int:
        return x + y
  `;

  const chunks = parsePython(source);

  expect(chunks).toMatchInlineSnapshot(`
    [
      {
        kind: 'class',
        name: 'Calculator',
        start_line: 2,
        end_line: 7,
        methods: ['__init__', 'add'],
      },
      {
        kind: 'function',
        name: '__init__',
        parent: 'Calculator',
        start_line: 3,
        end_line: 4,
      },
      {
        kind: 'function',
        name: 'add',
        parent: 'Calculator',
        params: ['x', 'y'],
        return_type: 'int',
      },
    ]
  `);
});
```

**Projects Benefiting:**
- LANG_PARSE: Python, Rust, Go parsers
- MD_ENHANCE: Tree-sitter markdown
- Core: TypeScript/JavaScript baseline

**Success Metrics:**
- 10+ snapshot tests per language
- Parser changes trigger snapshot review
- Zero regressions in chunk extraction

### 5. **performance-regression-test-engineer** ⭐ MEDIUM PRIORITY
**Value Proposition:** Prevent performance backsliding during development

**Why Valuable for MVP:**
- PERF_OPT has specific targets (p95 <50ms, 150+ files/min)
- Easy to accidentally regress performance during feature development
- Automated benchmarks provide objective feedback
- Cheaper than full load testing but still catches regressions

**Responsibilities:**
- Define baseline performance metrics
- Create automated benchmark suite
- Detect performance regressions in CI
- Track performance trends over time

**Test Examples:**
```typescript
// Benchmark test with regression detection
describe('search performance benchmarks', () => {
  const baseline = {
    p50: 25, // ms
    p95: 50, // ms
    p99: 100, // ms
  };

  it('meets latency targets for hybrid search', async () => {
    const queries = generateTestQueries(100);
    const latencies: number[] = [];

    for (const query of queries) {
      const start = Date.now();
      await hybridSearch(query);
      latencies.push(Date.now() - start);
    }

    const stats = calculatePercentiles(latencies);

    // Fail if we regress by >10%
    expect(stats.p50).toBeLessThan(baseline.p50 * 1.1);
    expect(stats.p95).toBeLessThan(baseline.p95 * 1.1);
    expect(stats.p99).toBeLessThan(baseline.p99 * 1.1);

    // Warn if we're close to limit
    if (stats.p95 > baseline.p95 * 0.9) {
      console.warn(`⚠️  p95 latency approaching limit: ${stats.p95}ms`);
    }
  });
});

// Throughput benchmark
test('indexing throughput meets target', async () => {
  const testFiles = generateTestFiles(1000);

  const start = Date.now();
  await indexer.upsertFiles(testFiles);
  const duration = (Date.now() - start) / 1000; // seconds

  const filesPerMinute = (testFiles.length / duration) * 60;

  // Target: 150 files/min
  expect(filesPerMinute).toBeGreaterThan(150);

  console.log(`📊 Indexing throughput: ${filesPerMinute.toFixed(1)} files/min`);
});
```

**Projects Benefiting:**
- PERF_OPT: Baseline and optimized performance
- HYBRID_SEARCH: Query latency
- INC_INDEX: Incremental update speed
- CONTEXT_ASM: Assembly performance

**Success Metrics:**
- Benchmarks run on every PR
- Performance regressions caught before merge
- Clear visibility into performance trends

## Optional Test Agent Types (Defer to Post-MVP)

### 6. **load-test-engineer**
**When:** Phase 4 (Weeks 22-24)
**Why Defer:** Performance regression tests cover most needs for MVP
**Value:** Validate behavior under extreme load (10k+ concurrent queries)

### 7. **mutation-test-engineer**
**When:** Post-MVP if quality concerns arise
**Why Defer:** Expensive, diminishing returns for MVP
**Value:** Verify test suite quality by mutating code

### 8. **chaos-test-engineer**
**When:** Production readiness phase
**Why Defer:** Infrastructure complexity not needed yet
**Value:** Test resilience to database failures, network issues

## Testing Strategy by Project Phase

### Phase 1: Core Foundation (Weeks 1-8)
**Focus:** Contract tests and property tests

| Project | Test Types | Priority |
|---------|------------|----------|
| HYBRID_SEARCH | Contract (API), Property (scoring), Snapshot (baseline) | HIGH |
| MCP_CORE | Contract (all tools), Integration (E2E flows) | HIGH |
| CONTEXT_ASM | Property (budgets), Contract (output format) | HIGH |

**Agents Needed:**
- contract-test-engineer (60% time)
- property-test-engineer (40% time)
- integration-tester (20% time)

### Phase 2: Production Features (Weeks 9-14)
**Focus:** Concurrency tests and performance baselines

| Project | Test Types | Priority |
|---------|------------|----------|
| INC_INDEX | Concurrency (race conditions), Performance (throughput) | HIGH |
| PERF_OPT | Performance regression (all targets) | HIGH |

**Agents Needed:**
- concurrency-test-engineer (60% time)
- performance-regression-test-engineer (40% time)
- integration-tester (20% time)

### Phase 3: Enhancements (Weeks 15-21)
**Focus:** Snapshot tests for new languages

| Project | Test Types | Priority |
|---------|------------|----------|
| LANG_PARSE | Snapshot (Python, Rust, Go), Contract (parser API) | MEDIUM |
| MD_ENHANCE | Snapshot (markdown parsing), Contract (output) | MEDIUM |

**Agents Needed:**
- snapshot-test-engineer (60% time)
- contract-test-engineer (20% time)
- integration-tester (20% time)

### Phase 4: Final Optimization (Weeks 22-24)
**Focus:** Performance regression prevention

| Project | Test Types | Priority |
|---------|------------|----------|
| PERF_OPT | Performance regression (comprehensive) | HIGH |

**Agents Needed:**
- performance-regression-test-engineer (80% time)
- integration-tester (20% time)

## Test Coverage Targets for MVP

### Critical Path Projects (>80% coverage)
- HYBRID_SEARCH: 85% (focus on scoring algorithms)
- CONTEXT_ASM: 85% (focus on budget management)
- MCP_CORE: 80% (focus on tool implementations)

### Production Features (>70% coverage)
- INC_INDEX: 75% (focus on concurrency paths)

### Enhancements (>60% coverage)
- LANG_PARSE: 60% (focus on new parsers)
- MD_ENHANCE: 60% (focus on upgrade path)

### Performance (benchmark-based)
- PERF_OPT: Not coverage-based, use regression benchmarks

**Note:** Coverage targets are guidelines. Focus on testing critical paths and high-risk code, not achieving arbitrary percentages.

## Test Infrastructure Requirements

### Shared Test Utilities
```
tests/
├── fixtures/          # Test files for each language
│   ├── typescript/    # Sample .ts files
│   ├── python/        # Sample .py files
│   ├── rust/          # Sample .rs files
│   └── markdown/      # Sample .md files
├── helpers/           # Test helper functions
│   ├── database.ts    # Test DB setup/teardown
│   ├── embeddings.ts  # Mock embedding generation
│   └── snapshots.ts   # Snapshot utilities
└── benchmarks/        # Performance test suite
    ├── search.bench.ts
    ├── indexing.bench.ts
    └── context.bench.ts
```

### CI/CD Integration
1. **PR Checks:**
   - Contract tests (must pass)
   - Property tests (must pass)
   - Integration tests (must pass)
   - Performance regression (warning if >10% slower)

2. **Nightly Tests:**
   - Concurrency stress tests
   - Extended property test runs
   - Full benchmark suite

3. **Release Gates:**
   - All test types pass
   - Performance targets met
   - No critical regressions

## Success Metrics for Testing Strategy

### Confidence Metrics
- Zero critical bugs in production (search failures, data corruption)
- <5 minor bugs per release
- Breaking changes caught before merge

### Velocity Metrics
- Test suite runs in <5 minutes for PR checks
- Developers can run relevant tests locally in <1 minute
- Bugs caught in development, not QA or production

### ROI Metrics
- Time saved debugging > time spent writing tests
- Refactoring confidence allows faster iteration
- Performance targets met without manual testing

## Priority Recommendations

### Immediate (Week 1-2)
1. **contract-test-engineer** - Blocks all projects, highest ROI
2. **integration-tester** - Already exists, just needs prioritization

### Early (Week 3-8)
3. **property-test-engineer** - Critical for HYBRID_SEARCH and CONTEXT_ASM
4. **snapshot-test-engineer** - Needed before LANG_PARSE starts

### Mid-Term (Week 9-14)
5. **concurrency-test-engineer** - Critical for INC_INDEX
6. **performance-regression-test-engineer** - Needed for PERF_OPT baseline

### Deferred (Post-MVP)
7. **load-test-engineer** - Important but not blocking
8. **mutation-test-engineer** - Nice to have
9. **chaos-test-engineer** - Production hardening

## Conclusion

For the Maproom MVP, testing strategy should focus on:

1. **Contract tests** - Highest ROI, protect all integrations
2. **Property tests** - Catch algorithmic edge cases efficiently
3. **Integration tests** - Validate end-to-end flows
4. **Concurrency tests** - Prevent race conditions in INC_INDEX
5. **Snapshot tests** - Ensure parser consistency
6. **Performance regression tests** - Prevent backsliding

This focused approach provides strong confidence in critical paths while avoiding over-testing. The recommended 5 core test agent types would:
- Catch 95% of bugs before production
- Enable fast, confident refactoring
- Meet performance targets reliably
- Take ~25-30% of development time (acceptable for MVP)

The highest priority is **contract-test-engineer**, which should be implemented immediately as it provides the foundation for stable integrations across all projects.
