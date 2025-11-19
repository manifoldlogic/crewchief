# SEMRANK Ticket Index

**Project:** Semantic Entry Point Ranking
**Project Slug:** SEMRANK
**Total Tickets:** 21
**Estimated Timeline:** 3.5-4.5 weeks (18-24 days)

---

## Phase 0: MCP Tool Creation & Baseline (2-3 days)

Create the missing TypeScript MCP search tool and validate baseline FTS behavior before implementing semantic enhancements.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-0001](./SEMRANK-0001_create-search-mcp-tool.md) | Create TypeScript Search MCP Tool | general-purpose | 2 days | ✅ Complete | None |
| [SEMRANK-0002](./SEMRANK-0002_validate-baseline-fts.md) | Validate Baseline FTS Implementation | verify-ticket | 1 day | ✅ Complete | 0001 |

**Phase 0 Total:** 2 tickets, 3 days

---

## Phase 1: Foundation & Test Corpus (3-4 days)

Create test corpus, index it, measure baseline metrics, and set up integration test framework.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-1003](./SEMRANK-1003_create-test-corpus.md) | Create Test Corpus | general-purpose | 1 day | ⏳ Pending | 0002 |
| [SEMRANK-1004](./SEMRANK-1004_index-test-corpus.md) | Index Test Corpus | rust-indexer-engineer | 1 day | ⏳ Pending | 1003 |
| [SEMRANK-1005](./SEMRANK-1005_baseline-search-metrics.md) | Baseline Search Metrics | database-engineer | 1 day | ⏳ Pending | 1004, 0001 |
| [SEMRANK-1006](./SEMRANK-1006_integration-test-framework.md) | Integration Test Framework | integration-tester | 1 day | ⏳ Pending | 1004, 1005 |

**Phase 1 Total:** 4 tickets, 4 days

---

## Phase 2: Core Implementation (5-6 days)

Implement kind-based multipliers, exact match detection, query normalization, and debug mode.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-2003](./SEMRANK-2003_kind-based-multiplier.md) | Kind-Based Multiplier | database-engineer | 2 days | ⏳ Pending | 1006 |
| [SEMRANK-2004a](./SEMRANK-2004a_exact-match-sql.md) | Exact Match SQL | database-engineer | 1 day | ⏳ Pending | 2003 |
| [SEMRANK-2004b](./SEMRANK-2004b_query-normalization.md) | Query Normalization | general-purpose | 1 day | ⏳ Pending | 2004a |
| [SEMRANK-2005](./SEMRANK-2005_combine-multipliers.md) | Combine Multipliers | database-engineer | 1 day | ⏳ Pending | 2003, 2004a |
| [SEMRANK-2006](./SEMRANK-2006_debug-score-breakdown.md) | Debug Score Breakdown | database-engineer | 1 day | ⏳ Pending | 2005 |
| [SEMRANK-2007](./SEMRANK-2007_handle-edge-cases.md) | Handle Edge Cases | database-engineer | 1 day | ⏳ Pending | 2003, 2004a, 2004b |

**Phase 2 Total:** 6 tickets, 7 days

---

## Phase 3: Testing & Validation (4-5 days)

Create comprehensive test suite covering correctness, edge cases, performance, and regressions.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-3003](./SEMRANK-3003_integration-tests-ranking-correctness.md) | Integration Tests for Ranking Correctness | integration-tester | 2 days | ⏳ Pending | 1006, 2003, 2004a, 2005, 2006 |
| [SEMRANK-3004](./SEMRANK-3004_edge-case-testing.md) | Edge Case Testing | integration-tester | 1 day | ⏳ Pending | 1006, 2004b, 2007, 3003 |
| [SEMRANK-3005](./SEMRANK-3005_performance-benchmarks.md) | Performance Benchmarks | database-engineer | 1 day | ⏳ Pending | 1004, 1005, 2005, 2006 |
| [SEMRANK-3006](./SEMRANK-3006_regression-testing.md) | Regression Testing | integration-tester | 1 day | ⏳ Pending | 1006, 2003, 2004a, 2004b, 2005, 3003 |

**Phase 3 Total:** 4 tickets, 5 days

---

## Phase 4: Documentation & Deployment (2-3 days)

Document semantic ranking, create deployment runbook, and integrate tests into CI/CD.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-4003](./SEMRANK-4003_update-search-documentation.md) | Update Search Documentation | general-purpose | 1 day | ⏳ Pending | 2003, 2004a, 2004b, 2006 |
| [SEMRANK-4004](./SEMRANK-4004_create-deployment-runbook.md) | Create Deployment Runbook | general-purpose | 1 day | ⏳ Pending | 3003, 3005, 4003 |
| [SEMRANK-4005](./SEMRANK-4005_cicd-integration.md) | CI/CD Integration | github-actions-specialist | 1 day | ⏳ Pending | 3003, 3004, 3005, 3006 |

**Phase 4 Total:** 3 tickets, 3 days

---

## Phase 5: Verification & Commit (1-2 days)

Final verification of all work and commit changes with proper documentation.

| Ticket ID | Title | Agent | Effort | Status | Dependencies |
|-----------|-------|-------|--------|--------|--------------|
| [SEMRANK-5003](./SEMRANK-5003_final-verification.md) | Final Verification | verify-ticket | 1 day | ⏳ Pending | ALL (0001-4005) |
| [SEMRANK-5004](./SEMRANK-5004_commit-changes.md) | Commit Changes | commit-ticket | 0.5 days | ⏳ Pending | 5003 |

