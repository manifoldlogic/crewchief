# Ticket: SEMRANK-4005: CI/CD Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (CI workflow itself)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate semantic ranking tests into CI/CD pipeline to ensure all future changes maintain search quality and performance. Tests will run on every PR to prevent regressions.

## Background
The SEMRANK project has created comprehensive tests (unit, integration, edge cases, performance benchmarks) in Phase 3. To prevent regressions and maintain search quality, these tests must be integrated into the CI/CD pipeline. Performance benchmarks should fail CI if latency increases by >10%.

This ticket implements the CI/CD integration requirements from Phase 4 (Documentation & Deployment) of the SEMRANK project plan.

## Acceptance Criteria
- [ ] Add test scripts to `/packages/maproom-mcp/package.json`:
  - [ ] `"test:unit"`: Run normalization unit tests
  - [ ] `"test:integration"`: Run all integration tests
  - [ ] `"test:benchmark"`: Run performance benchmarks
  - [ ] `"test:all"`: Run unit + integration + benchmarks
- [ ] Update `.github/workflows/test.yml`:
  - [ ] Add job: "test-maproom-mcp"
  - [ ] Run `pnpm test:all` in maproom-mcp package
  - [ ] Fail if any tests fail
  - [ ] Fail if p95 latency >10% vs baseline
  - [ ] Cache database for faster CI runs
- [ ] Add performance regression check:
  - [ ] Compare benchmark results to baseline
  - [ ] Fail if p95 > baseline × 1.1 (10% increase)
  - [ ] Log performance comparison in CI output
- [ ] Test CI workflow on PR:
  - [ ] Verify all tests run
  - [ ] Verify performance check works
  - [ ] Verify failures block merge
- [ ] Document CI integration in `/packages/maproom-mcp/docs/ci-cd.md`

## Technical Requirements
- Add test scripts to maproom-mcp package.json
- Integrate tests into existing GitHub Actions workflow
- Implement performance regression detection
- Cache PostgreSQL database to speed up CI
- Document CI setup and maintenance

## Implementation Notes

### Test Scripts (package.json)
Add to `/packages/maproom-mcp/package.json`:
```json
{
  "scripts": {
    "test:unit": "vitest run tests/normalization.test.ts",
    "test:integration": "vitest run tests/integration/",
    "test:benchmark": "vitest run tests/benchmarks/performance.test.ts",
    "test:all": "vitest run"
  }
}
```

### GitHub Actions Workflow Structure
Add to `.github/workflows/test.yml`:
```yaml
jobs:
  test-maproom-mcp:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
          POSTGRES_DB: maproom
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
      - name: Install dependencies
        run: pnpm install
      - name: Build packages
        run: pnpm build
      - name: Run tests
        working-directory: packages/maproom-mcp
        run: pnpm test:all
        env:
          DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom
      - name: Check performance regression
        run: node scripts/check-performance.js
```

### Performance Regression Check
Create `/scripts/check-performance.js`:
```javascript
// Parse benchmark results from test output
// Compare p95 latency to baseline (from architecture.md: p95 < 200ms)
// Exit 1 if p95 > baseline × 1.1 (10% increase)
```

Baseline values (from architecture.md):
- p95 latency: 200ms (target)
- Acceptable increase: 10% (220ms max)

### Database Caching Strategy
Options for faster CI:
1. **PostgreSQL service**: Spin up fresh DB each run (simpler, slower)
2. **Docker layer caching**: Cache pgvector image (moderate speedup)
3. **Pre-indexed fixture**: Cache indexed test corpus (faster, more complex)

Recommendation: Start with option 1 (fresh DB), optimize later if CI is slow.

### Test Organization
Ensure tests are organized for CI:
```
packages/maproom-mcp/tests/
├── normalization.test.ts          # Unit tests
├── integration/
│   ├── ranking-correctness.test.ts
│   ├── edge-cases.test.ts
│   └── regression.test.ts
└── benchmarks/
    └── performance.test.ts
```

### CI Failure Scenarios
Tests should fail if:
1. Any unit test fails (normalization logic broken)
2. Any integration test fails (ranking incorrect)
3. Edge case handling broken
4. Performance regression detected (p95 >10% increase)
5. Regression tests fail (old queries degraded)

### Documentation (ci-cd.md)
Document:
- How to run tests locally
- How to update performance baselines
- How to debug CI failures
- How to add new tests to CI
- Performance regression thresholds

## Dependencies
- SEMRANK-3003 (integration tests)
- SEMRANK-3004 (edge case tests)
- SEMRANK-3005 (performance benchmarks)
- SEMRANK-3006 (regression tests)

## Risk Assessment
- **Risk**: CI runs too slowly (>10 minutes)
  - **Mitigation**: Cache database/dependencies; parallelize tests; optimize test corpus size
- **Risk**: Flaky tests cause false failures
  - **Mitigation**: Use deterministic test data; add retries for network-dependent tests; seed RNG
- **Risk**: Performance baselines become outdated
  - **Mitigation**: Document how to update baselines; review baselines quarterly; log actual vs baseline in CI
- **Risk**: CI failures block legitimate PRs
  - **Mitigation**: Make performance threshold reasonable (10%); allow manual override with explanation; document debugging steps

## Files/Packages Affected
- `/packages/maproom-mcp/package.json` (add test scripts)
- `/.github/workflows/test.yml` (add maproom-mcp tests)
- `/scripts/check-performance.js` (new file for regression check)
- `/packages/maproom-mcp/docs/ci-cd.md` (new documentation)
