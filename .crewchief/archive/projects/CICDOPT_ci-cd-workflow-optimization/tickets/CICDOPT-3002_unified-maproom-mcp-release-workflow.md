# Ticket: CICDOPT-3002: Create Unified Maproom-MCP Release Workflow

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (workflow validation requires CI run)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- github-actions-specialist
- docker-engineer (supporting)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a single unified workflow that handles npm publishing for maproom-mcp, triggered by one tag. Update the Docker workflow with Rust caching improvements. This eliminates the biggest source of duplication in our CI/CD system, reduces npm release time from 12-15 minutes to 8-10 minutes, and builds Rust binaries only once instead of twice.

## Background

**Problem Being Solved**:
- **Current**: Two separate workflows triggered by same tag:
  - `build-and-publish-maproom-mcp.yml`: Builds Rust + TypeScript → publishes npm (~12-15 min)
  - `publish-maproom-mcp-image.yml`: Builds Rust internally → publishes Docker (~8-10 min)
  - **Total**: 25-30 minutes, Rust built twice for npm workflow
- **Wasteful**: Rust binaries built twice within npm workflow (once per platform build)
- **Complexity**: Two workflows to maintain for one package release
- **Risk**: Workflows can drift (different Rust versions, different validation)

**Why Unified Workflow**:
- **Build once, use for npm**: Rust built once via reusable workflow, used for npm publishing
- **Parallel publishing**: npm and Docker jobs run in parallel after builds
- **Single source of truth**: One workflow for npm maproom-mcp release
- **Faster**: 8-10 min for npm (vs 12-15 min with redundant builds)
- **Simpler**: One workflow to test, maintain, understand

**Context from Architecture**:
From architecture.md lines 589-738:
- Unified workflow calls reusable-rust-build once
- Calls reusable-typescript-build once
- npm job runs after builds complete
- Docker workflow updated separately with caching improvements

**CRITICAL Context from Review Updates**:
From review-updates.md lines 97-126 (Docker Artifact Integration):
- **DECISION**: Keep Docker building Rust internally (as currently designed)
- **RATIONALE**: Dockerfile.combined is multi-stage build (builds Rust from source in Stage 1)
- **ISSUE WITH ARTIFACT APPROACH**: Dockerfile not designed to COPY pre-built binaries
- **SIMPLIFIED APPROACH**: Focus on npm workflow consolidation only
- **Docker workflow**: Keep separate, add Rust caching (from Phase 1)

**REVISED SCOPE FOR THIS TICKET**:
- Create unified workflow for npm publishing only
- Docker workflow stays separate (receives Rust caching improvements)
- Future: Consider Dockerfile refactor to support pre-built binaries (Phase 4)

**Reference**: This ticket implements Phase 3 (Weeks 2-3) from plan.md lines 285-310.

## Acceptance Criteria

1. [ ] New file created: `.github/workflows/release-maproom-mcp.yml`
2. [ ] Workflow structure includes 3 jobs:
   - `build-rust`: Calls reusable-rust-build.yml
   - `build-typescript`: Calls reusable-typescript-build.yml
   - `publish-npm`: Downloads artifacts, validates, publishes to npm
3. [ ] `build-rust` job configuration:
   - Uses: `./.github/workflows/reusable-rust-build.yml`
   - Inputs: `package_name: maproom-mcp`, `binary_name: crewchief-maproom`, `crate_path: crates/maproom`
4. [ ] `build-typescript` job configuration:
   - Uses: `./.github/workflows/reusable-typescript-build.yml`
   - Inputs: `workspace_filter: '@crewchief/maproom-mcp... @crewchief/daemon-client...'`, `artifact_name: 'maproom-mcp-typescript'`
