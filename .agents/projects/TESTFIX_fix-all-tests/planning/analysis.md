# Analysis: Fix All Tests

## Problem Definition

The test suite across the CrewChief monorepo has significant failures that prevent CI from passing. Tests have fallen out of sync with the implementation due to API changes in the Rust `crewchief-maproom` crate and TypeScript packages.

### Current State Summary

**Rust Tests (crewchief-maproom)**
- 190 compilation errors across ~100 test files
- Tests won't compile due to API drift between tests and implementation

**TypeScript Tests**
- CLI package (`@crewchief/cli`): 53 tests failing, 2634 passing
- MCP package (`@crewchief/maproom-mcp`): Database connectivity required (tests fail without PostgreSQL)
- Daemon-client (`@crewchief/daemon-client`): 5 tests failing (performance tests require daemon binary)
- VSCode extension (`@crewchief/vscode-maproom`): 16 tests failing, 336 passing

**Test Environment Issues**
- Missing `vitest.config.ts` in CLI package causes test discovery in nested worktree directories
- Stale worktree at `packages/cli/.crewchief/worktrees/variant-test-*` causes ~30 duplicate test executions

---

### Verified Baseline (TESTFIX-1002, 2025-11-27)

After TESTFIX-1001 environment cleanup:

| Package | Test Files | Tests | Passing | Failing | Skipped | Notes |
|---------|------------|-------|---------|---------|---------|-------|
| **Rust (cargo check --tests)** | N/A | N/A | N/A | 190 errors | N/A | Compilation errors, not runtime |
| **CLI** | 53 | 1094 | 1078 | 16 | 0 | Vitest config fixed discovery |
| **VSCode** | 15 | 352 | 336 | 16 | 0 | Timeout issues in orchestrator |
| **Daemon-client** | 5 | 80 | 60 | 16 | 4 | Requires daemon binary |
| **MCP** | N/A | N/A | 2 | 1 | N/A | DB connectivity (`maproom-postgres` not found) |

**Key findings:**
- CLI failures reduced from 53 to 16 after vitest config fix (TESTFIX-1001)
- Daemon-client has more failures than expected (16 vs 5) - daemon crashes on startup
- MCP `test:connection` passes, full test requires PostgreSQL

**CI Configuration**
- Workflow: `.github/workflows/test.yml`
- Jobs: SQLite E2E, MCP SQLite, Rust SQLite, PostgreSQL Integration, Rust PostgreSQL
- Currently failing due to test compilation errors

## Root Cause Analysis

### 1. Rust API Changes (Major)

The `crewchief-maproom` crate underwent significant refactoring without updating tests:

| Error Category | Count | Root Cause |
|----------------|-------|------------|
| `EmbeddingService::new()` signature | 42 | Changed from 1 arg to 2 args |
| `ChangeType::New` struct fields | 30 | Changed from `{hash}` to tuple variant `(ContentHash)` |
| Missing `.await` on async methods | 19 | `from_env()` became async |
| `SearchOptions` removed field | 19 | `include_debug` field removed |
| `EmbeddingService` method access | 13+ | Methods return struct, not Result now |
| `FinalSearchResults.timing` | 8 | Field removed/renamed |
| `cost_metrics()` method | 7 | Method removed from EmbeddingService |
| `BasicWeightedFusion::with_weights` | 5 | Method removed/renamed |
| `ChangeType::Deleted` struct fields | 5 | Changed from `{hash}` to tuple variant |
| Missing `common` module | 4 | Test module not found |

### 2. TypeScript Test Issues

**CLI Package Issues:**
- `ScanOrchestrator` tests expect specific binary path (`crewchief-maproom`) but get actual path
- Pre-flight validator expects different message formats
- Variant injection tests failing due to worktree creation issues

**MCP Package Issues:**
- Tests require PostgreSQL connection (`maproom-postgres:5432`)
- `pnpm test` runs `run-blob-sha-tests.cjs` which needs database connectivity
- Use `pnpm test:connection` for local testing without database
- SQLite fixture tests available via `pnpm test:sqlite`

**VSCode Extension Issues:**
- 16 test failures in `src/process/orchestrator.test.ts` and `src/test/integration.test.ts`
- Tests timeout waiting for binary spawn operations
- May need mock infrastructure improvements

