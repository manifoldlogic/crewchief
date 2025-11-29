# CI/CD Integration for Maproom MCP

**Version**: 1.0
**Last Updated**: 2025-11-19
**Status**: Documentation for CI/CD integration

## Overview

This document describes how to integrate Maproom MCP semantic ranking tests into a CI/CD pipeline to ensure search quality and performance are maintained across all code changes.

## Test Suite Overview

The Maproom MCP package includes comprehensive tests for semantic entry point ranking:

### Test Categories

| Category | Script | Files | Purpose |
|----------|--------|-------|---------|
| **Unit Tests** | `pnpm test:unit` | `tests/unit/*.test.ts` | Query normalization logic |
| **Integration Tests** | `pnpm test:integration` | `tests/integration/*.test.ts` | End-to-end search quality |
| **SEMRANK Tests** | `pnpm test:semrank` | `tests/integration/regression.test.ts`<br>`tests/integration/search-quality.test.ts`<br>`tests/integration/semrank-edge-cases.test.ts` | Semantic ranking validation |
| **Legacy Tests** | `pnpm test:connection`<br>`pnpm test:blob-sha` | `tests/connection-fallback.test.cjs`<br>`tests/run-blob-sha-tests.cjs` | Connection and migration tests |
| **All Tests** | `pnpm test:all` | All test files | Complete test suite |

### Test Scripts

Available npm scripts in `package.json`:

```bash
# Run specific test suites
pnpm test:unit              # Unit tests only (normalization, utilities)
pnpm test:integration       # All integration tests
pnpm test:semrank           # SEMRANK-specific tests (regression, quality, edge cases)

# Run all tests
pnpm test:all               # connection + blob-sha + vitest (full suite)
pnpm test:vitest            # All vitest tests

# Legacy compatibility tests
pnpm test:connection        # Database connection fallback tests
pnpm test:blob-sha          # Blob SHA migration tests
```

## Running Tests Locally

### Prerequisites

1. **PostgreSQL Database**: Running instance with pgvector extension
2. **Test Corpus**: Indexed codebase (test-corpus or crewchief)
3. **Environment Variables**:
   ```bash
   export MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
   ```

### Quick Start

```bash
# Navigate to package
cd /workspace/packages/maproom-mcp

# Install dependencies (if not already installed)
pnpm install

# Build TypeScript
pnpm build

# Run all tests
pnpm test:all
```

### Running Specific Test Suites

```bash
# Unit tests (fast, no database required for some)
pnpm test:unit

# Integration tests (requires database with indexed data)
pnpm test:integration

# SEMRANK tests only
pnpm test:semrank

# Run with watch mode for development
pnpm exec vitest --watch tests/integration/regression.test.ts
```

## GitHub Actions Integration

### Recommended Workflow Structure

Add a test job to your GitHub Actions workflow (e.g., `.github/workflows/test.yml`):

```yaml
name: Test

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  test-maproom-mcp:
    name: Test Maproom MCP (Semantic Ranking)
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
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'

      - name: Install dependencies
        run: pnpm install

      - name: Build packages
        run: pnpm build

      - name: Setup database schema
        working-directory: packages/maproom-mcp
        run: |
          psql $DATABASE_URL -f config/init.sql
        env:
          DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom

      - name: Index test corpus
        working-directory: packages/maproom-mcp
        run: |
          # Index a small test corpus for integration tests
          # This step depends on your test setup
          # Example: node bin/cli.cjs scan /path/to/test-corpus
        env:
          MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom

      - name: Run all tests
        working-directory: packages/maproom-mcp
        run: pnpm test:all
        env:
          MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: packages/maproom-mcp/test-results/
          retention-days: 7
```

### Performance Regression Detection

To fail CI on performance regressions, add a performance check step:

```yaml
      - name: Check for performance regression
        working-directory: packages/maproom-mcp
        run: |
          # Extract p95 latency from test results
          # Compare to baseline (200ms target, 220ms max with 10% tolerance)
          # Exit 1 if regression detected
          node scripts/check-performance.js
```

Example `scripts/check-performance.js`:

