# Plan: Fix All Tests

## Overview

This plan organizes the test fixing work into logical phases with clear deliverables and verification steps.

## Phase 1: Environment Cleanup (Tickets 1001-1002)

**Goal**: Remove test pollution that causes false failures

### TESTFIX-1001: Clean Stale Worktrees and Configure Vitest
- Remove `packages/cli/.crewchief/worktrees/variant-test-*` directories
- **Create `packages/cli/vitest.config.ts`** with explicit exclude patterns:
  ```typescript
  import { defineConfig } from 'vitest/config'

  export default defineConfig({
    test: {
      globals: true,
      environment: 'node',
      include: ['src/**/*.test.ts', 'tests/**/*.test.ts'],
      exclude: [
        '**/node_modules/**',
        '**/.crewchief/**',
        '**/dist/**',
      ],
    },
  })
  ```
- Verify vitest only discovers intended test files (not from nested worktrees)

### TESTFIX-1002: Verify Local Test Environment
- Run `cargo check --tests` and document exact error count (baseline: 190)
- Run `pnpm test` and document exact failure counts:
  - CLI: 53 failures
  - VSCode: 16 failures
  - Daemon-client: 5 failures (performance tests, requires daemon binary)
  - MCP: Database connectivity issues (use `pnpm test:connection` locally)
- Establish baseline for measuring progress

**Deliverable**: Clean test environment with documented baseline

---

## Phase 2: Rust Test Compilation (Ticket 1003)

**Goal**: Get all Rust tests to compile (190 errors → 0)

### TESTFIX-1003: Fix All Rust Test Compilation Errors

Fix all 190 compilation errors using mechanical API migrations:

**Pattern 1: EmbeddingService API (~42 errors)**
- Update `EmbeddingService::new(config)` → `EmbeddingService::from_env().await?` or two-arg constructor
- Add missing `.await` calls on async methods
- Files: `tests/embedding_service_test.rs`, `tests/embedding_integration.rs`, `tests/e2e_*.rs`

**Pattern 2: ChangeType Enum (~35 errors)**
- Update `ChangeType::New { hash }` → `ChangeType::New(hash)`
- Update `ChangeType::Deleted { hash }` → `ChangeType::Deleted(hash)`
- Files: `tests/incremental_*.rs`, `tests/cleanup_*.rs`, `tests/embedding_inheritance_test.rs`

**Pattern 3: SearchOptions and SearchResults (~30 errors)**
- Remove `include_debug` field from SearchOptions construction
- Update `FinalSearchResults` field access (no `timing`, use `metadata`)
- Update `ChunkSearchResult` field access (use `source_scores` map)
- Files: `tests/search_*.rs`, `tests/fusion_*.rs`, `tests/context_*.rs`

**Pattern 4: BasicWeightedFusion API (~5 errors)**
- Update `BasicWeightedFusion::with_weights()` calls
- Files: `tests/weighted_fusion_test.rs`, `tests/fusion_*.rs`

**Pattern 5: Missing Modules and Private Access (~6 errors)**
- Remove `mod common` references that fail to find module
- Fix private module access (`feature_flags`)
- Fix metrics registry import
- Files: `tests/context_assembler_test.rs`, `tests/metrics_integration_test.rs`

**Pattern 6: Remaining Errors (~20 errors)**
- Fix EmbeddingConfig missing field errors
- Address any edge cases

**Verification**: `cargo check --tests` exits with 0 errors

**Deliverable**: All Rust tests compile successfully

---

## Phase 3: Rust Test Execution (Ticket 1004)

**Goal**: Get Rust tests passing

### TESTFIX-1004: Run and Verify Rust Tests

**SQLite Tests (Primary)**:
- Execute `cargo test --features sqlite`
- Fix any runtime failures (not compilation)
- Document any tests that need database or network

**PostgreSQL Tests (Compilation Only)**:
- Execute `cargo check --tests --features postgres`
- Full execution requires PostgreSQL connection (CI handles this)
- Ensure compilation passes for CI

**Deliverable**: Rust tests pass locally (SQLite) and compile for CI (PostgreSQL)

---

## Phase 4: TypeScript Test Fixes (Tickets 1005-1007)

**Goal**: Get TypeScript tests passing (74 failures → 0)

### TESTFIX-1005: Fix CLI Package Tests (53 failures)

**ScanOrchestrator Tests (~3 failures)**:
- Update binary path expectations (flexible matching)
- Fix spawn argument assertions
- Files: `packages/cli/src/search-optimization/scan-orchestrator.test.ts`

**PreFlightValidator Tests (~5 failures)**:
- Update message format assertions
- Handle different worktree states
- Files: `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`

**Search Optimization Tests (~7 failures)**:
- Fix genetic-iterator tests
- Fix competition-runner tests
- Files: `packages/cli/tests/search-optimization/genetic-iterator.test.ts`, `packages/cli/tests/search-optimization/competition-runner.test.ts`

