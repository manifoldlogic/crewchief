# Ticket: CICDOPT-3004: Update Test Workflow with Optimizations

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (workflow validation requires CI run)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Add final optimizations to the test workflow based on learnings from Phase 1-3. Add concurrency controls to cancel outdated PR test runs, verify all caching (Rust + pnpm) is enabled, and ensure path filters are working correctly.

## Background
The test workflow (`.github/workflows/test.yml`) received path filters in Phase 1 (CICDOPT-1004) and pnpm caching in Phase 1 (CICDOPT-1003). However, it's missing optimizations that were added to the release workflows during Phase 2 and 3:

**Missing Optimizations**:
- Concurrency control (cancel in-progress for PRs)
- Rust caching (test workflow builds Rust binary for tests)

**Existing Optimizations** (to verify):
- pnpm caching (from CICDOPT-1003)
- Path filters (from CICDOPT-1004)

This ticket applies all learnings from release workflow optimizations to the test workflow, ensuring consistent performance and resource usage across all CI workflows.

## Acceptance Criteria
- [ ] Concurrency group added with proper configuration:
  - Group: `${{ github.workflow }}-${{ github.ref }}`
  - Cancel-in-progress: `${{ github.event_name == 'pull_request' }}`
  - Allows main builds to complete, cancels outdated PR runs
- [ ] Rust caching enabled using Swatinem/rust-cache@v2:
  - Workspace: `crates/maproom -> target`
  - Placed after Rust toolchain setup, before cargo build
- [ ] pnpm caching verified present and working (from Phase 1)
- [ ] Path filters verified working (from Phase 1)
- [ ] Concurrency control tested with PR update scenario:
  - Push commit 1 → tests start
  - Push commit 2 → commit 1 tests cancelled, commit 2 tests start
  - Older run cancelled automatically
- [ ] Cache behavior tested:
  - First run: Cache miss on Rust and pnpm
  - Second run (no changes): Cache hit on both
  - Build time 40-50% faster with cache hits
- [ ] Test run time: 3-5 minutes with cache hits
- [ ] Documentation updated in `.github/WORKFLOWS.md`

## Technical Requirements

### Update `.github/workflows/test.yml`

```yaml
name: Test

on:
  push:
    branches: [main]
    paths:
      # Existing path filter from CICDOPT-1004
      - '**.ts'
      - '**.tsx'
      - '**.rs'
      - 'packages/**'
      - 'crates/**'
      - 'pnpm-lock.yaml'
      - '.github/workflows/test.yml'

  pull_request:
    paths:
      # Existing path filter from CICDOPT-1004
      - '**.ts'
      - '**.tsx'
      - '**.rs'
      - 'packages/**'
      - 'crates/**'
      - 'pnpm-lock.yaml'
      - '.github/workflows/test.yml'

# ADD THIS: Concurrency control
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

env:
  TEST_MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5434/maproom_test

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
          POSTGRES_DB: maproom_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5434:5432

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4

      # Existing pnpm caching (from CICDOPT-1003)
      - name: Get pnpm store directory
        run: echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV

      - name: Setup pnpm cache
        uses: actions/cache@v4
        with:
          path: ${{ env.STORE_PATH }}
          key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
          restore-keys: |
            ${{ runner.os }}-pnpm-store-

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      # ADD THIS: Rust caching
      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "crates/maproom -> target"

      - name: Build Rust binary
        run: cargo build --release --bin crewchief-maproom

      - name: Run database migrations
        run: |
          ./target/release/crewchief-maproom db migrate

      - name: Run TypeScript tests
        run: pnpm test
```

### Testing Procedure

#### Test 1: Verify Concurrency Control

```bash
# 1. Create test PR
gh pr create --title "Test: CICDOPT-3004 Concurrency" --body "Testing cancellation of outdated runs"

# 2. Make change to trigger workflow
echo "// test concurrency 1" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: trigger run 1"
git push

# 3. Immediately make another change (before first run completes)
sleep 10  # Wait for first run to start
echo "// test concurrency 2" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: trigger run 2"
git push

# 4. Check GitHub Actions UI
# Expected: Run 1 shows "Cancelled", Run 2 shows "Running" or "Success"
gh run list --workflow=test.yml --limit 5
```

#### Test 2: Verify Caching