```javascript
import fs from 'fs';
import path from 'path';

// Baseline performance targets (from architecture.md)
const BASELINE_P95 = 200; // ms
const MAX_REGRESSION = 0.10; // 10% increase allowed
const MAX_P95 = BASELINE_P95 * (1 + MAX_REGRESSION); // 220ms

// Parse test results (adapt to your test output format)
function extractP95FromTestResults() {
  // Read vitest JSON output or parse stdout logs
  // Return p95 latency in milliseconds
  // This is a placeholder - implement based on your test output format
  return null; // or actual p95 value
}

const p95 = extractP95FromTestResults();

if (p95 === null) {
  console.log('⚠️  Could not extract p95 latency from test results');
  console.log('   Skipping performance regression check');
  process.exit(0); // Don't fail if we can't extract metrics
}

console.log(`📊 Performance Check:`);
console.log(`   Baseline p95: ${BASELINE_P95}ms`);
console.log(`   Current p95: ${p95}ms`);
console.log(`   Max allowed: ${MAX_P95}ms (10% tolerance)`);

if (p95 > MAX_P95) {
  console.error(`❌ PERFORMANCE REGRESSION DETECTED!`);
  console.error(`   p95 latency (${p95}ms) exceeds maximum (${MAX_P95}ms)`);
  console.error(`   This is a ${((p95 / BASELINE_P95 - 1) * 100).toFixed(1)}% increase from baseline`);
  process.exit(1);
} else {
  const improvement = ((1 - p95 / BASELINE_P95) * 100).toFixed(1);
  if (p95 < BASELINE_P95) {
    console.log(`✅ Performance improved by ${improvement}%`);
  } else {
    const regression = ((p95 / BASELINE_P95 - 1) * 100).toFixed(1);
    console.log(`✅ Performance within acceptable range (+${regression}%)`);
  }
  process.exit(0);
}
```

## Performance Baselines

### Current Targets (from SEMRANK benchmarks)

| Metric | Baseline | Max Allowed (10% tolerance) | Target |
|--------|----------|----------------------------|--------|
| p50 latency | 50ms | 55ms | <100ms |
| p95 latency | 200ms | 220ms | <200ms |
| p99 latency | 300ms | 330ms | <500ms |
| Top-1 implementation rate | 70% | 63% (90% of target) | >70% |
| Average implementation rank | 3 | 3.3 | <3 |

### Updating Baselines

After intentional performance optimizations or multiplier tuning:

1. **Run benchmarks locally**:
   ```bash
   cd /workspace/packages/maproom-mcp
   pnpm test:semrank --reporter=verbose
   ```

2. **Record new metrics**:
   - Document in `docs/search-ranking.md`
   - Update `scripts/check-performance.js` with new baselines
   - Commit with rationale in commit message

3. **Review quarterly**:
   - Check if baselines are still relevant
   - Adjust for infrastructure changes (faster hardware, new PostgreSQL version)
   - Document any baseline updates in architecture.md

## Test Failure Scenarios

### When CI Should Fail

| Failure Type | Description | Action |
|-------------|-------------|---------|
| **Unit Test Failure** | Normalization logic broken | Fix normalization function, verify edge cases |
| **Integration Test Failure** | Ranking incorrect or search quality degraded | Check multiplier values, verify SQL logic |
| **Edge Case Failure** | Null handling, special characters, multi-word queries | Add test case, fix edge case handling |
| **Performance Regression** | p95 latency >10% increase | Profile queries, optimize SQL, check database indexes |
| **Regression Test Failure** | Known failure cases reappear | Verify multipliers, check for code reverts |

### Debugging CI Failures

1. **Review CI logs**:
   - Check which specific test failed
   - Look for error messages and stack traces
   - Note any database connection issues

2. **Reproduce locally**:
   ```bash
   cd /workspace/packages/maproom-mcp
   export MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom

   # Run specific failing test
   pnpm exec vitest run tests/integration/regression.test.ts
   ```

3. **Common issues**:
   - **Database not seeded**: Ensure test corpus is indexed
   - **Wrong environment variables**: Check DATABASE_URL
   - **Timing issues**: Tests may be flaky if database is slow
   - **Schema mismatch**: Ensure init.sql matches expectations

4. **Get help**:
   - Check `docs/search-ranking.md` for semantic ranking details
   - Review `docs/deployment/semantic-ranking-rollout.md` for troubleshooting
   - Examine test file directly for expected behavior

## Adding New Tests to CI

### Process

1. **Write test**:
   ```bash
   # Create test file
   touch packages/maproom-mcp/tests/integration/my-new-test.test.ts

   # Write test using vitest
   # Follow patterns from existing tests
   ```

