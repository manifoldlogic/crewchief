# Quality Strategy: Semantic Entry Point Ranking

## Testing Philosophy

**Goal:** Ship with confidence that search returns correct entry points, not ceremonial test coverage.

**Approach:**
- Integration tests with real codebase samples
- Regression tests for known failure cases
- Performance benchmarks to prevent degradation
- No unit tests for simple CASE statements (waste of time)

## Critical Paths

### 1. Search Ranking Correctness

**Risk:** New scoring breaks search quality.

**Mitigation:**
- Before/after comparison on test corpus
- Golden dataset: Known queries with expected top results
- Automated validation of ranking improvements

**Test Coverage:**
- ✅ Implementation ranks above test for exact symbol match
- ✅ Implementation ranks above docs for concept search
- ✅ Exact match boost applies correctly
- ✅ Kind multipliers apply to all chunk types
- ✅ Edge cases (null symbol_name, unknown kind)

### 2. Performance & Latency

**Risk:** SQL complexity slows search unacceptably.

**Mitigation:**
- Benchmark before/after with realistic dataset sizes
- Target: <10ms latency increase for p95
- Load test with 100+ concurrent queries

**Test Coverage:**
- ✅ Query latency within acceptable bounds
- ✅ No N+1 query issues
- ✅ Database query plan is optimal

### 3. Backward Compatibility

**Risk:** Breaking existing search workflows.

**Mitigation:**
- Existing FTS tests continue to pass
- Search API unchanged (no parameter changes)
- Results may re-order, but all valid results still returned

**Test Coverage:**
- ✅ All existing search integration tests pass
- ✅ Search API contract unchanged
- ✅ Debug mode returns score breakdown

## Test Pyramid

### Level 1: Integration Tests (Primary Focus)

**Purpose:** Validate real-world search quality with actual codebase.

**Test Corpus:** Sample repository with known structure across 3 languages (Rust, TypeScript, Python).

**Specification (from plan.md SEMRANK-1003):**
- **Languages**: Rust, TypeScript, Python (3 total)
- **Structure per language**: 5 functions + 3 tests + 2 docs = 10 chunks
- **Total**: 30 chunks minimum, 50 chunks maximum
- **Time box**: 1 day maximum creation time
- **Fallback**: Use existing maproom codebase subset if creation exceeds time

**Example Structure:**
```
test-repo/
├── rust/
│   ├── src/
│   │   ├── auth/authenticate.rs        // fn authenticate() [kind=func]
│   │   ├── auth/validate_token.rs      // fn validate_token() [kind=func]
│   │   └── db/connection.rs            // fn connect_db() [kind=func]
│   ├── tests/
│   │   ├── auth_test.rs                // test_authenticate() [kind=func, in tests/]
│   │   └── db_test.rs                  // test_connect_db() [kind=func, in tests/]
│   └── docs/
│       └── auth_guide.md               // Documentation [kind=heading_1/heading_2]
├── typescript/
│   ├── src/
│   │   ├── auth.ts                     // function authenticate() [kind=func]
│   │   └── hooks/useAuth.ts            // function useAuth() [kind=hook]
│   ├── __tests__/
│   │   └── auth.test.ts                // test functions
│   └── README.md                       // Documentation
└── python/
    ├── src/
    │   ├── auth.py                     // def authenticate() [kind=func]
    │   └── db.py                       // def connect_db() [kind=func]
    ├── tests/
    │   └── test_auth.py                // test functions
    └── docs/
        └── api.md                      // Documentation
```

**Minimal Viable Approach:**
- Create representative samples, NOT full applications
- Use simple, self-contained functions to avoid dependency complexity
- Prioritize variety in chunk kinds over depth of functionality

**Test Cases:**

1. **Exact Function Name Match:**
   ```typescript
   test('search for "authenticate" returns implementation first', async () => {
     const results = await search({ query: 'authenticate', repo: 'test-repo' });

     expect(results[0].symbol_name).toBe('authenticate');
     expect(results[0].kind).toBe('function');
     expect(results[0].relpath).toContain('src/auth/authenticate.rs');
     expect(results[0].relpath).not.toContain('test');
   });
   ```

2. **Implementation Over Test:**
   ```typescript
   test('implementation ranks higher than test with same term frequency', async () => {
     const results = await search({ query: 'validate_token', repo: 'test-repo' });

     const implRank = results.findIndex(r => r.kind === 'function' && r.relpath.includes('src/'));
     const testRank = results.findIndex(r => r.kind === 'test' && r.relpath.includes('tests/'));

     expect(implRank).toBeLessThan(testRank);
   });
   ```

