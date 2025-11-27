# Quality Strategy: Fix All Tests

## Testing Philosophy

This project is about fixing tests, not writing new ones. The quality strategy focuses on:
1. **Preserving test intent** - Tests should still validate the same behaviors
2. **Confidence before coverage** - Passing tests that catch real bugs
3. **Regression prevention** - CI must gate all PRs

## Verification Approach

### Primary Success Criteria

| Criterion | Verification Method | Pass Condition |
|-----------|---------------------|----------------|
| Rust tests compile | `cargo check --tests` | Exit code 0, no errors |
| Rust SQLite tests pass | `cargo test --features sqlite` | All tests pass |
| Rust PostgreSQL tests compile | `cargo check --tests --features postgres` | Exit code 0 |
| CLI tests pass | `pnpm test` in packages/cli | All tests pass |
| CLI vitest config | `packages/cli/vitest.config.ts` exists | Excludes `.crewchief/**` |
| MCP tests (local) | `pnpm test:connection` in packages/maproom-mcp | Connection test passes |
| MCP tests (CI) | `pnpm test` with PostgreSQL available | All tests pass |
| Daemon-client unit tests | `pnpm test` in packages/daemon-client | Unit tests pass (42) |
| VSCode tests pass | `pnpm test` in packages/vscode-maproom | All tests pass |
| CI pipeline passes | GitHub Actions | All jobs green |

### Per-Ticket Verification

Each ticket should verify:

1. **Before changes**: Document current error state
2. **After changes**: Tests compile and pass
3. **Regression check**: No new failures introduced

## Critical Paths

### Path 1: Rust Test Compilation

The most critical path - blocking all Rust testing.

```
cargo check --tests
├── embedding_service_test.rs  ← HIGH PRIORITY
├── incremental_*_test.rs      ← HIGH PRIORITY
├── search_*_test.rs           ← MEDIUM PRIORITY
└── other tests                ← LOWER PRIORITY
```

**Verification**: `cargo check --tests 2>&1 | grep "^error" | wc -l` should be 0

### Path 2: TypeScript Test Execution

Secondary path - some tests already pass.

```
pnpm test
├── packages/cli           ← 53 failures to fix
├── packages/maproom-mcp   ← Use test:connection locally, test:sqlite for fixtures
├── packages/daemon-client ← 42 unit tests pass, 5 performance tests need daemon
└── packages/vscode-maproom ← 16 failures to fix
```

**Local Test Commands:**
- CLI: `pnpm test` (requires vitest.config.ts to exclude .crewchief)
- MCP: `pnpm test:connection` (no database) or `pnpm test:sqlite` (SQLite fixture)
- Daemon-client: `pnpm test` (unit tests pass, performance tests skip without daemon)
- VSCode: `pnpm test`

**CI Test Commands:**
- MCP: `pnpm test` (PostgreSQL available via test.yml)
- Daemon-client: Full suite (daemon binary built)

**Verification**: All packages pass with appropriate commands

### Path 3: CI Pipeline

Final integration check.

```
.github/workflows/test.yml
├── test-sqlite-e2e       ← Shell script tests
├── test-mcp-sqlite       ← TypeScript + SQLite
├── test-rust-sqlite      ← Rust + SQLite
├── test-postgres         ← TypeScript + PostgreSQL
└── test-rust-postgres    ← Rust + PostgreSQL (compile only)
```

**Verification**: GitHub Actions workflow shows all jobs passing

## Risk Mitigation

### Risk: Tests Pass But Don't Test Anything

**Mitigation**:
- Review each test modification to ensure assertions remain meaningful
- Avoid simply removing failing assertions
- Add comments explaining test intent if unclear

### Risk: Environment-Dependent Failures

**Mitigation**:
- Test locally with clean environment
- Verify in CI (Actions)
- Document any environment requirements

### Risk: Flaky Tests

**Mitigation**:
- Run each test multiple times locally
- Add appropriate timeouts
- Use deterministic test data

## Test Categories

### Unit Tests (Fast, Isolated)

- Rust: `src/**/*_test.rs`, inline `#[test]` functions
- TypeScript: `*.test.ts` files with mocked dependencies

**Quality checks**:
- No network calls
- No database (or in-memory only)
- Deterministic results

### Integration Tests (Slower, Real Dependencies)

- Rust: `tests/*.rs` files
- TypeScript: `*.int.test.ts`, `*.e2e.test.ts` files

**Quality checks**:
- Proper cleanup after each test
- Isolated test databases
- Reasonable timeouts

### E2E Tests (Full Stack)

- Bash: `tests/e2e/*.sh`
- TypeScript: Tests that spawn real processes

**Quality checks**:
- Clear setup/teardown
- Documented prerequisites
- Timeout handling

## Acceptance Criteria Matrix

| Ticket Type | Must Pass | Should Pass | Nice to Have |
|-------------|-----------|-------------|--------------|
| Rust API fix | Compilation | Related tests | All Rust tests |
| TypeScript fix | Target test file | Same test suite | All TS tests |
| CI fix | Target job | All CI jobs | Full workflow |

## Continuous Verification

### During Development

```bash
# Quick Rust check (frequent)
cargo check --tests

# Full Rust test (before commit)
cargo test --features sqlite

# Quick TypeScript check (frequent)
pnpm test --filter @crewchief/cli

# MCP local check (no database required)
cd packages/maproom-mcp && pnpm test:connection

# Full test suite (before PR)
pnpm test  # Note: MCP tests may fail without PostgreSQL
```

**Prerequisite:** Ensure `packages/cli/vitest.config.ts` exists to prevent duplicate test discovery from nested worktrees.

### CI Requirements

Every commit that fixes tests should:
1. Not introduce new failures
2. Reduce total error count
3. Pass CI for affected components

## Post-Implementation Verification

After all tickets complete:

1. **Clean build**: Fresh clone, `pnpm install`, `pnpm build`
2. **Full test suite**: `pnpm test` (all packages)
3. **Rust full suite**: `cargo test` (all features)
4. **CI validation**: Push branch, verify Actions pass

## Documentation Requirements

### Required Per Ticket

- [ ] Error count before/after
- [ ] Files modified
- [ ] Test verification method

### Required Post-Project

- [ ] Summary of all API changes migrated
- [ ] Known limitations or skipped tests
- [ ] CI configuration changes (if any)