2. **Test locally**:
   ```bash
   pnpm exec vitest run tests/integration/my-new-test.test.ts
   ```

3. **Add to appropriate script** (if needed):
   ```json
   "test:custom": "vitest run tests/integration/my-new-test.test.ts"
   ```

4. **Verify in CI**:
   - Open PR
   - Check that test runs in CI
   - Ensure test passes

### Test Organization

```
packages/maproom-mcp/tests/
├── unit/                           # Unit tests (no database)
│   ├── normalize.test.ts           # Query normalization
│   └── worktree-resolution.test.ts # Worktree logic
├── integration/                    # Integration tests (database required)
│   ├── regression.test.ts          # SEMRANK regression tests
│   ├── search-quality.test.ts      # SEMRANK search quality
│   ├── semrank-edge-cases.test.ts  # SEMRANK edge cases
│   ├── worktree-scoping.test.ts    # Worktree filtering
│   └── claude_desktop_test.ts      # Full MCP integration
├── tools/                          # MCP tool tests
│   ├── open.test.ts
│   ├── context.int.test.ts
│   └── ...
├── filters/                        # Filter tests
│   └── file-type.e2e.test.ts
└── migrations/                     # Schema migration tests
    └── schema-integration.test.ts
```

## Cache Strategy for Faster CI

### Options

1. **PostgreSQL Service** (current recommendation):
   - Fresh database each run
   - Simple, no caching complexity
   - ~2-3 minutes setup time

2. **Docker Layer Caching**:
   - Cache pgvector image
   - Moderate speedup (~30 seconds saved)
   - Add to workflow:
     ```yaml
     - name: Cache Docker layers
       uses: actions/cache@v3
       with:
         path: /tmp/.buildx-cache
         key: ${{ runner.os }}-buildx-${{ github.sha }}
         restore-keys: |
           ${{ runner.os }}-buildx-
     ```

3. **Pre-indexed Fixture** (advanced):
   - Cache PostgreSQL data directory with indexed test corpus
   - Fastest (~1 minute setup)
   - Complex to maintain, risk of stale data
   - **Not recommended initially**

### Current Approach

Start with option 1 (fresh PostgreSQL service). If CI is too slow (>10 minutes total), optimize incrementally.

## Maintenance

### Regular Tasks

- **Weekly**: Review CI test results for flakiness
- **Monthly**: Check performance baselines are reasonable
- **Quarterly**: Review and update baselines after infrastructure changes
- **After SEMRANK changes**: Verify all tests pass, update baselines if intentional

### Updating Documentation

When adding new tests or changing CI behavior:
1. Update this document (`docs/ci-cd.md`)
2. Update deployment runbook if deployment process changes
3. Document performance baseline changes in architecture.md

## Troubleshooting

### Tests Pass Locally But Fail in CI

**Possible causes:**
- Environment variable differences
- Different PostgreSQL version
- Timing issues (CI may be slower)
- Missing test corpus data

**Solutions:**
- Check CI logs for environment variable values
- Ensure PostgreSQL version matches (pg16 with pgvector)
- Add retry logic for timing-sensitive tests
- Verify test corpus is indexed in CI setup step

### CI Takes Too Long (>10 minutes)

**Optimization steps:**
1. Parallelize test suites
2. Cache dependencies (pnpm cache)
3. Cache Docker images
4. Reduce test corpus size
5. Skip non-critical tests in PR, run full suite on merge

### Flaky Tests

**Diagnosis:**
- Identify which test is flaky (fails intermittently)
- Check for timing dependencies (sleep/wait statements)
- Look for non-deterministic behavior (random data, timestamps)

**Solutions:**
- Use fixed seeds for random data
- Increase timeouts for slow operations
- Use deterministic test data
- Add retries for network-dependent tests

## References

- **SEMRANK Project**: `.crewchief/projects/SEMRANK_semantic-entry-point-ranking/`
- **Search Ranking Guide**: `packages/maproom-mcp/docs/search-ranking.md`
- **Deployment Runbook**: `packages/maproom-mcp/docs/deployment/semantic-ranking-rollout.md`
- **Architecture**: `docs/architecture/MAPROOM_ARCHITECTURE.md`

---

**Document Ownership**: Maproom MCP Team
**Review Schedule**: Quarterly or after major CI changes
**Last Review**: 2025-11-19