3. **Implementation Over Documentation:**
   ```typescript
   test('implementation ranks higher than documentation', async () => {
     const results = await search({ query: 'authenticate', repo: 'test-repo' });

     const implRank = results.findIndex(r => r.kind === 'function');
     const docRank = results.findIndex(r => r.relpath.includes('docs/'));

     expect(implRank).toBeLessThan(docRank);
   });
   ```

4. **Score Breakdown (Debug Mode):**
   ```typescript
   test('debug mode shows score breakdown', async () => {
     const results = await search({
       query: 'authenticate',
       repo: 'test-repo',
       debug: true
     });

     expect(results[0].score_breakdown).toBeDefined();
     expect(results[0].score_breakdown).toMatchObject({
       base_fts: expect.any(Number),
       kind_multiplier: expect.any(Number),
       exact_match_multiplier: expect.any(Number),
       final: expect.any(Number)
     });
   });
   ```

5. **Edge Cases:**
   ```typescript
   test('handles null symbol_name gracefully', async () => {
     // Docs/comments have no symbol_name
     const results = await search({ query: 'authentication guide', repo: 'test-repo' });
     expect(() => results).not.toThrow();
   });

   test('handles unknown kind gracefully', async () => {
     // Some chunks may have kind = null or unknown
     const results = await search({ query: 'config', repo: 'test-repo' });
     expect(() => results).not.toThrow();
   });

   test('case-insensitive exact match works', async () => {
     const lower = await search({ query: 'authenticate', repo: 'test-repo' });
     const upper = await search({ query: 'AUTHENTICATE', repo: 'test-repo' });
     const camel = await search({ query: 'Authenticate', repo: 'test-repo' });

     expect(lower[0].id).toBe(upper[0].id);
     expect(lower[0].id).toBe(camel[0].id);
   });
   ```

**Test Data Setup:**
- Index test-repo before test suite runs
- Seed database with known chunks and metadata
- Use real tree-sitter parsing (no mocks)

### Level 2: Performance Benchmarks

**Purpose:** Prevent performance regressions.

**Approach:**
- Measure latency before/after code changes
- Compare against baseline (current FTS without enhancements)
- Fail CI if p95 latency increases >10%

**Baseline Measurement Methodology (from plan.md SEMRANK-1005):**

**Golden Query Set:**
- 20 representative queries across languages (Rust, TypeScript, Python)
- Mix of exact function names and concept searches
- Examples:
  - `"authenticate"` (exact function)
  - `"HTTP handler"` (concept)
  - `"database connection"` (concept)
  - `"XMLParser"` (acronym test)
  - `"useAuth"` (React hook)

**Measurement Protocol:**
- Run each query 100 times
- Record p50, p95, p99 latencies
- Measure on warm database (after 10 warm-up queries)
- Use consistent hardware/environment

**Baseline Format (CSV):**
```csv
query,latency_p50_ms,latency_p95_ms,top_3_kinds,implementation_rank
authenticate,45,78,"func,heading_1,func",1
HTTP handler,52,89,"func,heading_2,func",1
database connection,48,82,"func,module,func",1
```

**Benchmark Script Requirements:**
- Automated execution (no manual steps)
- Reproducible results (±5ms variance acceptable)
- Export to CSV for comparison
- Log database query plans (EXPLAIN ANALYZE) for slow queries

**Benchmark Scenarios:**

1. **Small Corpus (1K chunks):**
   ```typescript
   benchmark('search latency - small corpus', async (b) => {
     const corpus = await loadCorpus('small'); // 1K chunks

     await b.start();
     for (let i = 0; i < 100; i++) {
       await search({ query: queries[i % queries.length], repo: 'small' });
     }
     await b.end();

     expect(b.avgLatency).toBeLessThan(50); // 50ms p50
   });
   ```

2. **Medium Corpus (100K chunks):**
   ```typescript
   benchmark('search latency - medium corpus', async (b) => {
     const corpus = await loadCorpus('medium'); // 100K chunks

     await b.start();
     for (let i = 0; i < 100; i++) {
       await search({ query: queries[i % queries.length], repo: 'medium' });
     }
     await b.end();

     expect(b.p95Latency).toBeLessThan(150); // 150ms p95
   });
   ```

