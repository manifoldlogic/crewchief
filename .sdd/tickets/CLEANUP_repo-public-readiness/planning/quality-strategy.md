# Quality Strategy: Repository Public Readiness

## Testing Philosophy

Repository cleanup is unique in that the primary risk is **regression** - accidentally breaking existing functionality or removing needed files. Our quality strategy focuses on:

1. **Non-destructive verification** - Test before and after each phase
2. **Reversibility** - Every change can be undone via archive branch
3. **Incremental validation** - Check after each major removal
4. **Comprehensive final sweep** - Full test suite before completion

## Coverage Requirements

### Build Verification

**Must pass after every phase:**
```bash
# TypeScript/Node
pnpm install
pnpm build

# Rust
cargo build --release

# Docker (if applicable)
docker build -t crewchief-test .
```

### Test Suite Execution

**Full test suite must pass:**
```bash
# TypeScript tests
pnpm test

# Rust tests
cargo test

# Integration tests
pnpm test:integration  # if exists
```

**Coverage thresholds:** Existing coverage levels must be maintained. No cleanup task should reduce test coverage.

## Test Types

### Pre-Cleanup Baseline

**Scope:** Establish working state before any changes.

**Actions:**
1. Run full test suite, record results
2. Capture `du -sh` on key directories
3. Run security scan, save as baseline
4. Document current file count

### Per-Phase Validation

**Scope:** After each phase, verify no regressions.

**Checklist:**
- [ ] `pnpm build` succeeds
- [ ] `pnpm test` passes (same count as baseline)
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] No new linting errors

### Final Validation

**Scope:** Complete verification before marking cleanup complete.

**Actions:**
1. Full test suite (TypeScript + Rust)
2. Security rescan
3. Build all packages
4. Manual smoke test of key features
5. Repository structure review

## Critical Paths

The following must remain functional throughout cleanup:

### 1. Core CLI Functionality

**Tests to run:**
```bash
pnpm --filter @crewchief/cli test
```

**Manual verification:**
- `crewchief --help` works
- Basic commands function

### 2. Maproom Indexing

**Tests to run:**
```bash
cargo test -p maproom
```

**Manual verification:**
- `crewchief-maproom scan` works on test repo
- Search returns results

### 3. MCP Server

**Tests to run:**
```bash
pnpm --filter @crewchief/maproom-mcp test
```

**Manual verification:**
- MCP server starts
- Tools respond to queries

### 4. VSCode Extension Build

**Tests to run:**
```bash
pnpm --filter @crewchief/vscode-maproom test
```

**Manual verification:**
- Extension compiles without errors

## Negative Testing Requirements

### What NOT to Remove

Before removing any file, verify it is NOT:

1. **Referenced in imports** - grep for filename in source files
2. **Referenced in configs** - check `package.json`, `Cargo.toml`, CI configs
3. **Referenced in tests** - check test files for file references
4. **Part of build output** - check if file is generated

### Files That Look Temporary But Aren't

Be careful with:
- `*.example` files - these are templates, not temporary
- Files in `fixtures/` directories - test data, needed
- `migrations/*.sql` - database migrations, critical
- `scripts/*.sh` - may be needed for CI/release

## Test Data Strategy

### Preserve Test Fixtures

Do not remove:
- `/crates/maproom/tests/fixtures/`
- `/packages/*/tests/fixtures/` (if exists)
- `/packages/maproom-mcp/tests/corpus/`
- `/packages/maproom-mcp/tests/setup/`

### Remove Test Artifacts

Safe to remove:
- `/tests/manual/*.md` - manual test reports (not fixtures)
- Any `*.log` in test directories
- Generated coverage reports in `coverage/`

## Quality Gates

### Before Starting Any Phase

- [ ] Current branch is a feature branch (not main)
- [ ] Archive branch created with current state
- [ ] Baseline test results recorded
- [ ] No uncommitted changes

### Before Completing Each Phase

- [ ] All tests pass
- [ ] Build succeeds
- [ ] No new linting errors
- [ ] Changes committed with descriptive message
- [ ] Spot-check removed files were truly unused

### Before Marking Ticket Complete

- [ ] Full test suite passes
- [ ] Security scan clean
- [ ] Documentation updated
- [ ] Archive branch pushed to remote
- [ ] CONTRIBUTING.md updated
- [ ] Final review of repository structure
- [ ] All success metrics from plan.md verified

## Rollback Procedure

If any phase breaks functionality:

1. **Stop immediately** - Do not proceed with more deletions
2. **Identify what broke** - Run tests to find failures
3. **Check git diff** - What was removed?
4. **Restore from archive branch** - `git checkout archive/project-history -- <path>`
5. **Re-run tests** - Verify restoration fixed issue
6. **Document** - Note what was incorrectly flagged for removal

## Automated Checks (Optional)

Consider adding to CI:

```yaml
# .github/workflows/cleanup-guard.yml
name: Cleanup Guard
on: [push, pull_request]
jobs:
  check-no-bloat:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check for log files
        run: |
          if find . -name "*.log" -not -path "./node_modules/*" | grep -q .; then
            echo "Found .log files in repo!"
            find . -name "*.log" -not -path "./node_modules/*"
            exit 1
          fi
      - name: Check for backup files
        run: |
          if find . -name "*.bak" -o -name "*.tmp" | grep -q .; then
            echo "Found backup/temp files in repo!"
            exit 1
          fi
```