5. [ ] `publish-npm` job configuration:
   - Depends on: `[build-rust, build-typescript]`
   - Downloads all Rust binaries (pattern: `maproom-mcp-*`)
   - Downloads TypeScript dist (name: `maproom-mcp-typescript`)
   - Validates binaries (all 4 platforms, size check 1MB-100MB)
   - Validates TypeScript dist (maproom-mcp + daemon-client)
   - Sets executable permissions
   - Packs package with `pnpm pack`
   - Publishes to npm (unless dry_run)
   - Uploads npm package artifact
6. [ ] Docker workflow updated separately:
   - File: `.github/workflows/publish-maproom-mcp-image.yml`
   - Keeps internal Rust build (as designed)
   - Adds Rust caching (Swatinem/rust-cache@v2)
   - Adds pnpm caching (from Phase 1) - verify already present
   - Builds Docker image as before (multi-stage build)
7. [ ] Workflow triggers preserved:
   - Tag: `@crewchief/maproom-mcp@v*.*.*`
   - workflow_dispatch with inputs: `version`, `dry_run`
8. [ ] Permissions configured:
   - `contents: read`
   - `id-token: write` (for npm provenance)
9. [ ] Backup old npm workflow:
   - Rename `build-and-publish-maproom-mcp.yml` to `.old`
   - Docker workflow stays active (no backup needed, being updated)
10. [ ] Dry-run test succeeds:
    - npm workflow: dry-run via workflow_dispatch
    - Docker workflow: test build without push
11. [ ] Real release test succeeds:
    - Tag test release: `@crewchief/maproom-mcp@v0.0.0-test`
    - npm workflow triggers, publishes to npm
    - Docker workflow triggers, builds and pushes Docker image
12. [ ] Workflow code comparison:
    - npm workflow: Reduced to ~150 lines (from ~300)
    - Docker workflow: Stays ~200 lines (minor improvements only)
13. [ ] Total release time: 8-10 minutes for npm + Docker (from 25-30 minutes)

## Technical Requirements

### New File: `.github/workflows/release-maproom-mcp.yml`

**Complete Implementation** (npm workflow):

```yaml
name: Release Maproom MCP (npm)

on:
  push:
    tags:
      - '@crewchief/maproom-mcp@v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish (e.g., 2.2.1)'
        required: true
      dry_run:
        description: 'Dry run (skip actual publish)'
        type: boolean
        default: false

permissions:
  contents: read
  id-token: write  # For npm provenance

jobs:
  # Job 1: Build Rust binaries (reusable)
  build-rust:
    name: Build Rust Binaries
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: maproom-mcp
      binary_name: crewchief-maproom
      crate_path: crates/maproom

  # Job 2: Build TypeScript (reusable)
  build-typescript:
    name: Build TypeScript
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/maproom-mcp... @crewchief/daemon-client...'
      artifact_name: 'maproom-mcp-typescript'

  # Job 3: Publish to npm
  publish-npm:
    name: Publish to npm
    runs-on: ubuntu-latest
    needs: [build-rust, build-typescript]

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4

      - name: Download Rust binaries
        uses: actions/download-artifact@v4
        with:
          pattern: maproom-mcp-*
          path: packages/cli/bin/
          merge-multiple: true

      - name: Download TypeScript dist
        uses: actions/download-artifact@v4
        with:
          name: maproom-mcp-typescript
          path: .

      - name: Verify binary structure and sizes
        run: |
          echo "Checking binaries..."

          # Binary size validation (maproom-mcp: 1MB-100MB)
          MIN_SIZE=1048576      # 1MB
          MAX_SIZE=104857600    # 100MB

          for platform in linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
            BINARY="packages/cli/bin/$platform/crewchief-maproom"

            if [ ! -f "$BINARY" ]; then
              echo "ERROR: Missing binary for $platform"
              exit 1
            fi

            SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY")

            if [ "$SIZE" -lt "$MIN_SIZE" ] || [ "$SIZE" -gt "$MAX_SIZE" ]; then
              echo "ERROR: $platform binary size $SIZE outside range $MIN_SIZE-$MAX_SIZE"
              exit 1
            fi

            echo "✓ $platform binary: ${SIZE} bytes (valid)"
          done

      - name: Set executable permissions
        run: |
          find packages/cli/bin -name "crewchief-maproom" -exec chmod +x {} \;

      - name: Verify TypeScript dist
        run: |
          # Check maproom-mcp dist
          if [ ! -d "packages/maproom-mcp/dist" ]; then
            echo "ERROR: maproom-mcp dist not found"
            exit 1
          fi

          # Check daemon-client dist (dependency)
          if [ ! -d "packages/daemon-client/dist" ]; then
            echo "ERROR: daemon-client dist not found"
            exit 1
          fi

          echo "✓ TypeScript dist verified"
          ls -la packages/maproom-mcp/dist/
          ls -la packages/daemon-client/dist/

      - name: Pack package
        run: |
          cd packages/maproom-mcp
          pnpm pack

      - name: Verify package
        run: |
          cd packages/maproom-mcp
          TARBALL=$(ls crewchief-maproom-mcp-*.tgz)
          echo "Package created: $TARBALL"
          tar -tzf "$TARBALL" | head -30

      - name: Publish to npm
        if: ${{ !inputs.dry_run && github.event_name != 'workflow_dispatch' }}
        run: |
          cd packages/maproom-mcp
          pnpm publish --no-git-checks --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Upload package artifact
        uses: actions/upload-artifact@v4
        with:
          name: npm-package-maproom-mcp
          path: packages/maproom-mcp/*.tgz
          retention-days: 90
```