3. **Concurrent Load:**
   ```typescript
   benchmark('concurrent search load', async (b) => {
     await b.start();

     const promises = [];
     for (let i = 0; i < 100; i++) {
       promises.push(search({ query: queries[i % queries.length] }));
     }
     await Promise.all(promises);

     await b.end();

     expect(b.p95Latency).toBeLessThan(500); // 500ms p95 under load
   });
   ```

**Performance Targets:**
- p50: <50ms (small corpus), <100ms (medium corpus)
- p95: <100ms (small corpus), <200ms (medium corpus)
- p99: <200ms (small corpus), <500ms (medium corpus)

### Level 3: Regression Tests

**Purpose:** Ensure known failures stay fixed.

**Approach:**
- Document current failure cases
- Create regression tests before fixing
- Tests should fail on main branch, pass after fix

**Known Failures to Test:**

1. **"validate_provider" returns tests first** (from failure analysis)
2. **"authentication" returns docs before AuthService**
3. **Case variations don't match** ("Authenticate" vs "authenticate")

**Example:**
```typescript
test('REGRESSION: validate_provider returns implementation first', async () => {
  // This test fails on main branch (before semantic ranking)
  // Should pass after implementation

  const results = await search({ query: 'validate_provider', repo: 'maproom' });

  expect(results[0].kind).toBe('function');
  expect(results[0].relpath).toMatch(/src.*config\.rs$/);
  expect(results[0].relpath).not.toMatch(/test/);
});
```

### Level 4: Unit Tests (Minimal)

**Scope:** Only test complex logic that can't be validated via integration.

**What to Unit Test:**
- Query normalization function (if complex)
- Edge case handling for multiplier calculation

**What NOT to Unit Test:**
- SQL CASE statements (validate via integration tests)
- Database queries (too tightly coupled)
- Simple arithmetic (waste of time)

**Example (Query Normalization with Acronym Handling):**
```typescript
describe('normalizeForExactMatch', () => {
  test('handles snake_case', () => {
    expect(normalizeForExactMatch('validate_provider')).toBe('validate_provider');
  });

  test('handles camelCase', () => {
    expect(normalizeForExactMatch('validateProvider')).toBe('validate_provider');
  });

  // NEW: Acronym handling tests (from review feedback)
  test('handles acronyms at start', () => {
    expect(normalizeForExactMatch('XMLParser')).toBe('xml_parser');
  });

  test('handles acronyms in middle', () => {
    expect(normalizeForExactMatch('parseXMLData')).toBe('parse_xml_data');
  });

  test('handles consecutive capitals', () => {
    expect(normalizeForExactMatch('HTTPSHandler')).toBe('https_handler');
  });

  test('handles mixed acronyms and camelCase', () => {
    expect(normalizeForExactMatch('validateHTTPRequest')).toBe('validate_http_request');
  });

  test('handles numbers with capitals', () => {
    expect(normalizeForExactMatch('Base64Encoder')).toBe('base64_encoder');
  });

  test('handles kebab-case', () => {
    expect(normalizeForExactMatch('validate-provider')).toBe('validate_provider');
  });

  test('handles spaces', () => {
    expect(normalizeForExactMatch('validate provider')).toBe('validate_provider');
  });

  test('handles case insensitive', () => {
    expect(normalizeForExactMatch('ValidateProvider')).toBe('validate_provider');
  });
});
```

## Test Environment Setup

### Database Seeding

**Approach:** Use fixtures for consistent test data.

```typescript
// test/fixtures/test-corpus.sql
-- Known chunks with controlled metadata for predictable testing

INSERT INTO maproom.chunks (symbol_name, kind, relpath, ts_doc, ...) VALUES
  ('authenticate', 'function', 'src/auth/authenticate.rs', ...),
  ('test_authenticate', 'test', 'tests/auth_test.rs', ...),
  ('validate_token', 'function', 'src/auth/validate_token.rs', ...),
  (NULL, 'documentation', 'docs/auth_guide.md', ...);
```

**Setup/Teardown:**
```typescript
beforeAll(async () => {
  await db.query('TRUNCATE maproom.chunks CASCADE');
  await db.query(await fs.readFile('test/fixtures/test-corpus.sql'));
});

afterAll(async () => {
  await db.close();
});
```

### Continuous Integration

**GitHub Actions Workflow:**
```yaml
- name: Run Integration Tests
  run: pnpm test:integration

- name: Run Performance Benchmarks
  run: pnpm test:benchmark

- name: Check Benchmark Regression
  run: |
    if [ $BENCHMARK_P95 -gt 200 ]; then
      echo "Performance regression detected"
      exit 1
    fi
```