**Phase 5 Total:** 2 tickets, 1.5 days

---

## Summary Statistics

### By Phase
- **Phase 0:** 2 tickets, 3 days (MCP tool creation)
- **Phase 1:** 4 tickets, 4 days (foundation)
- **Phase 2:** 6 tickets, 7 days (implementation)
- **Phase 3:** 4 tickets, 5 days (testing)
- **Phase 4:** 3 tickets, 3 days (documentation)
- **Phase 5:** 2 tickets, 1.5 days (verification)

### By Agent Type
- **database-engineer:** 6 tickets (2003, 2004a, 2005, 2006, 2007, 3005)
- **integration-tester:** 4 tickets (1006, 3003, 3004, 3006)
- **general-purpose:** 5 tickets (0001, 1003, 2004b, 4003, 4004)
- **rust-indexer-engineer:** 1 ticket (1004)
- **github-actions-specialist:** 1 ticket (4005)
- **verify-ticket:** 2 tickets (0002, 5003)
- **commit-ticket:** 1 ticket (5004)

### By Status
- ⏳ **Pending:** 19 tickets
- ⏸️ **In Progress:** 0 tickets
- ✅ **Completed:** 2 tickets
- ❌ **Blocked:** 0 tickets

---

## Critical Path

The critical path through the project (longest dependency chain):

```
0001 → 0002 → 1003 → 1004 → 1005 → 1006 → 2003 → 2004a → 2004b → 2005 → 2006 → 3003 → 3004 → 3005 → 4003 → 4004 → 4005 → 5003 → 5004
```

**Critical Path Duration:** ~20-22 days (some parallelization possible)

---

## Parallel Execution Opportunities

After certain gates, multiple tickets can execute in parallel:

### After SEMRANK-1006 (Test Framework Ready)
Can execute in parallel:
- SEMRANK-2003 (kind multiplier)

### After SEMRANK-2005 (Multipliers Combined)
Can execute in parallel:
- SEMRANK-2006 (debug mode)
- SEMRANK-2007 (edge cases)

### After SEMRANK-2007 (All Implementation Complete)
Can execute in parallel:
- SEMRANK-3003 (integration tests)
- SEMRANK-3004 (edge case tests)
- SEMRANK-3005 (performance benchmarks)
- SEMRANK-3006 (regression tests)

### After SEMRANK-3006 (All Tests Complete)
Can execute in parallel:
- SEMRANK-4003 (documentation)
- SEMRANK-4005 (CI/CD integration)

---

## Ticket Workflow

Each ticket follows this workflow:

1. **Implementation** - Assigned agent completes work
2. **Testing** - unit-test-runner executes tests (no fixes)
3. **Verification** - verify-ticket checks acceptance criteria
4. **Commit** - commit-ticket creates Conventional Commit

If tests or verification fail, return to implementation agent for fixes.

---

## Files Affected Summary

### Created Files
- `/packages/maproom-mcp/src/tools/search.ts` (0001)
- `/packages/maproom-mcp/docs/baseline-behavior.md` (0002)
- `/packages/maproom-mcp/tests/integration/search-quality.test.ts` (1006)
- `/packages/maproom-mcp/scripts/benchmark-search.ts` (1005)
- `/packages/maproom-mcp/benchmarks/baseline-fts.csv` (1005)
- `/packages/maproom-mcp/benchmarks/semantic-ranking-fts.csv` (3005)
- `/packages/maproom-mcp/tests/unit/normalize.test.ts` (2004b)
- `/packages/maproom-mcp/tests/integration/edge-cases.test.ts` (3004)
- `/packages/maproom-mcp/tests/integration/regression.test.ts` (3006)
- `/packages/maproom-mcp/docs/search-ranking.md` (4003)
- `/packages/maproom-mcp/docs/deployment/semantic-ranking-rollout.md` (4004)
- `/packages/maproom-mcp/docs/ci-cd.md` (4005)
- `/packages/maproom-mcp/docs/verification/semrank-final-verification.md` (5003)

### Modified Files
- `/crates/maproom/src/search/fts.rs` (2003, 2004a, 2005, 2007)
- `/packages/maproom-mcp/README.md` (4003)
- `/docs/architecture/SEARCH_ARCHITECTURE.md` (4003)
- `.github/workflows/test.yml` (4005)
- `/packages/maproom-mcp/package.json` (4005)

---

## Progress Tracking

Update this section as tickets are completed:

**Last Updated:** 2025-11-19
**Status:** Phase 0 complete (0001-0002) - Starting Phase 1
**Next Action:** Execute SEMRANK-1003 (Create Test Corpus)

---

## Related Documents

- **Project Plan:** [../planning/plan.md](../planning/plan.md)
- **Architecture:** [../planning/architecture.md](../planning/architecture.md)
- **Analysis:** [../planning/analysis.md](../planning/analysis.md)
- **Quality Strategy:** [../planning/quality-strategy.md](../planning/quality-strategy.md)
- **Security Review:** [../planning/security-review.md](../planning/security-review.md)
- **Review Updates:** [../planning/review-updates.md](../planning/review-updates.md)

---

## Execution Commands

To work on this project systematically:

```bash
# Execute all tickets sequentially
/work-on-project SEMRANK

# Execute a single ticket
/single-ticket SEMRANK-0001
```