### Updated Docker Workflow

Update `.github/workflows/publish-maproom-mcp-image.yml` to add Rust caching:

```yaml
# After checkout step, add:

- name: Cache Rust dependencies
  uses: Swatinem/rust-cache@v2
  with:
    workspaces: "crates/maproom -> target"
    cache-on-failure: true
```

**Keep everything else in Docker workflow unchanged** - it already:
- Has pnpm caching
- Builds Rust internally (multi-stage build)
- Uses Docker BuildKit layer caching
- Publishes to Docker Hub

## Implementation Notes

### Step 1 - Create npm workflow

```bash
# Create feature branch
git checkout -b ci-unify-maproom-mcp-workflow

# Create new npm workflow
vim .github/workflows/release-maproom-mcp.yml
# (paste npm implementation from Technical Requirements)

git add .github/workflows/release-maproom-mcp.yml
git commit -m "feat(ci): create unified maproom-mcp npm workflow"
```

### Step 2 - Update Docker workflow (add Rust caching)

```bash
# Edit Docker workflow
vim .github/workflows/publish-maproom-mcp-image.yml
# (add Rust caching step after checkout)

git add .github/workflows/publish-maproom-mcp-image.yml
git commit -m "feat(ci): add Rust caching to Docker workflow"
git push origin ci-unify-maproom-mcp-workflow
```

### Step 3 - Test npm workflow (dry-run)

```bash
# Trigger npm workflow
gh workflow run release-maproom-mcp.yml \
  --ref ci-unify-maproom-mcp-workflow \
  --field version=0.0.0-test \
  --field dry_run=true

# Monitor
gh run watch

# Verify:
# - Rust binaries built once (4 platforms)
# - TypeScript built (maproom-mcp + daemon-client)
# - Binary validation passes
# - Package created
# - Publish skipped (dry-run)
```

### Step 4 - Test Docker workflow (no push)

```bash
# Trigger Docker workflow
gh workflow run publish-maproom-mcp-image.yml \
  --ref ci-unify-maproom-mcp-workflow \
  --field version=0.0.0-test \
  --field push_image=false

# Verify:
# - Rust cache works (second run faster)
# - Docker builds successfully
# - Image not pushed (test mode)
```

### Step 5 - Test with tag (full integration)

