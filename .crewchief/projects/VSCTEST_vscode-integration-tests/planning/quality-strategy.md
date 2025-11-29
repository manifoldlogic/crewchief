# Quality Strategy: VS Code Integration Tests

## Testing Philosophy

This project adds a thin layer of real VS Code integration tests focused on **sticking points** - the areas most likely to fail in production but impossible to catch with mocked tests. The goal is confidence, not coverage.

## Test Categories

### Category 1: Activation Tests (Critical)

These tests verify the extension can activate in a real VS Code environment.

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Extension is present | Package.json correct | Extension not installable |
| Extension activates | No activation errors | Users see error on startup |
| Activation under 500ms | Performance requirement | Blocks VS Code startup |
| Output channel created | Logging works | Can't debug issues |

### Category 2: Command Tests (High)

Verify commands are registered and callable.

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| showOutput registered | Command exists | Feature not accessible |
| setup registered | Command exists | Can't configure extension |
| restartWatchers registered | Command exists | Can't recover from issues |
| showStatus registered | Command exists | Can't check status |

### Category 3: Status Bar Tests (Medium)

Verify status bar integration works.

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Status bar item created | UI component exists | No visual feedback |
| Status bar visible | Actually shows | Users can't see status |
| Status bar clickable | Interaction works | Dead UI element |

### Category 4: Configuration Tests (Medium)

Verify settings system works.

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Read configuration | Settings accessible | Can't customize behavior |
| Workspace state | State persistence | Setup wizard repeats |

### Category 5: Error Handling Tests (Low - Nice to Have)

Verify graceful degradation.

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| No workspace graceful | Error message shown | Confusing failure |
| Missing database graceful | Guidance shown | Users don't know what to do |

## Test Count Target

**Target: 8-12 focused tests**

- Activation: 3-4 tests
- Commands: 1-2 tests (batch command checks)
- Status Bar: 2-3 tests
- Configuration: 2 tests
- Error Handling: 1-2 tests (optional)

## Pass/Fail Criteria

### For Individual Tests
- Tests must complete within timeout (30s per test, 60s for slow operations)
- Assertions must pass without flakiness
- No error output to console

### For Test Suite
- All tests pass on clean VS Code instance
- Tests pass in devcontainer with xvfb
- Tests pass in CI environment

### For Release
- Integration tests are informational, not blocking for MVP
- Documented known issues if tests skip in certain environments

## Test Execution Strategy

### Local Development
```bash
# Quick: Run unit tests only (mocked)
pnpm test

# Full: Run both unit and integration tests
pnpm test && pnpm test:e2e

# Debug: Run integration tests with VS Code debugger
# Use "Extension Tests" launch configuration
```

### In Devcontainer
```bash
# Requires xvfb
xvfb-run -a pnpm test:e2e
```

### In CI
```yaml
# Run after unit tests
- run: pnpm test
- run: xvfb-run -a pnpm test:e2e
  continue-on-error: true  # Don't block releases initially
```

## Flakiness Prevention

### Timeout Strategy
- Suite setup: 30 seconds (VS Code startup)
- Individual test: 10 seconds default
- Slow tests: Explicit 30 second timeout

### Retry Strategy
- No automatic retries (indicates real problems)
- Manual retry with increased timeout for CI

### Isolation Strategy
- Each test starts with fresh extension state
- Tests don't share mutable state
- Test workspace is read-only fixture

## Environment Requirements

### Local Machine
- Node.js 20+
- VS Code installed (for development)
- Disk space for VS Code download (~500MB)

### Devcontainer
- xvfb package installed
- Network access for VS Code download
- Sufficient memory (2GB+)

### CI
- Ubuntu runner
- xvfb-run available
- GitHub Actions cache for VS Code

## Test Fixtures

### Workspace Fixture
```
fixtures/workspace/
├── .vscode/
│   └── settings.json    # Test-specific settings
├── sample.ts            # Sample TypeScript file
└── .maproom/            # Optional: Pre-created database
    └── maproom.db       # Empty or minimal test database
```

### Settings Fixture
```json
{
  "maproom.database.sqlitePath": "",
  "maproom.ollama.endpoint": "http://127.0.0.1:11434"
}
```

## Coverage Considerations

### Not Measured
Integration tests don't contribute to code coverage metrics. They test integration, not code paths.

### What We Gain
- Confidence that activation works
- Verification that VS Code APIs behave as expected
- Catch issues that mocks can't reveal

### What Remains Uncovered
- Full end-to-end workflows (process spawning, indexing)
- Performance under load
- All error scenarios
- UI rendering correctness

## Known Limitations

### Cannot Test
1. Ollama integration (requires running service)
2. Process spawning (requires binary)
3. Database operations (requires SQLite file)
4. File watching (requires real file changes)

### Workarounds
- Mock binary path to non-existent file (tests error handling)
- Create minimal SQLite fixture for database tests
- Skip Ollama tests if service not running

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Tests too slow | Focus on critical paths only |
| Flaky tests | Long timeouts, no shared state |
| Different VS Code versions | Test stable version only |
| Devcontainer differences | Document requirements clearly |
| CI failures | Make E2E tests informational initially |

## Success Metrics

### MVP Success
- [ ] 8+ integration tests pass locally
- [ ] Tests pass in devcontainer with xvfb
- [ ] Tests run in CI (can be allowed to fail)
- [ ] Documentation for running tests exists

### Future Success
- [ ] Tests pass in CI consistently
- [ ] Tests catch real activation bugs
- [ ] Test suite runs in under 2 minutes
- [ ] Tests integrated into release checklist