**Daemon-Client Package Issues:**
- Performance tests fail because daemon binary crashes immediately
- Unit tests pass (42 tests in rpc.test.ts and errors.test.ts)
- Only 5 failures in performance.test.ts (require running daemon)

### 3. Test Environment Pollution

A stale worktree directory at `packages/cli/.crewchief/worktrees/variant-test-variant-minimal-1763839369508` is causing:
- Vitest to discover and run duplicate test files (CLI package has no vitest.config.ts to exclude)
- Tests to fail trying to create nested worktrees
- ~30 false-positive test failures due to duplicate execution

**Root Cause:** The CLI package lacks a `vitest.config.ts` file, so vitest uses default discovery and finds test files in nested `.crewchief/` directories.

## Existing Solutions Research

### Industry Patterns

1. **Test-Implementation Sync** - Keep tests updated with implementation via:
   - Co-located tests (same file/directory as implementation)
   - Breaking changes require test updates in same PR
   - API versioning with deprecated aliases

2. **Test Isolation** - Prevent environment pollution:
   - Git worktree cleanup in test teardown
   - Vitest `exclude` patterns for nested directories
   - CI artifact isolation

3. **Multi-Package Testing** - Monorepo test strategies:
   - Package-specific test configurations
   - Shared test utilities as separate package
   - Dependency-aware test ordering

### Current Project Patterns

The project uses:
- Vitest for TypeScript (isolated configs per package)
- Cargo test for Rust (feature flags for backends)
- CI separates SQLite (fast, no deps) from PostgreSQL (integration)

## Scope Assessment

### In Scope

1. **Rust Test Compilation Fixes**
   - Update EmbeddingService instantiation patterns
   - Fix ChangeType enum usage
   - Add missing `.await` calls
   - Update struct field access
   - Remove references to deleted APIs

2. **TypeScript Test Fixes**
   - Fix assertion message expectations
   - Update binary path expectations
   - Add proper test isolation

3. **Environment Cleanup**
   - Remove stale worktree directory
   - Add vitest exclude patterns
   - Ensure tests clean up after themselves

4. **CI Configuration Verification**
   - Verify all packages have tests configured
   - Ensure proper dependencies for each test job
   - Add missing test coverage

### Out of Scope

- Adding new tests for untested functionality
- Performance optimization of test suite
- Test coverage threshold changes
- New CI test jobs beyond fixing existing

## API Change Details

### EmbeddingService Changes

**Old API (in tests):**
```rust
EmbeddingService::new(config)
```

**New API (implementation):**
```rust
EmbeddingService::new(provider: Box<dyn EmbeddingProvider>, cache: Arc<EmbeddingCache>)
```

**Migration Pattern:**
```rust
// For tests, use from_env() which handles config internally
let service = EmbeddingService::from_env().await?;

// Or construct manually with provider and cache
let provider = create_provider_from_env().await?;
let cache = EmbeddingCache::new(CacheConfig::default())?;
let service = EmbeddingService::new(provider, Arc::new(cache));
```

### ChangeType Enum Changes

**Old API (in tests):**
```rust
ChangeType::New { hash: content_hash }
ChangeType::Deleted { hash: content_hash }
```

**New API (implementation):**
```rust
ChangeType::New(content_hash)
ChangeType::Deleted(content_hash)
```

### SearchOptions Changes

**Removed field:** `include_debug`
**Migration:** Remove field from struct construction

### FinalSearchResults Changes

**Removed/changed fields:** `timing`, `query_processing`, individual score fields
**Migration:** Use `metadata` field for search metadata

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing working tests | High | Run full test suite after each change |
| Missing test coverage after removal | Medium | Verify each test still tests meaningful behavior |
| CI configuration mismatch | Medium | Test locally before push |
| Stale worktree causing issues | Low | Clean up early in project |

## Dependencies

- Rust toolchain (stable)
- Node.js 20+
- pnpm
- PostgreSQL (for integration tests)
- Git (for worktree cleanup)

## Success Criteria

1. All Rust tests compile: `cargo check --tests` succeeds
2. All Rust tests pass: `cargo test` succeeds (SQLite and PostgreSQL features)
3. All TypeScript tests pass: `pnpm test` succeeds in all packages
4. CI pipeline passes: All jobs in `.github/workflows/test.yml` succeed
5. No stale worktree artifacts remain
