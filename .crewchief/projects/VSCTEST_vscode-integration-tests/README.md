# VSCTEST: VS Code Integration Tests

## Status: Planning

## Summary

Add real VS Code integration tests using `@vscode/test-electron` to verify extension activation and setup works correctly in actual VS Code instances, including devcontainer environments.

## Problem Statement

The vscode-maproom extension has comprehensive unit tests (270+ tests, 71% coverage) but all use mocked VS Code APIs. Real VS Code environment issues - activation timing, command registration, status bar rendering, devcontainer compatibility - cannot be caught by mocked tests. This project adds targeted integration tests for the most likely sticking points.

## Proposed Solution

Use `@vscode/test-electron` to run 8-12 focused tests that verify:
- Extension activates without errors
- Commands are registered and accessible
- Status bar item appears and responds
- Configuration system works
- Graceful handling when workspace missing

Tests will run in a real (but headless) VS Code instance using xvfb in Linux/devcontainer environments.

## Scope

### In Scope
- @vscode/test-electron infrastructure setup
- Mocha-based test suite for VS Code tests
- 8-12 focused integration tests
- Devcontainer xvfb configuration
- CI workflow integration (informational initially)

### Out of Scope
- UI automation (vscode-extension-tester)
- Performance benchmarking
- Multiple VS Code version testing
- Full end-to-end workflow testing

## Agents

| Phase | Agent | Purpose |
|-------|-------|---------|
| 1-2 | vscode-extension-specialist | Test infrastructure and core tests |
| 3 | docker-engineer | Devcontainer xvfb setup |
| 4 | github-actions-specialist | CI workflow integration |

## Planning Documents

- [analysis.md](./planning/analysis.md) - Problem analysis and research
- [architecture.md](./planning/architecture.md) - Solution design and structure
- [quality-strategy.md](./planning/quality-strategy.md) - Testing approach
- [security-review.md](./planning/security-review.md) - Security assessment
- [plan.md](./planning/plan.md) - Execution phases and tickets

## Quick Reference

### Phase Overview
1. **Infrastructure Setup** - Dependencies, test runner, fixtures
2. **Core Integration Tests** - Activation, commands, status bar, config
3. **Devcontainer Support** - xvfb configuration
4. **CI Integration** - GitHub Actions workflow

### Ticket Count
6 tickets across 4 phases

### Key Technologies
- @vscode/test-electron
- Mocha (test framework)
- xvfb (headless display)

## Next Steps

Run `/review-project VSCTEST` before creating tickets.
