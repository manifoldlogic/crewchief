# GitHub Actions Workflows Documentation

Comprehensive guide to CrewChief's CI/CD workflows, including architecture, usage patterns, troubleshooting, and maintenance procedures.

**Target Audience**: Developers working with GitHub Actions workflows, DevOps engineers, and team members maintaining the CI/CD pipeline.

**Last Updated**: 2025-01 (Phase 2 - Reusable Infrastructure)

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Workflow Catalog](#workflow-catalog)
3. [Reusable Workflows](#reusable-workflows)
4. [Testing Procedures](#testing-procedures)
5. [Common Tasks](#common-tasks)
6. [Troubleshooting Guide](#troubleshooting-guide)
7. [Rollback Procedures](#rollback-procedures)
8. [Metrics & Monitoring](#metrics--monitoring)
9. [Best Practices](#best-practices)
10. [Additional Resources](#additional-resources)

---

## Architecture Overview

### Design Principles

CrewChief's CI/CD architecture follows these core principles:

1. **Build Once, Use Many**: Reusable workflows eliminate duplication
2. **Cache Aggressively**: Minimize redundant downloads and builds
3. **Single Source of Truth**: Centralized logic for consistency
4. **Fail Fast**: Early validation catches issues quickly
5. **Parallel When Possible**: Maximize throughput with concurrent builds

### Workflow Structure

```
.github/workflows/
├── test.yml                          # PR/push testing (3-5 min)
├── reusable-rust-build.yml           # Reusable: Rust binaries (4 platforms)
├── reusable-typescript-build.yml     # Reusable: TypeScript packages
├── test-reusable-rust.yml            # Test caller (temporary)
├── test-reusable-typescript.yml      # Test caller (temporary)
├── build-and-publish-cli.yml         # CLI npm publish (future: uses reusables)
├── build-and-publish-maproom-mcp.yml # Maproom npm publish (future: uses reusables)
└── publish-maproom-mcp-image.yml     # Docker image publish
```

### Workflow Dependencies

```
Test Workflow (test.yml)
  - Runs on: push to main, pull requests
  - Triggers: Only when code/deps/workflows/schemas change (path filters)
  - Uses: pnpm caching, Rust caching

Reusable Workflows (Phase 2)
  - reusable-rust-build.yml
    ├─ Called by: build-and-publish-cli.yml (future)
    ├─ Called by: build-and-publish-maproom-mcp.yml (future)
    └─ Provides: Cross-platform Rust binaries

  - reusable-typescript-build.yml
    ├─ Called by: build-and-publish-cli.yml (future)
    ├─ Called by: build-and-publish-maproom-mcp.yml (future)
    └─ Provides: TypeScript dist/ artifacts

Release Workflows
  - build-and-publish-cli.yml
    ├─ Triggers: @crewchief/cli@v* tags
    └─ Future: Will use both reusable workflows (Phase 3)

  - build-and-publish-maproom-mcp.yml
    ├─ Triggers: @crewchief/maproom-mcp@v* tags
    └─ Future: Will use both reusable workflows (Phase 3)

  - publish-maproom-mcp-image.yml
    ├─ Triggers: @crewchief/maproom-mcp@v* tags
    └─ Builds: Multi-platform Docker images
```

### Performance Baselines

**Before Optimization**:
- Test workflow: 5-8 min
- CLI release: 12-15 min
- Maproom-MCP release: 25-30 min (two separate workflows)

**After Phase 1 (Caching + Path Filters)**:
- Test workflow: 3-5 min (40% faster)
- Test runs: 80% reduction (path filters skip docs-only changes)
- Cache hit rate: 80%+ (pnpm store, Rust dependencies)

**After Phase 3 (Consolidation - Future)**:
- CLI release: 6-8 min (50% faster)
- Maproom-MCP release: 8-10 min (67% faster, single unified workflow)

---

## Workflow Catalog

### test.yml

**Purpose**: Run tests on pull requests and pushes to main

**Triggers**:
- `push` to `main` branch (when code changes)
- `pull_request` (when code changes)
- **Path filters**: Only runs when these files change:
  - Rust code: `crates/**`, `Cargo.toml`, `Cargo.lock`, `**.rs`
  - TypeScript code: `packages/*/src/**`, `packages/*/tests/**`, `**.ts`
  - Dependencies: `pnpm-lock.yaml`
  - Workflows: `test.yml`, `reusable-rust-build.yml`, `reusable-typescript-build.yml`
  - Database: `packages/maproom-mcp/config/init.sql`, `crates/maproom/migrations/**`

**Duration**: 3-5 minutes (with cache hit), 6-9 minutes cold

**Key Features**:
- **Concurrency control**: Cancels outdated PR runs, allows main builds to complete
- **pnpm store caching**: 40-60% faster installs
- **Rust dependency caching**: `Swatinem/rust-cache@v2` for cargo dependencies
- **Isolated PostgreSQL test database**: pgvector/pgvector:pg16 service container
- **Rust migrations for schema setup**: Ensures test DB matches production schema
- **Path filters**: Reduce unnecessary runs by 80% (only trigger on code/dependency changes)

**Example Output**:
```bash
$ gh run list --workflow=test.yml --limit 3
✓  chore: update docs    test  main  1234567  3m 42s ago
-  feat: add feature     test  pr-42 1234566  5m 12s ago  (skipped - docs only)
✓  fix: bug fix          test  main  1234565  1h 23m ago
```

---

### msrv.yml

**Purpose**: Verify the `maproom` crate compiles with Rust 1.85, the minimum supported Rust version (MSRV) declared in `crates/maproom/Cargo.toml`.

**Triggers**:
- `push` to `main` branch
- `pull_request`
- **Path filters**: Only runs when these files change:
  - Rust crate: `crates/maproom/**`, `Cargo.toml`, `Cargo.lock`

**Runner**: `blacksmith-4vcpu-ubuntu-2404`

**Key Features**:
- **Pinned toolchain**: Uses `dtolnay/rust-toolchain@1.85` — not `stable`
- **Build only**: Runs `cargo build -p maproom`; does not run tests (dev-dependencies may require a newer toolchain)
- **Rust cache**: `Swatinem/rust-cache@v2` with `shared-key: msrv` for faster builds

---

### reusable-rust-build.yml

**Purpose**: Reusable workflow for building Rust binaries across all platforms

**Type**: `workflow_call` (reusable)

**Inputs**:
- `package_name` (required): Artifact prefix (e.g., `"cli"`, `"maproom-mcp"`)
- `crate_path` (optional): Path to Rust crate (default: `"crates/maproom"`)
- `binary_name` (optional): Binary to build (default: `"maproom"`)
- `platforms` (optional): JSON array of platforms (default: all 4)

**Outputs**:
- `artifact_prefix`: Name prefix for uploaded artifacts

**Platform Matrix**:
- **linux-x64**: x86_64-unknown-linux-gnu (cross-compilation)
- **linux-arm64**: aarch64-unknown-linux-gnu (cross-compilation)
- **darwin-x64**: x86_64-apple-darwin (native build on macos-13)
- **darwin-arm64**: aarch64-apple-darwin (native build on macos-latest)

**Duration**: 8-12 minutes (all 4 platforms in parallel)

**Key Features**:
- Rust caching (Swatinem/rust-cache@v2)
- Platform-specific binary stripping (ARM64 via Docker container)
- Binary validation (existence, file type, execution test for linux-x64)
- Artifacts uploaded per platform with 7-day retention

**Usage Example**:
```yaml
jobs:
  build-binaries:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: cli
      binary_name: maproom
      crate_path: crates/maproom
```

---

### reusable-typescript-build.yml

**Purpose**: Reusable workflow for building TypeScript workspace packages

**Type**: `workflow_call` (reusable)

**Inputs**:
- `workspace_filter` (optional): pnpm filter pattern (default: `"./packages/*"`)
- `artifact_name` (optional): Artifact name (default: `"typescript-dist"`)

**Outputs**:
- `artifact_name`: Name of uploaded artifact

**Duration**: 1-2 minutes (with cache hit)

**Key Features**:
- pnpm store caching (40-60% faster installs)
- Flexible workspace filtering (build all or specific packages)
- Automatic dependency resolution
- Dist artifact upload (excludes node_modules)
- Verification step ensures dist/ directories exist

**Usage Example**:
```yaml
jobs:
  build-typescript:
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/cli...'  # CLI + dependencies
      artifact_name: cli-dist

  use-artifacts:
    needs: build-typescript
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: cli-dist
```

**Workspace Filter Patterns**:
- `./packages/*` - All packages
- `@crewchief/cli...` - CLI + dependencies (daemon-client)
- `@crewchief/cli... @crewchief/maproom-mcp...` - Multiple packages
- `./packages/* !@crewchief/vscode-maproom` - All except one

---

### build-and-publish-cli.yml

**Purpose**: Build and publish `@crewchief/cli` to npm

**Triggers**:
- `push` with tag pattern: `@crewchief/cli@v*.*.*`
- `workflow_dispatch` (manual, with dry-run option)

**Duration**: 12-15 minutes (current), 6-8 minutes (future with reusables)

**Current Implementation**:
- Builds Rust binaries inline (duplicates reusable-rust-build logic)
- Uses Rust caching from Phase 1

**Future (Phase 3)**:
- Will call `reusable-rust-build.yml` and `reusable-typescript-build.yml`
- 50% faster due to eliminating duplicated logic

---

### build-and-publish-maproom-mcp.yml

**Purpose**: Build and publish `@crewchief/maproom-mcp` to npm

**Triggers**:
- `push` with tag pattern: `@crewchief/maproom-mcp@v*.*.*`
- `workflow_dispatch` (manual, with dry-run option)

**Duration**: 25-30 minutes (current), 8-10 minutes (future with reusables)

**Current Implementation**:
- Builds Rust binaries inline (duplicates reusable-rust-build logic)
- Uses pnpm for package management (converted from npm in Phase 1)
- Uses Rust caching and pnpm caching from Phase 1

**Future (Phase 3)**:
- Will call `reusable-rust-build.yml` and `reusable-typescript-build.yml`
- 67% faster due to eliminating duplicated logic

---

### publish-maproom-mcp-image.yml

**Purpose**: Build and publish multi-platform Docker images for Maproom MCP

**Triggers**:
- `push` with tag pattern: `@crewchief/maproom-mcp@v*.*.*`
- `workflow_dispatch` (manual, with push toggle)

**Platforms**:
- linux/amd64
- linux/arm64

**Duration**: 15-20 minutes (multi-platform build)

**Key Features**:
- pnpm store caching
- TypeScript package builds (daemon-client dependency)
- QEMU for ARM64 emulation
- Docker layer caching (GitHub Actions cache)
- Trivy security scanning
- Multi-tag strategy (latest, major, minor, full version)

**Registry**:
- Docker Hub: `manifoldlogic/crewchief_maproom-mcp`

---

## Reusable Workflows

### What are Reusable Workflows?

Reusable workflows are GitHub Actions workflows that can be called from other workflows using the `workflow_call` trigger. They eliminate duplication, ensure consistency, and simplify maintenance.

**Benefits**:
- **Single source of truth**: Update once, all callers benefit
- **Consistency**: Same logic across all packages
- **Testable**: Can test reusables in isolation before production use
- **Composable**: Mix and match reusables for different release scenarios

### How to Call a Reusable Workflow

**Basic Pattern**:
```yaml
jobs:
  call-reusable:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: my-package
      binary_name: my-binary
```

**With Outputs**:
```yaml
jobs:
  build:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: cli

  download-artifacts:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          # Output from reusable workflow
          name: ${{ needs.build.outputs.artifact_prefix }}-linux-x64
```

**Multiple Reusables**:
```yaml
jobs:
  build-rust:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: maproom-mcp

  build-typescript:
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/maproom-mcp...'

  publish:
    needs: [build-rust, build-typescript]
    runs-on: ubuntu-latest
    steps:
      - name: Download Rust binaries
        uses: actions/download-artifact@v4
        with:
          path: binaries/

      - name: Download TypeScript dist
        uses: actions/download-artifact@v4
        with:
          name: ${{ needs.build-typescript.outputs.artifact_name }}
```

### Required vs Optional Inputs

**Rust Build Workflow**:
- **Required**: `package_name` (string)
- **Optional**:
  - `crate_path` (default: `"crates/maproom"`)
  - `binary_name` (default: `"maproom"`)
  - `platforms` (default: all 4 platforms)

**TypeScript Build Workflow**:
- **Optional** (all have defaults):
  - `workspace_filter` (default: `"./packages/*"`)
  - `artifact_name` (default: `"typescript-dist"`)

**Best Practice**: Always provide `package_name` and `binary_name` explicitly for clarity, even when using defaults.

---

## Testing Procedures

### Testing Reusable Workflows

**Step 1: Use Test Caller Workflows**

Test reusable workflows in isolation before integrating them into production:

```bash
# Test Rust build workflow
gh workflow run test-reusable-rust.yml

# Test TypeScript build workflow
gh workflow run test-reusable-typescript.yml

# Monitor execution
gh run watch
```

**Step 2: Verify Artifacts**

Download and inspect artifacts to ensure correct structure:

```bash
# Get latest run ID
RUN_ID=$(gh run list --workflow=test-reusable-rust.yml --limit 1 --json databaseId --jq '.[0].databaseId')

# Download all artifacts
gh run download $RUN_ID

# Verify structure
ls -la test-cli-*/
# Expected: test-cli-linux-x64/maproom, etc.

# Verify binaries
file test-cli-*/maproom
# Expected: "executable" and "stripped" for each platform
```

**Step 3: Validate Cache Behavior**

Test cache hit performance:

```bash
# First run (cache miss)
gh workflow run test-reusable-typescript.yml
# Check logs: "Cache not found" → ~60 sec install

# Second run (cache hit)
gh workflow run test-reusable-typescript.yml
# Check logs: "Cache restored" → ~10-15 sec install
```

### Testing on Feature Branch

**Safe Testing Pattern**:

```bash
# Create feature branch
git checkout -b ci-test-changes

# Make workflow changes
vim .github/workflows/test.yml

# Commit and push
git add .github/workflows/
git commit -m "test(ci): experiment with path filters"
git push origin ci-test-changes

# Trigger workflow on feature branch
gh workflow run test.yml --ref ci-test-changes

# Monitor
gh run watch

# If successful, merge to main
# If failed, iterate on feature branch
```

**Key Points**:
- Always test on feature branch first
- Use `workflow_dispatch` for manual testing
- Verify cache behavior with multiple runs
- Check artifact contents, not just existence

### Dry-Run Release Testing

**Test Release Workflows Without Publishing**:

```bash
# Dry-run CLI release
gh workflow run build-and-publish-cli.yml \
  --ref feature-branch \
  --field version=0.0.0-test \
  --field dry_run=true

# Dry-run Maproom-MCP npm release
gh workflow run build-and-publish-maproom-mcp.yml \
  --ref feature-branch \
  --field version=0.0.0-test \
  --field dry_run=true

# Dry-run Docker image build (without push)
gh workflow run publish-maproom-mcp-image.yml \
  --ref feature-branch \
  --field version=0.0.0-test \
  --field push_to_registry=false
```

**Expected Behavior**:
- All build steps execute normally
- Validation and verification complete
- Artifacts created successfully
- **Publish steps skipped** (dry-run or push_to_registry=false)

---

## Common Tasks

### Adding a New Platform to Rust Build Matrix

**Scenario**: Add Windows x64 support to Rust builds

**Steps**:

1. **Update reusable-rust-build.yml** (`.github/workflows/reusable-rust-build.yml`):

```yaml
# Add to matrix.config array (around line 47)
matrix:
  config:
    # ... existing platforms ...
    - platform: win32-x64
      target: x86_64-pc-windows-msvc
      os: windows-latest
      use_cross: false
```

2. **Update default platforms input** (line 29):

```yaml
platforms:
  description: 'JSON array of platforms to build'
  required: false
  type: string
  default: '["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"]'
```

3. **Test on feature branch**:

```bash
git checkout -b ci-add-windows-platform
git add .github/workflows/reusable-rust-build.yml
git commit -m "feat(ci): add Windows x64 platform to Rust build"
git push origin ci-add-windows-platform

# Test
gh workflow run test-reusable-rust.yml --ref ci-add-windows-platform
gh run watch
```

4. **Verify artifact**:

```bash
# Download and check
RUN_ID=$(gh run list --workflow=test-reusable-rust.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run download $RUN_ID

# Verify Windows binary
file test-cli-win32-x64/maproom.exe
# Expected: PE32+ executable (Windows)
```

---

### Modifying pnpm Workspace Filter Patterns

**Scenario**: Build only maproom-mcp package (excluding CLI)

**Pattern**: `'@crewchief/maproom-mcp...'`

**Usage in Workflow**:

```yaml
jobs:
  build-maproom-only:
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/maproom-mcp...'
      artifact_name: maproom-mcp-dist
```

**Common Patterns**:

| Pattern | Builds | Use Case |
|---------|--------|----------|
| `./packages/*` | All workspace packages | Full monorepo build |
| `@crewchief/cli...` | CLI + daemon-client | CLI release |
| `@crewchief/maproom-mcp...` | Maproom-MCP + daemon-client | Maproom release |
| `./packages/* !@crewchief/vscode-maproom` | All except VSCode | Exclude WIP package |

**Testing Filter Changes**:

```bash
# Test specific filter
gh workflow run test-reusable-typescript.yml

# Check which packages were built
RUN_ID=$(gh run list --workflow=test-reusable-typescript.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run view $RUN_ID --log | grep "Building"
# Expected output shows which packages pnpm built
```

---

### Updating Node.js Version Across Workflows

**Scenario**: Upgrade from Node.js 20 to Node.js 22

**Files to Update**:
1. `test.yml`
2. `reusable-typescript-build.yml`
3. `build-and-publish-cli.yml`
4. `build-and-publish-maproom-mcp.yml`
5. `publish-maproom-mcp-image.yml`

**Find and Replace**:

```bash
# Find all Node.js version references
grep -r "node-version: '20'" .github/workflows/

# Update using sed (or manual edit)
find .github/workflows -name "*.yml" -exec sed -i "s/node-version: '20'/node-version: '22'/g" {} \;

# Verify changes
git diff .github/workflows/
```

**Test After Update**:

```bash
git checkout -b ci-upgrade-node-22
git add .github/workflows/
git commit -m "chore(ci): upgrade Node.js to v22"
git push origin ci-upgrade-node-22

# Test each workflow type
gh workflow run test.yml --ref ci-upgrade-node-22
gh workflow run test-reusable-typescript.yml --ref ci-upgrade-node-22

# Monitor for issues
gh run watch
```

**Common Issues**:
- Check for Node.js 22 compatibility in dependencies
- Verify pnpm still works (pnpm 9+ required for Node 22)
- Watch for deprecated API warnings in logs

---

### Changing Artifact Retention Periods

**Scenario**: Reduce artifact retention from 7 days to 3 days to save storage

**Default Retention**: 7 days (specified in reusable workflows)

**Update reusable-rust-build.yml** (line 134):

```yaml
- name: Upload artifact
  uses: actions/upload-artifact@v4
  with:
    name: ${{ inputs.package_name }}-${{ matrix.config.platform }}
    path: target/${{ matrix.config.target }}/release/${{ inputs.binary_name }}
    if-no-files-found: error
    retention-days: 3  # Changed from 7
```

**Update reusable-typescript-build.yml** (line 81):

```yaml
- name: Upload dist artifacts
  uses: actions/upload-artifact@v4
  with:
    name: ${{ inputs.artifact_name }}
    path: |
      packages/*/dist/
      !packages/*/node_modules/
    if-no-files-found: error
    retention-days: 3  # Changed from 7
```

**Considerations**:
- **Release workflows**: Keep 7+ days for debugging failed releases
- **Test workflows**: 3 days is often sufficient
- **PR artifacts**: 1 day may be enough (quick feedback only)
- **Storage limits**: GitHub free tier includes 500MB, paid plans scale

---

## Troubleshooting Guide

### Cache Miss Issues

**Symptom**: "Cache not found for key: ..." in workflow logs, slow install times

**Common Causes**:

1. **Lock file changed**
   - **Detection**: Lock file hash in cache key changed
   - **Expected**: First run after dependency changes always misses cache
   - **Resolution**: Normal behavior, cache will be saved for next run

2. **Cache evicted by GitHub**
   - **Detection**: Cache was valid but no longer exists
   - **Cause**: GitHub evicts least-recently-used caches after 7 days or when repo exceeds 10GB total cache
   - **Resolution**: Normal behavior, cache will be recreated

3. **Cache key mismatch**
   - **Detection**: Cache key format changed in workflow update
   - **Example**: Changed from `pnpm-store-${{ hashFiles(...) }}` to `${{ runner.os }}-pnpm-store-${{ hashFiles(...) }}`
   - **Resolution**: Old caches are orphaned, new caches will be created

**Verification**:

```bash
# List current caches
gh cache list --limit 20

# Check cache size
gh cache list --json | jq '[.[].sizeInBytes] | add'

# Expected: 500MB-2GB for typical project
```

**Manual Cache Clearing** (last resort):

```bash
# List caches
gh cache list

# Delete specific cache
gh cache delete <cache-id>

# Delete all caches (USE WITH CAUTION)
for cache_id in $(gh cache list --json | jq -r '.[].id'); do
  gh cache delete $cache_id
done
```

---

### Artifact Download Failures

**Symptom**: "Artifact not found" or "Download failed" errors

**Common Causes**:

1. **Artifact name mismatch**
   - **Detection**: Download specifies different name than upload
   - **Example**:
     ```yaml
     # Upload
     name: typescript-dist

     # Download (WRONG)
     name: ts-dist  # Mismatch!
     ```
   - **Resolution**: Use exact artifact name from upload step

2. **Artifact retention expired**
   - **Detection**: Workflow run is older than retention period (default 7 days)
   - **Resolution**: Re-run workflow to regenerate artifacts

3. **Upstream job failed**
   - **Detection**: Job that uploads artifact failed, but downstream job still tried to download
   - **Resolution**: Add `needs:` dependency and check `if: success()` condition
   - **Fix**:
     ```yaml
     download-job:
       needs: upload-job
       if: success()  # Only run if upstream succeeded
     ```

4. **Parallel downloads with same name**
   - **Detection**: Multiple jobs download to same path simultaneously
   - **Resolution**: Use unique paths for each download
   - **Fix**:
     ```yaml
     - uses: actions/download-artifact@v4
       with:
         name: my-artifact
         path: artifacts/job-${{ github.job }}/  # Unique per job
     ```

**Verification**:

```bash
# List artifacts for a specific run
gh run view <run-id> --log | grep "Uploading artifact"

# Check artifact size and retention
gh api repos/:owner/:repo/actions/runs/<run-id>/artifacts
```

---

### Binary Validation Errors

**Symptom**: "Binary too small", "Binary failed to execute", or "File type mismatch"

**Common Causes**:

1. **Incomplete build**
   - **Detection**: Binary size < 1MB (suspiciously small)
   - **Cause**: Build failed partway through but didn't exit with error
   - **Resolution**: Check build logs for warnings/errors, ensure `set -e` in scripts

2. **Platform mismatch**
   - **Detection**: Trying to execute ARM64 binary on x64 runner
   - **Example**: Execution test runs for all platforms (should only run for linux-x64)
   - **Resolution**: Add platform condition to execution test:
     ```yaml
     if: matrix.config.platform == 'linux-x64'
     ```

3. **Stripping failed**
   - **Detection**: Binary works but validation fails on file type check
   - **Cause**: `strip` command failed silently
   - **Resolution**: Check strip step logs, verify Docker container availability for ARM64

4. **Binary not executable**
   - **Detection**: "Permission denied" when executing binary
   - **Cause**: File permissions not set correctly in artifact
   - **Resolution**: Add `chmod +x` before upload:
     ```yaml
     - name: Make binary executable
       run: chmod +x target/release/my-binary
     ```

**Debug Steps**:

```bash
# Download artifact
gh run download <run-id>

# Check file type
file my-artifact/my-binary
# Expected: "ELF 64-bit LSB executable, x86-64, dynamically linked, stripped"

# Check size
ls -lh my-artifact/my-binary
# Expected: 5-20MB for typical Rust binary

# Check permissions
ls -l my-artifact/my-binary
# Expected: -rwxr-xr-x (executable bits set)

# Test execution (if platform matches)
./my-artifact/my-binary --version
```

---

### Platform-Specific Build Failures

**Symptom**: Build succeeds on some platforms but fails on others

**Common Causes**:

1. **Cross-compilation toolchain missing**
   - **Platform**: linux-arm64, linux-x64 (when using cross)
   - **Detection**: "cross: command not found" or "linker not found"
   - **Cause**: `cross` tool not installed or wrong version
   - **Resolution**: Verify `cross` installation step runs before build:
     ```yaml
     - name: Install cross
       if: matrix.config.use_cross
       run: cargo install cross --git https://github.com/cross-rs/cross
     ```

2. **macOS runner version mismatch**
   - **Platform**: darwin-x64, darwin-arm64
   - **Detection**: Build works locally but fails in CI
   - **Cause**: Using wrong macOS runner (macos-latest vs macos-13)
   - **Resolution**: Use correct runner for target:
     - darwin-x64: `macos-13` (Intel)
     - darwin-arm64: `macos-latest` (Apple Silicon)

3. **Docker unavailable for ARM64 stripping**
   - **Platform**: linux-arm64
   - **Detection**: "docker: command not found" during strip step
   - **Cause**: Docker not available on runner (rare)
   - **Resolution**: Check runner has Docker, verify container image accessible:
     ```bash
     docker pull ghcr.io/cross-rs/aarch64-unknown-linux-gnu:latest
     ```

4. **Rust target not installed**
   - **Platform**: Any
   - **Detection**: "can't find crate for `std`" or "target not found"
   - **Cause**: Rust toolchain doesn't have target installed
   - **Resolution**: Verify target installation in setup step:
     ```yaml
     - uses: dtolnay/rust-toolchain@stable
       with:
         targets: ${{ matrix.config.target }}
     ```

**Platform-Specific Debug**:

```bash
# Check available Rust targets
rustup target list --installed

# Install missing target
rustup target add aarch64-unknown-linux-gnu

# Test cross-compilation locally
cross build --target aarch64-unknown-linux-gnu --release

# Verify cross version
cross --version
# Expected: cross 0.2.x or newer
```

---

### Path Filter Not Triggering

**Symptom**: Workflow doesn't run when expected, or runs when it shouldn't

**Common Causes**:

1. **Path pattern doesn't match**
   - **Detection**: Changed file path not covered by any filter pattern
   - **Example**: Changed `packages/cli/CLAUDE.md` but pattern is `**.ts`
   - **Resolution**: Add broader pattern or specific path:
     ```yaml
     paths:
       - '**.ts'
       - '**.md'  # Now covers CLAUDE.md files
     ```

2. **Workflow file itself excluded**
   - **Detection**: Changed `test.yml` but workflow doesn't run to validate change
   - **Cause**: Workflow file not in `paths:` filter
   - **Resolution**: Always include workflow file itself:
     ```yaml
     paths:
       - '.github/workflows/test.yml'  # Self-trigger
     ```

3. **Reusable workflow changes not triggering**
   - **Detection**: Changed `reusable-rust-build.yml` but test workflow doesn't run
   - **Cause**: Reusable workflows not in `paths:` filter
   - **Resolution**: Include all workflow dependencies:
     ```yaml
     paths:
       - '.github/workflows/test.yml'
       - '.github/workflows/reusable-rust-build.yml'
       - '.github/workflows/reusable-typescript-build.yml'
     ```

4. **Push vs PR filter mismatch**
   - **Detection**: Workflow runs on PR but not on push (or vice versa)
   - **Cause**: Different `paths:` arrays for `push:` and `pull_request:`
   - **Resolution**: Keep both arrays identical:
     ```yaml
     on:
       push:
         paths: [...]  # Same list
       pull_request:
         paths: [...]  # Same list (keep in sync!)
     ```

**Testing Path Filters**:

```bash
# Create test PR with specific file changes
git checkout -b test-path-filters
echo "# Test" >> docs/README.md
git add docs/README.md
git commit -m "test: docs change"
git push origin test-path-filters
gh pr create --title "Test path filters"

# Check if workflow ran
gh pr checks
# Expected: Workflow skipped or not listed

# Now change a code file
echo "// Test" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: code change"
git push

# Check again
gh pr checks --watch
# Expected: Workflow running
```

---

## Rollback Procedures

### Quick Rollback (Git Revert)

**Time Estimate**: < 2 minutes

**When to Use**: Workflow changes broke CI, need immediate fix

**Steps**:

```bash
# 1. Identify breaking commit
git log --oneline .github/workflows/ | head -5

# 2. Revert the commit (creates new commit that undoes changes)
git revert <commit-sha>

# 3. Push revert commit
git push origin main

# 4. Verify workflows working
gh run list --limit 5
gh run watch  # Monitor next workflow run
```

**Example**:
```bash
$ git log --oneline .github/workflows/test.yml | head -3
abc1234 feat(ci): add new path filter pattern
def5678 fix(ci): update pnpm caching
ghi9012 chore(ci): bump Node.js to v22

# Breaking change was abc1234
$ git revert abc1234
[main xyz7890] Revert "feat(ci): add new path filter pattern"

$ git push origin main
```

---

### Emergency Rollback (Restore Backup)

**Time Estimate**: < 2 minutes

**When to Use**: Multiple related commits need reverting, or revert fails

**Prerequisites**: Backup workflow files with `.old` extension before making changes

**Steps**:

```bash
# 1. List backup files
ls -la .github/workflows/*.old

# 2. Restore from backup
cp .github/workflows/test.yml.old .github/workflows/test.yml

# 3. Commit restoration
git add .github/workflows/test.yml
git commit -m "fix(ci): emergency rollback to working test workflow"
git push origin main

# 4. Verify
gh run watch
```

**Creating Backups** (before making changes):

```bash
# Before modifying test.yml
cp .github/workflows/test.yml .github/workflows/test.yml.old

# Verify backup created
ls -la .github/workflows/test.yml*
# Shows: test.yml and test.yml.old
```

---

### Clear Corrupted Cache

**Time Estimate**: < 1 minute + cache rebuild time

**When to Use**: Cache corruption suspected (rare), inconsistent build failures

**WARNING**: This will force cache rebuild on next run (slower)

**Steps**:

```bash
# 1. List all caches
gh cache list

# 2. Identify problematic cache (e.g., pnpm-store cache)
gh cache list | grep pnpm-store

# 3. Delete specific cache
gh cache delete <cache-id>

# Or delete all caches (CAUTION)
gh cache list --json | jq -r '.[].id' | xargs -I {} gh cache delete {}

# 4. Trigger workflow to rebuild cache
gh workflow run test.yml

# 5. Verify cache recreation
gh run watch
gh cache list  # Should see new cache after run completes
```

**Expected Impact**:
- Next workflow run will be slower (cache miss)
- Subsequent runs will be fast again (cache hit)
- No code changes needed

---

### Complete Emergency Rollback

**Time Estimate**: < 5 minutes total

**When to Use**: Multiple workflows broken, production releases blocked

**Steps** (execute in order):

```bash
# 1. Revert all workflow changes from last deploy
git log --oneline .github/workflows/ | head -10
# Identify all commits since last known-good state

# 2. Create emergency rollback branch
git checkout -b emergency-rollback-$(date +%Y%m%d-%H%M)

# 3. Revert commits (most recent first)
git revert abc1234 def5678 ghi9012

# OR restore from backups if available
cp .github/workflows/*.old .github/workflows/
git add .github/workflows/
git commit -m "fix(ci): emergency rollback all workflows"

# 4. Push and merge immediately
git push origin emergency-rollback-$(date +%Y%m%d-%H%M)
gh pr create --title "Emergency CI Rollback" --body "Restoring last working state"
gh pr merge --merge  # Skip checks for emergency

# 5. Clear all caches (force fresh state)
gh cache list --json | jq -r '.[].id' | xargs -I {} gh cache delete {}

# 6. Trigger test workflow to verify
gh workflow run test.yml --ref main
gh run watch

# 7. Monitor for success
gh run list --limit 10
```

**Post-Rollback**:
- Document what broke (for root cause analysis)
- Test fixes on feature branch before re-applying
- Consider adding validation step to prevent recurrence

---

## Metrics & Monitoring

### Checking Workflow Duration Trends

**View Recent Runs**:

```bash
# List last 10 runs of test workflow
gh run list --workflow=test.yml --limit 10

# Show duration for each run
gh run list --workflow=test.yml --limit 10 --json durationMs,conclusion,createdAt \
  | jq -r '.[] | "\(.createdAt) - \(.conclusion) - \(.durationMs / 60000 | floor)m \(.durationMs % 60000 / 1000 | floor)s"'
```

**Example Output**:
```
2025-01-15T10:23:45Z - success - 3m 42s
2025-01-15T09:15:22Z - success - 4m 18s
2025-01-14T16:45:10Z - failure - 2m 5s  (failed early)
2025-01-14T15:30:00Z - success - 3m 55s
```

**Baseline Comparison**:

```bash
# Before optimization (Phase 0)
# Average: 5-8 minutes

# After Phase 1 (caching + path filters)
# Average: 3-5 minutes
# Improvement: 40% faster

# Expected after Phase 3 (consolidation)
# Average: 2-3 minutes
# Improvement: 60% faster
```

---

### Measuring Cache Hit Rates

**Check Cache Status in Logs**:

```bash
# Get latest test run
RUN_ID=$(gh run list --workflow=test.yml --limit 1 --json databaseId --jq '.[0].databaseId')

# Search logs for cache hits/misses
gh run view $RUN_ID --log | grep -i "cache"
```

**Expected Output (Cache Hit)**:
```
Cache restored from key: Linux-pnpm-store-abc123...
Restored 714 packages from cache (0.5s)
```

**Expected Output (Cache Miss)**:
```
Cache not found for key: Linux-pnpm-store-xyz789...
Installing dependencies (45.2s)
Cache saved successfully
```

**Calculate Hit Rate**:

```bash
# Count cache hits in last 20 runs
HITS=$(gh run list --workflow=test.yml --limit 20 --json databaseId --jq '.[].databaseId' | \
  xargs -I {} sh -c 'gh run view {} --log | grep -q "Cache restored" && echo 1' | wc -l)

echo "Cache hit rate: $((HITS * 100 / 20))%"
# Target: 80%+
```

---

### Performance Improvement Verification

**Before/After Comparison**:

```bash
# Measure average duration before optimization
gh run list --workflow=test.yml --created="<2025-01-01" --limit 50 --json durationMs \
  | jq '[.[].durationMs] | add / length / 60000'
# Example output: 6.5 (minutes)

# Measure average duration after optimization
gh run list --workflow=test.yml --created=">2025-01-01" --limit 50 --json durationMs \
  | jq '[.[].durationMs] | add / length / 60000'
# Example output: 3.8 (minutes)

# Calculate improvement
# (6.5 - 3.8) / 6.5 * 100 = 41.5% faster
```

**Workflow-Specific Metrics**:

| Workflow | Before | After Phase 1 | Target Phase 3 | Improvement |
|----------|--------|---------------|----------------|-------------|
| test.yml | 5-8 min | 3-5 min | 2-3 min | 40% → 60% |
| CLI release | 12-15 min | 12-15 min | 6-8 min | 0% → 50% |
| Maproom release | 25-30 min | 25-30 min | 8-10 min | 0% → 67% |

**Expected Trends**:
- **Cache hit rate**: 80%+ after Phase 1
- **Test workflow frequency**: 80% reduction (path filters)
- **Failed builds**: No change (quality maintained)
- **Release time**: 50-67% faster after Phase 3

---

## Best Practices

### When to Use workflow_dispatch

**Use Cases**:

1. **Testing workflow changes**
   ```yaml
   on:
     push:
       tags: ['@crewchief/cli@v*']
     workflow_dispatch:  # For testing before creating tag
       inputs:
         dry_run:
           description: 'Skip publish step'
           type: boolean
           default: true
   ```

2. **Manual releases** (with safety guard)
   ```yaml
   on:
     workflow_dispatch:
       inputs:
         version:
           description: 'Version to release (e.g., 1.2.3)'
           required: true
         confirm:
           description: 'Type "RELEASE" to confirm'
           required: true
   ```

3. **Debugging failed runs**
   ```yaml
   on:
     workflow_dispatch:
       inputs:
         debug:
           description: 'Enable debug logging'
           type: boolean
           default: false
   ```

**Anti-Pattern**: Using `workflow_dispatch` as primary trigger for production releases (prefer tags)

---

### Safe Testing Pattern

**Recommended Workflow**:

1. **Feature branch** → Test changes in isolation
2. **Dry-run** → Execute workflow without side effects
3. **Manual trigger** → Verify behavior before merge
4. **Merge to main** → Apply changes to production
5. **Monitor** → Watch first production run closely

**Example**:

```bash
# 1. Create feature branch
git checkout -b ci-improve-caching

# 2. Make changes
vim .github/workflows/test.yml

# 3. Commit and push
git add .github/workflows/
git commit -m "feat(ci): improve cache key strategy"
git push origin ci-improve-caching

# 4. Test on feature branch
gh workflow run test.yml --ref ci-improve-caching
gh run watch

# 5. If successful, create PR
gh pr create --title "Improve CI caching" --fill

# 6. Merge after review
gh pr merge --squash

# 7. Monitor first main run
gh run list --branch main --limit 1
gh run watch
```

**Red Flags**:
- Committing workflow changes directly to main
- Skipping dry-run testing for releases
- Not monitoring first production run after workflow change

---

### Path Filter Maintenance

**Best Practices**:

1. **Keep push and pull_request in sync**
   ```yaml
   on:
     push:
       paths: &code_paths
         - 'crates/**'
         - '**.rs'
     pull_request:
       paths: *code_paths  # YAML anchor for DRY
   ```

2. **Include workflow file itself**
   ```yaml
   paths:
     - '.github/workflows/test.yml'  # Self-trigger
   ```

3. **Include reusable dependencies**
   ```yaml
   paths:
     - '.github/workflows/reusable-rust-build.yml'
     - '.github/workflows/reusable-typescript-build.yml'
   ```

4. **Test filter changes**
   ```bash
   # Docs-only change (should skip)
   echo "# Test" >> README.md
   git add README.md
   git commit -m "docs: update readme"
   git push
   gh run list --limit 1  # Should show "skipped" or no run

   # Code change (should run)
   echo "// Test" >> packages/cli/src/index.ts
   git add packages/cli/src/index.ts
   git commit -m "feat: add test code"
   git push
   gh run list --limit 1  # Should show "in_progress" or "completed"
   ```

**Review Quarterly**: Ensure path filters still match project structure

---

### Artifact Retention Considerations

**Retention Guidelines**:

| Workflow Type | Retention | Rationale |
|---------------|-----------|-----------|
| PR/Test | 1-3 days | Quick feedback, not needed long-term |
| Release | 7-14 days | Debugging failed releases, comparison |
| Production | 30-90 days | Compliance, rollback capability |

**Storage Limits**:
- **GitHub Free**: 500 MB total artifact storage
- **GitHub Pro**: 2 GB total artifact storage
- **GitHub Enterprise**: 50 GB+ total artifact storage

**Optimization**:
```yaml
# Short retention for frequent workflows
- name: Upload test results
  uses: actions/upload-artifact@v4
  with:
    name: test-results
    path: test-results/
    retention-days: 1  # Only need for immediate review

# Longer retention for releases
- name: Upload release binaries
  uses: actions/upload-artifact@v4
  with:
    name: binaries
    path: binaries/
    retention-days: 30  # Keep for rollback scenarios
```

**Cleanup Script** (if approaching storage limits):

```bash
# List artifacts sorted by size
gh api repos/:owner/:repo/actions/artifacts \
  | jq -r '.artifacts[] | "\(.size_in_bytes)\t\(.name)\t\(.id)"' \
  | sort -rn | head -20

# Delete specific large artifact
gh api repos/:owner/:repo/actions/artifacts/<artifact-id> -X DELETE

# Delete all expired artifacts (automatic, but can force)
gh api repos/:owner/:repo/actions/artifacts \
  | jq -r '.artifacts[] | select(.expired == true) | .id' \
  | xargs -I {} gh api repos/:owner/:repo/actions/artifacts/{} -X DELETE
```

---

## Additional Resources

### GitHub Actions Documentation

- **Reusable Workflows**: https://docs.github.com/en/actions/using-workflows/reusing-workflows
- **Workflow Syntax**: https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions
- **Caching Dependencies**: https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows
- **Artifacts**: https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts
- **Path Filters**: https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions#onpushpull_requestpull_request_targetpathspaths-ignore

### Package Management

- **pnpm Filtering**: https://pnpm.io/filtering
- **pnpm Workspaces**: https://pnpm.io/workspaces
- **pnpm CI**: https://pnpm.io/continuous-integration

### Rust Cross-Compilation

- **cross-rs/cross**: https://github.com/cross-rs/cross
- **Rust Toolchains**: https://rust-lang.github.io/rustup/cross-compilation.html
- **Rust Cache Action**: https://github.com/Swatinem/rust-cache

### Project Documentation

- **CI/CD Troubleshooting**: [/workspace/.github/CLAUDE.md](/workspace/.github/CLAUDE.md)
- **Architecture**: [/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/architecture.md](/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/architecture.md)
- **Quality Strategy**: [/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/quality-strategy.md](/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/quality-strategy.md)
- **Project Plan**: [/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/plan.md](/workspace/.crewchief/archive/projects/CICDOPT_ci-cd-workflow-optimization/planning/plan.md)

### GitHub CLI

- **gh Documentation**: https://cli.github.com/manual/
- **gh run**: https://cli.github.com/manual/gh_run
- **gh workflow**: https://cli.github.com/manual/gh_workflow
- **gh cache**: https://cli.github.com/manual/gh_cache

---

**Maintained by**: DevOps team
**Last major update**: Phase 2 - Reusable Infrastructure (2025-01)
**Next review**: Phase 3 completion or quarterly (whichever comes first)

**Questions or Issues**: Create issue in repository or contact DevOps team