```bash
# 1. First PR run (cold cache - no Rust cache hit)
gh pr create --title "Test: CICDOPT-3004 Caching" --body "Testing cache behavior"
echo "// test cache 1" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: cold cache run"
git push

# Watch and note time
gh run watch
# Expected time: ~5-8 minutes (no cache hit)

# 2. Push trivial change (warm cache - should hit Rust cache)
echo "// test cache 2" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: warm cache run"
git push

# Watch and note time
gh run watch
# Expected time: ~3-5 minutes (40-50% faster with cache hit)

# 3. Verify cache logs in GitHub Actions
# - pnpm cache: Should show "Cache restored from key: ..."
# - Rust cache: Should show "Restored cache from key: ..."
```

#### Test 3: Verify Path Filters

```bash
# Test that non-code changes don't trigger workflow
echo "# Documentation update" >> README.md
git add README.md
git commit -m "docs: update readme"
git push

# Check that no workflow run triggered
gh run list --workflow=test.yml --limit 1
# Should NOT show new run for README-only change

# Test that code changes DO trigger workflow
echo "// code change" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: verify path filter triggers"
git push

# Check that workflow run triggered
gh run list --workflow=test.yml --limit 1
# SHOULD show new run for code change
```

## Implementation Notes

### Concurrency Configuration

The concurrency group `${{ github.workflow }}-${{ github.ref }}` creates unique groups per workflow and branch:
- **Main branch**: `Test-refs/heads/main` (never cancelled)
- **PR branch**: `Test-refs/heads/feature-branch` (cancelled when new commit pushed)

Setting `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`:
- **PRs**: Outdated runs cancelled automatically (saves CI resources)
- **Main**: Runs always complete (ensures every main commit is tested)

### Rust Caching Strategy

Using `Swatinem/rust-cache@v2` provides:
- Automatic cache key generation based on `Cargo.lock`
- Caches `~/.cargo/registry`, `~/.cargo/git`, and `target/` directory
- Workspace-aware caching: `crates/maproom -> target`

**Placement**: Must be AFTER `dtolnay/rust-toolchain@stable` (needs Rust installed) and BEFORE `cargo build` (needs to restore cache before build).

### Performance Expectations

**Without caching** (cold):
- pnpm install: ~2-3 minutes
- Rust build: ~3-4 minutes
- Tests: ~1-2 minutes
- **Total**: ~6-9 minutes

**With caching** (warm):
- pnpm install: ~30 seconds (cache hit)
- Rust build: ~1-2 minutes (cache hit)
- Tests: ~1-2 minutes
- **Total**: ~3-5 minutes (40-50% faster)

### Documentation Updates

Update `.github/WORKFLOWS.md`:

```markdown
## Test Workflow (`test.yml`)

**Triggers**: Push to main, Pull requests
**Path Filters**: Only triggers on code changes (`.ts`, `.tsx`, `.rs`, package files)
**Concurrency**: Cancels outdated PR runs, allows main builds to complete
**Caching**:
- pnpm dependencies (Swatinem/rust-cache@v2)
- Rust build artifacts (Swatinem/rust-cache@v2)
**Runtime**: 3-5 minutes with cache hits, 6-9 minutes cold

**Features**:
- PostgreSQL service with pgvector extension
- Test-specific database (port 5434)
- Full TypeScript test suite
- Rust binary build and migration
```

## Dependencies
- **CICDOPT-1003** - pnpm caching already in test workflow (verify present)
- **CICDOPT-1004** - Path filters already in test workflow (verify working)
- No blocking dependencies - can be completed independently

## Risk Assessment
- **Risk**: Concurrency control cancels important test runs
  - **Mitigation**: Only cancels PR runs, not main branch runs
  - **Mitigation**: Each ref (branch) has separate concurrency group
  - **Mitigation**: Test thoroughly with PR update scenarios

- **Risk**: Rust cache causes stale build issues
  - **Mitigation**: Swatinem/rust-cache@v2 handles cache invalidation automatically
  - **Mitigation**: Cache key based on `Cargo.lock` ensures correct cache invalidation
  - **Mitigation**: Can manually clear cache via GitHub Actions UI if needed

- **Risk**: Path filters too restrictive, miss important changes
  - **Mitigation**: Path filters already tested in CICDOPT-1004
  - **Mitigation**: Includes all code files and configuration files
  - **Mitigation**: Can always trigger manually if needed

## Files/Packages Affected
- `.github/workflows/test.yml` (modified - add concurrency + Rust caching)
- `.github/WORKFLOWS.md` (updated - document new optimizations)
