# TESTFIX: Fix All Tests

## Project Summary

Fix all test failures across the CrewChief monorepo to restore CI health. Tests have fallen out of sync with implementation due to API changes in the Rust `crewchief-maproom` crate and TypeScript packages.

## Problem Statement

- **190 Rust test compilation errors** due to API changes (EmbeddingService, ChangeType, SearchOptions)
- **53 CLI test failures** due to assertion mismatches and environment pollution
- **16 VSCode extension test failures** due to process spawn timeouts
- **5 Daemon-client test failures** (performance tests require daemon binary)
- **MCP tests require PostgreSQL** - use `pnpm test:connection` locally
- **CI pipeline failing** - all test jobs blocked
- **Stale worktree** at `packages/cli/.crewchief/worktrees/variant-test-*` causing duplicate test discovery

## Proposed Solution

Systematic repair of test files to align with current APIs:
1. Clean up environment pollution (stale worktrees) + add vitest config
2. Fix Rust test compilation errors by API pattern
3. Fix TypeScript test assertion mismatches
4. Verify CI pipeline passes

## Key Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Rust compilation errors | 190 | 0 |
| CLI test failures | 53 | 0 |
| VSCode test failures | 16 | 0 |
| CI jobs passing | 0/5 | 5/5 |

## Phases

1. **Environment Cleanup** (2 tickets) - Remove test pollution, add vitest config
2. **Rust Test Compilation** (1 ticket) - Fix all 190 compilation errors
3. **Rust Test Execution** (1 ticket) - Verify tests pass
4. **TypeScript Test Fixes** (3 tickets) - Fix CLI, VSCode, configure MCP/daemon-client
5. **CI Verification** (2 tickets) - Confirm pipeline passes

**Total: 9 tickets** (consolidated from 17 after review)

## Relevant Agents

- **rust-indexer-engineer** - Rust API expertise for test fixes
- **vscode-extension-specialist** - VSCode test fixes
- **unit-test-runner** - Test execution and verification
- **github-actions-specialist** - CI configuration
- **general-purpose** - TypeScript tests and cleanup

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space and root cause analysis
- [Architecture](planning/architecture.md) - Technical approach and patterns
- [Quality Strategy](planning/quality-strategy.md) - Verification approach
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Phased execution plan
- [Project Review](planning/project-review.md) - Critical review findings
- [Review Updates](planning/review-updates.md) - Changes made to address review

## Status

**Phase**: Tickets Created - Ready for Execution
**Review Status**: PASSED
**Tickets**: 9 tickets across 5 phases

## Next Steps

Run `/work-on-project TESTFIX` to execute all tickets, or `/single-ticket TESTFIX-1001` to start with the first ticket.
