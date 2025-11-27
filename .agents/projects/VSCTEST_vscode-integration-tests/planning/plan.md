# Plan: VS Code Integration Tests

## Overview

Add real VS Code integration tests using `@vscode/test-electron` to verify extension activation and setup works correctly, including in devcontainer environments.

## Phases

### Phase 1: Infrastructure Setup

**Objective**: Set up the test infrastructure and dependencies.

**Deliverables**:
1. Install @vscode/test-electron and related dependencies
2. Create separate tsconfig for E2E tests
3. Create test runner (runTests.ts)
4. Create test suite index (suite/index.ts)
5. Add package.json scripts for E2E tests
6. Create test workspace fixture

**Agent Assignment**: vscode-extension-specialist

**Acceptance Criteria**:
- [ ] Dependencies installed and building
- [ ] `pnpm test:e2e:compile` succeeds
- [ ] Empty test suite runs (even if no tests yet)

**Estimated Tickets**: 2

---

### Phase 2: Core Integration Tests

**Objective**: Implement the critical activation and command tests.

**Deliverables**:
1. Activation tests (extension present, activates, timing)
2. Command registration tests
3. Status bar tests
4. Configuration access tests

**Agent Assignment**: vscode-extension-specialist

**Acceptance Criteria**:
- [ ] 8+ integration tests pass
- [ ] Tests run in under 2 minutes
- [ ] Tests pass on clean VS Code instance

**Estimated Tickets**: 2-3

---

### Phase 3: Devcontainer Support

**Objective**: Ensure tests run in devcontainer with xvfb.

**Deliverables**:
1. Add xvfb to devcontainer (Dockerfile or feature)
2. Create helper script for running tests with xvfb
3. Document devcontainer test requirements
4. Verify tests pass in devcontainer

**Agent Assignment**: docker-engineer (for devcontainer changes)

**Acceptance Criteria**:
- [ ] `xvfb-run -a pnpm test:e2e` works in devcontainer
- [ ] Documentation updated with devcontainer instructions

**Estimated Tickets**: 1-2

---

### Phase 4: CI Integration

**Objective**: Add integration tests to CI pipeline.

**Deliverables**:
1. Add E2E test job to GitHub Actions workflow
2. Cache VS Code download for faster CI
3. Configure test as informational (allowed to fail initially)
4. Add VS Code launch configuration for debugging

**Agent Assignment**: github-actions-specialist

**Acceptance Criteria**:
- [ ] E2E tests run in CI
- [ ] Test results visible in CI output
- [ ] Debug configuration works locally

**Estimated Tickets**: 1-2

---

## Ticket Summary

| Phase | Ticket ID | Title | Agent |
|-------|-----------|-------|-------|
| 1 | VSCTEST-1001 | Install dependencies and create test infrastructure | vscode-extension-specialist |
| 1 | VSCTEST-1002 | Create test runner and suite index | vscode-extension-specialist |
| 2 | VSCTEST-2001 | Implement activation and command tests | vscode-extension-specialist |
| 2 | VSCTEST-2002 | Implement status bar and configuration tests | vscode-extension-specialist |
| 3 | VSCTEST-3001 | Add xvfb support to devcontainer | docker-engineer |
| 4 | VSCTEST-4001 | Add E2E tests to CI workflow | github-actions-specialist |

**Total: 6 tickets**

## Dependencies

### External Dependencies
- @vscode/test-electron (npm package)
- Mocha (npm package)
- xvfb (system package for Linux)

### Internal Dependencies
- Extension must build successfully (`pnpm build`)
- Existing unit tests should pass (`pnpm test`)

## Risk Management

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| xvfb issues in devcontainer | Medium | High | Test early, fallback to skip |
| VS Code download failures | Low | Medium | Cache in CI, retry logic |
| Test flakiness | Medium | Medium | Long timeouts, sequential tests |
| Blocking release | Low | High | Make tests informational initially |

## Success Criteria

### MVP Success (Must Have)
- [ ] Infrastructure set up and working
- [ ] 8+ integration tests passing locally
- [ ] Tests run in devcontainer with xvfb
- [ ] Basic CI integration (can be informational)

### Full Success (Nice to Have)
- [ ] Tests consistently pass in CI
- [ ] Tests integrated into release checklist
- [ ] Debug configuration documented
- [ ] All documented sticking points covered

## Timeline Notes

This is a small, focused project. Phases can be executed sequentially by the assigned agents. No complex dependencies between phases except that Phase 1 must complete before Phase 2.

## Out of Scope

- UI automation testing (vscode-extension-tester)
- Performance benchmarking
- Multiple VS Code version testing
- Web extension testing
- Visual regression testing