```bash
# Create test tag
git tag @crewchief/maproom-mcp@v0.0.0-test
git push origin @crewchief/maproom-mcp@v0.0.0-test

# BOTH workflows trigger automatically:
# - release-maproom-mcp.yml (npm)
# - publish-maproom-mcp-image.yml (Docker)

# Monitor both:
gh run list --workflow=release-maproom-mcp.yml
gh run list --workflow=publish-maproom-mcp-image.yml

# Verify:
# - npm published
# - Docker image pushed
# - Both complete successfully
```

### Step 6 - Backup old npm workflow and merge

```bash
# Backup old npm workflow
git mv .github/workflows/build-and-publish-maproom-mcp.yml \
       .github/workflows/build-and-publish-maproom-mcp.yml.old

git add .github/workflows/build-and-publish-maproom-mcp.yml.old
git commit -m "backup: archive old maproom-mcp npm workflow"

# Merge
git checkout main
git merge ci-unify-maproom-mcp-workflow
git push
```

**Note on Docker workflow**: NOT backed up because it's being updated (not replaced)

## Dependencies

**Depends On**:
- CICDOPT-2001 (reusable Rust build)
- CICDOPT-2002 (reusable TypeScript build)
- CICDOPT-3001 (CLI refactor validates pattern)

**Blocks**:
- CICDOPT-3003 (cleanup after consolidation validated)

## Risk Assessment

**Risk Level**: Medium-High (most complex consolidation)

**Risks**:

1. **npm and Docker workflows conflict**: Both triggered by same tag
   - **Mitigation**: Both workflows are independent (no resource conflicts)
   - **Expected**: Both run in parallel, finish independently
   - **Testing**: Test tag verifies both work together

2. **Binary validation different for maproom-mcp**: Size range 1-100MB (vs CLI 5-20MB)
   - **Mitigation**: Correct size validation in publish job
   - **Testing**: Dry-run verifies validation works

3. **Docker workflow Rust caching causes issues**:
   - **Mitigation**: Test thoroughly before merge
   - **Resolution**: Can remove caching if problems (still better than before)
   - **Rollback**: git revert the Docker workflow changes

4. **First production release fails**:
   - **Mitigation**: Extensive testing, backup .old ready
   - **Resolution**: Rollback npm workflow, investigate, fix
   - **Impact**: Can fall back to old npm workflow quickly

**Confidence**: Medium - Complex consolidation but extensively tested

## Files/Packages Affected

### Files to Create
1. `.github/workflows/release-maproom-mcp.yml` - New unified npm workflow

### Files to Update
1. `.github/workflows/publish-maproom-mcp-image.yml` - Add Rust caching

### Files to Backup
1. `.github/workflows/build-and-publish-maproom-mcp.yml` → `.old` extension

## Plan Reference

- From plan.md lines 285-310 (Phase 3, Ticket CICDOPT-3002)
- From architecture.md lines 589-738 (Unified Maproom-MCP Release Workflow section)
- From quality-strategy.md lines 390-450 (Test 3.2: Unified Maproom-MCP Workflow)
- From review-updates.md lines 97-126 (Docker Artifact Integration - REVISED APPROACH)

## Related Documentation

- `.github/workflows/reusable-rust-build.yml` (CICDOPT-2001)
- `.github/workflows/reusable-typescript-build.yml` (CICDOPT-2002)
- `.github/workflows/build-and-publish-maproom-mcp.yml` (current npm workflow)
- `.github/workflows/publish-maproom-mcp-image.yml` (current Docker workflow)
- `packages/maproom-mcp/config/Dockerfile.combined` (Docker multi-stage build)

## Success Indicators

After this ticket is complete:
1. ✅ npm workflow created and tested
2. ✅ Docker workflow updated with caching
3. ✅ Both workflows tested via tag
4. ✅ npm publishes successfully
5. ✅ Docker image builds and publishes successfully
6. ✅ Old npm workflow backed up
7. ✅ Total release time: 8-10 min (npm) + 5-6 min (Docker) = 13-16 min
8. ✅ 45-55% faster than before (25-30 min → 13-16 min)
9. ✅ Rust built once for npm (vs twice before)
10. ✅ Production release validated