**Variant Injection Tests (~6 failures)**:
- Fix worktree creation in nested test environment
- Ensure proper cleanup
- Files: `packages/cli/tests/sdk/variant-injection.test.ts`

**Remaining CLI failures (~32)**:
- Address any other CLI test failures
- Most likely duplicates from stale worktree (fixed by Phase 1)

**Verification**: `pnpm test` in `packages/cli` passes

### TESTFIX-1006: Fix VSCode Extension Tests (16 failures)

**Orchestrator Tests**:
- Fix process spawn timeouts
- Update mock infrastructure
- Files: `packages/vscode-maproom/src/process/orchestrator.test.ts`

**Integration Tests**:
- Fix integration test failures
- Files: `packages/vscode-maproom/src/test/integration.test.ts`

**Verification**: `pnpm test` in `packages/vscode-maproom` passes

### TESTFIX-1007: Configure MCP and Daemon-Client Tests

**MCP Package**:
- Document that `pnpm test` requires PostgreSQL (`maproom-postgres:5432`)
- Use `pnpm test:connection` for local testing without database
- Use `pnpm test:sqlite` for SQLite fixture tests
- CI handles full database tests

**Daemon-Client Package**:
- 42 unit tests pass (no changes needed)
- 5 performance tests fail (require running daemon binary)
- Document performance tests are CI-only

**Verification**: Local-safe test commands documented

**Deliverable**: All TypeScript tests pass (`pnpm test` in all packages)

---

## Phase 5: CI Verification (Tickets 1008-1009)

**Goal**: Ensure CI pipeline passes

### TESTFIX-1008: Verify CI Configuration
- Review test.yml covers all packages
- Ensure dependencies are correct
- Add any missing test configurations

**Files**:
- `.github/workflows/test.yml`

### TESTFIX-1009: Full CI Validation
- Push changes to branch
- Verify all CI jobs pass
- Fix any CI-specific failures

**Deliverable**: All CI jobs pass

---

## Agent Assignments

| Ticket | Primary Agent | Rationale |
|--------|---------------|-----------|
| TESTFIX-1001 | general-purpose | File cleanup, vitest config |
| TESTFIX-1002 | unit-test-runner | Baseline test execution |
| TESTFIX-1003 | rust-indexer-engineer | Rust API expertise |
| TESTFIX-1004 | unit-test-runner | Test execution and verification |
| TESTFIX-1005 | general-purpose | TypeScript CLI test modifications |
| TESTFIX-1006 | vscode-extension-specialist | VSCode test fixes |
| TESTFIX-1007 | general-purpose | MCP/daemon-client test config |
| TESTFIX-1008 | github-actions-specialist | CI configuration |
| TESTFIX-1009 | unit-test-runner | Final verification |

---

## Dependencies

```
┌─────────────┐
│ Phase 1     │ ← Start here (environment cleanup)
│ (1001-1002) │
└──────┬──────┘
       │
       ├──────────────────────┐
       ▼                      ▼
┌─────────────┐        ┌─────────────┐
│ Phase 2     │        │ Phase 4     │ ← TypeScript (parallel)
│ (1003)      │        │ (1005-1007) │
│ Rust compile│        │ TS tests    │
└──────┬──────┘        └──────┬──────┘
       │                      │
       ▼                      │
┌─────────────┐               │
│ Phase 3     │               │
│ (1004)      │               │
│ Rust tests  │               │
└──────┬──────┘               │
       │                      │
       └──────────┬───────────┘
                  ▼
           ┌─────────────┐
           │ Phase 5     │ ← Final verification
           │ (1008-1009) │
           └─────────────┘
```

**Note**: Phase 4 (TypeScript) can run in parallel with Phase 2-3 (Rust) since they are independent.

---

## Timeline Considerations

This is a focused cleanup project. Work should proceed sequentially through phases, with parallel execution within phases where dependencies allow.

**Critical Path**:
1. Environment cleanup (unblocks accurate test counting)
2. Rust compilation (unblocks all Rust testing)
3. CI verification (confirms end-to-end success)

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Rust compilation errors | 190 | 0 |
| CLI test failures | 53 | 0 |
| VSCode test failures | 16 | 0 |
| Daemon-client performance tests | 5 | 0 (or documented as CI-only) |
| MCP database connectivity | N/A | Documented local commands |
| CI jobs passing | 0/5 | 5/5 |
| Stale worktrees | 1 | 0 |

---

## Rollback Strategy

If a ticket introduces new failures:
1. Revert the specific changes
2. Re-run tests to confirm revert is clean
3. Investigate root cause before retrying

---

## Post-Project

After all tickets complete:
1. Run full test suite locally
2. Push to main branch (or PR)
3. Verify CI passes
4. Archive project
