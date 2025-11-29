# Ticket: CFGVER-5004: CI/CD Pipeline Updates

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Agents
- database-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Update GitHub Actions workflow to run all new tests (unit + integration) and generate coverage reports. The CI pipeline must validate config-manager changes on every PR, check coverage thresholds, and provide clear feedback on test failures.

## Background
CI pipeline needs to run all new tests and enforce quality gates before merging config-manager changes. The quality strategy requires 80%+ code coverage and all tests passing. Integration tests should run but gracefully skip Docker tests if Docker is unavailable in CI.

Reference: `quality-strategy.md` lines 251-284 for CI workflow structure.

## Acceptance Criteria
- [ ] Unit tests run in CI on every PR touching config-manager code
- [ ] Integration tests run in CI (skip Docker tests if unavailable)
- [ ] Coverage report generated and checked (>= 80% threshold)
- [ ] Test results visible in PR checks (pass/fail status)
- [ ] Failing tests block merge
- [ ] Coverage uploaded to Codecov (optional but recommended)
- [ ] Workflow only triggers on relevant file changes (performance)

## Technical Requirements

**GitHub Actions Workflow:**

Create or modify: `.github/workflows/test-config-manager.yml`

```yaml
name: Config Manager Tests

on:
  pull_request:
    paths:
      - 'packages/maproom-mcp/src/config-manager.ts'
      - 'packages/maproom-mcp/tests/**'
      - 'packages/maproom-mcp/bin/cli.cjs'
      - 'packages/maproom-mcp/package.json'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: cd packages/maproom-mcp && npm install

      - name: Run unit tests
        run: cd packages/maproom-mcp && npm test -- --coverage

      - name: Check coverage threshold
        run: cd packages/maproom-mcp && npm run test:coverage-check

      - name: Run integration tests
        run: cd packages/maproom-mcp && npm run test:integration
        continue-on-error: true  # Docker may not be available

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./packages/maproom-mcp/coverage/lcov.info
        if: always()
```

**Package.json Scripts:**

Add to `packages/maproom-mcp/package.json`:
```json
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage",
    "test:coverage-check": "vitest run --coverage --coverage.thresholds.lines=80 --coverage.thresholds.functions=80 --coverage.thresholds.branches=75",
    "test:integration": "vitest run --config vitest.integration.config.js"
  }
}
```

**Vitest Configuration:**

Ensure `packages/maproom-mcp/vitest.config.js` includes:
```javascript
export default {
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov', 'html'],
      include: ['src/**/*.js'],
      exclude: ['tests/**', 'node_modules/**'],
    },
  },
};
```

**Path Filters:**
Workflow only triggers on changes to:
- `packages/maproom-mcp/src/config-manager.ts`
- `packages/maproom-mcp/tests/**`
- `packages/maproom-mcp/bin/cli.cjs`
- `packages/maproom-mcp/package.json`

This prevents unnecessary CI runs on unrelated changes.

## Implementation Notes

**Workflow Testing Strategy:**

1. **Create Workflow File**
   - Place in `.github/workflows/test-config-manager.yml`
   - Test locally first with act (optional): `act pull_request`

2. **Add Package Scripts**
   - Update `packages/maproom-mcp/package.json`
   - Test scripts locally before committing

3. **Configure Vitest**
   - Ensure coverage thresholds in vitest.config.js
   - Verify coverage reports generate correctly

4. **Test Workflow**
   - Create test PR to verify workflow runs
   - Verify coverage report uploads
   - Verify failing tests block merge

**Coverage Threshold Strategy:**
- Lines: 80% (primary metric)
- Functions: 80% (ensure all functions tested)
- Branches: 75% (some branches hard to reach)

**Integration Test Handling:**
Use `continue-on-error: true` for integration tests because:
- Docker may not be available in CI
- Integration tests still valuable for local development
- Unit tests provide sufficient confidence for merge

**Codecov Integration (Optional):**
If using Codecov:
1. Add CODECOV_TOKEN secret to GitHub repo
2. Uncomment upload coverage step
3. View coverage trends in Codecov dashboard

**Reference CI Strategy:**
See `quality-strategy.md` lines 251-284 for complete CI workflow.

## Dependencies
- CFGVER-5001 (unit tests complete)
- CFGVER-5002 (integration tests complete)

## Risk Assessment
- **Risk**: CI environment limitations (no Docker)
  - **Mitigation**: Mark integration tests as `continue-on-error`, focus on unit tests

- **Risk**: Flaky tests blocking PRs
  - **Mitigation**: Ensure test isolation, use memfs for unit tests, mark known flaky tests

- **Risk**: Coverage threshold too strict blocks legitimate changes
  - **Mitigation**: 80% is balanced, can adjust in vitest.config.js if needed

- **Risk**: Workflow triggers on every file change (performance)
  - **Mitigation**: Use path filters to only trigger on config-manager changes

## Files/Packages Affected
- **Create**: `.github/workflows/test-config-manager.yml` (GitHub Actions workflow)
- **Modify**: `packages/maproom-mcp/package.json` (add test scripts)
- **Modify**: `packages/maproom-mcp/vitest.config.js` (coverage thresholds)
- **Create**: `packages/maproom-mcp/vitest.integration.config.js` (if needed for integration tests)