## Acceptance Criteria

### Must-Have (Blocking)

- [ ] Search for exact function name returns implementation as #1 result (not test)
- [ ] Implementation ranks higher than test with same symbol name
- [ ] Implementation ranks higher than documentation mentioning the symbol
- [ ] Case-insensitive exact match works (ValidateProvider == validate_provider)
- [ ] Null symbol_name chunks don't crash (docs, comments)
- [ ] Unknown kind chunks don't crash (fallback to 1.0 multiplier)
- [ ] Debug mode returns score breakdown
- [ ] p95 latency increase <10% vs baseline
- [ ] All existing search integration tests still pass

### Should-Have (Important)

- [ ] Multi-word queries normalized correctly ("validate provider" → "validate_provider")
- [ ] Score breakdown shows reasonable values (base: 0-1, kind: 0.3-2.5, exact: 1.0-3.0)
- [ ] Concurrent load test passes (100 queries in parallel)
- [ ] Performance benchmarks documented and tracked

### Nice-to-Have (Future)

- [ ] Configurable multipliers via environment variables
- [ ] Metrics logging for search quality monitoring
- [ ] A/B testing framework for multiplier tuning

## Risk Mitigation

### Risk: Breaking Existing Searches

**Impact:** High - users rely on current behavior.

**Mitigation:**
- Run full regression suite before merge
- Compare top-10 results before/after on sample queries
- Gradual rollout: feature flag to toggle new ranking

**Contingency:**
- Keep old SQL query as fallback
- Easy rollback via feature flag

### Risk: Performance Degradation

**Impact:** Medium - slow search frustrates users.

**Mitigation:**
- Benchmark on realistic dataset sizes
- Monitor query plans (EXPLAIN ANALYZE)
- Set hard latency SLOs in CI

**Contingency:**
- Profile slow queries
- Add database indices if needed (unlikely)
- Simplify multiplier logic if necessary

### Risk: Multipliers Poorly Tuned

**Impact:** Low - can adjust post-launch.

**Mitigation:**
- Start with research-based values
- Monitor metrics after launch
- Easy to adjust (just SQL CASE values)

**Contingency:**
- Expose configuration for quick tuning
- Collect user feedback on ranking quality

## Testing Timeline

**Phase 1: Test Corpus Creation (2 days)**
- Create sample repository with known structure
- Index with maproom
- Validate chunk metadata extraction

**Phase 2: Integration Test Suite (3 days)**
- Implement ranking correctness tests
- Implement edge case tests
- Implement regression tests

**Phase 3: Performance Benchmarks (2 days)**
- Set up benchmark harness
- Run baseline measurements
- Establish performance targets

**Phase 4: CI Integration (1 day)**
- Add tests to GitHub Actions
- Configure failure thresholds
- Document test results

**Total: 8 days** (1.5 weeks)

## Success Metrics

### Quantitative

- **Top-1 Accuracy:** >90% of exact function searches return implementation as #1
- **Implementation Rank:** Average rank of implementation <3 (top 3 results)
- **Latency:** p95 <200ms (no significant regression)
- **Test Coverage:** >80% of ranking logic covered by integration tests

### Qualitative

- Search results "feel correct" to developers
- Reduced need to scroll past tests/docs
- Effective starting points for context() traversal

## Maintenance Plan

### Ongoing Monitoring

**Metrics to Track:**
- Search latency (p50, p95, p99)
- Top-1 result kind distribution (function vs test vs doc)
- Debug mode usage (score breakdown analysis)

**Alerts:**
- p95 latency >500ms (page ops)
- >20% of searches return tests as #1 (ranking degradation)

### Future Iterations

1. **A/B Test Multiplier Values** (after 1 month of usage)
2. **Add Configuration System** (if tuning needed)
3. **Incorporate User Feedback** (implicit via click data)
4. **Extend to Graph Signal** (use relationships table)

## Conclusion

**Quality Strategy Summary:**
- Focus on integration tests with real codebase samples
- Performance benchmarks to prevent regressions
- Regression tests for known failure cases
- Minimal unit tests (only where necessary)

**Philosophy:** Ship with confidence that ranking correctness improves, not with 100% line coverage of trivial code.

**MVP Mindset:** Tests should prevent rework and catch real bugs, not satisfy coverage metrics.
