# Quality Strategy: CI/CD Workflow Optimization

## Testing Philosophy

**Goal**: Ship confidently without over-engineering

**Principles**:
1. **Validate before prod**: Test workflows before they affect real releases
2. **Fail fast**: Catch errors early in development
3. **Monitor metrics**: Track improvements and catch regressions
4. **Have rollback plan**: Can revert quickly if issues arise
5. **Test in isolation**: Use workflow_dispatch and branches for safe testing

## Testing Approach

### Phase 1: Quick Wins Testing

#### Test 1.1: Package.json Build Script Fix

**Critical Path**: This fix unblocks Docker workflow

**Testing**:
```bash
# Local testing
rm -rf node_modules packages/*/dist packages/*/node_modules
pnpm install
pnpm build  # Should succeed without circular dependency

# Verify all packages built
ls -la packages/cli/dist/cli/index.js
ls -la packages/daemon-client/dist/index.js
ls -la packages/maproom-mcp/dist/index.js
```

**Success Criteria**:
- ✅ Build completes without errors
- ✅ All expected dist/ files present
- ✅ No "MODULE_NOT_FOUND" errors

**Risk**: Low - aligns with pnpm best practices

---

#### Test 1.2: Rust Caching

**Critical Path**: Workflow must complete successfully with caching

**Testing Strategy**:

**Step 1 - Local Validation**:
```bash
# Validate YAML syntax
yamllint .github/workflows/build-and-publish-cli.yml
yamllint .github/workflows/build-and-publish-maproom-mcp.yml
```

**Step 2 - Branch Testing**:
```bash
# Create feature branch
git checkout -b ci-add-rust-caching

# Make changes to workflows
# Commit and push

# Trigger via workflow_dispatch (manual)
gh workflow run build-and-publish-cli.yml \
  --ref ci-add-rust-caching \
  --field version=0.0.0-test \
  --field dry_run=true
```

**Step 3 - Monitor First Run**:
```bash
# Watch workflow logs
gh run watch

# Look for:
# - "Cache not found" (expected first run)
# - Rust build completes
# - Artifacts uploaded successfully
```

**Step 4 - Verify Caching Works**:
```bash
# Trigger second run (same branch, no code changes)
gh workflow run build-and-publish-cli.yml \
  --ref ci-add-rust-caching \
  --field version=0.0.0-test2 \
  --field dry_run=true

# Look for:
# - "Cache restored from key: ..."
# - Build time 50-70% faster
# - "Saving cache..." at end
```

**Success Criteria**:
- ✅ First run: Cache miss, full build, cache saved
- ✅ Second run: Cache hit, fast build (<4 min vs 8-12 min)
- ✅ All 4 platforms build successfully
- ✅ Artifacts uploaded

**Metrics to Track**:
- Build time per platform (before/after)
- Cache hit rate percentage
- Total workflow duration

---

#### Test 1.3: pnpm Store Caching

**Testing Strategy**:

**Local Cache Simulation**:
```bash
# Clear pnpm cache
pnpm store prune

# Time installation without cache
time pnpm install --frozen-lockfile
# Note: ~60 seconds

# Second install (with local cache)
rm -rf node_modules packages/*/node_modules
time pnpm install --frozen-lockfile
# Note: Should be faster with local store
```

**CI Testing** (via workflow_dispatch):
```bash
# First run (cold cache)
gh workflow run test.yml --ref ci-add-pnpm-caching

# Monitor for:
# - "Cache not found for key: linux-pnpm-store-..."
# - pnpm install duration (~60 sec)
# - "Cache saved with key: ..."

# Second run (warm cache)
gh workflow run test.yml --ref ci-add-pnpm-caching

# Monitor for:
# - "Cache restored from key: ..."
# - pnpm install duration (~10-15 sec)
```

**Success Criteria**:
- ✅ Cache key based on lock file hash
- ✅ Cache hit reduces install time by 40-60%
- ✅ Cache works across workflows (test, release)

---

#### Test 1.4: Path Filters

**Testing Strategy**:

**Test Case 1 - Code Change (should trigger)**:
```bash
# Create branch with code change
git checkout -b test-path-filters
echo "// test" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: code change"
git push origin test-path-filters

# Create PR
gh pr create --title "Test: Code change" --body "Should trigger tests"

# Verify workflow runs
gh pr checks
# Expected: Test workflow running
```

**Test Case 2 - Docs Change (should skip)**:
```bash
# Edit markdown file
echo "# test" >> docs/README.md
git add docs/README.md
git commit -m "docs: update readme"
git push

# Create PR
gh pr create --title "Test: Docs change" --body "Should NOT trigger tests"

# Verify workflow skipped
gh pr checks
# Expected: Test workflow skipped (or very fast if already passed)
```

**Test Case 3 - Mixed Change (should trigger)**:
```bash
# Edit both code and docs
echo "// test" >> packages/cli/src/index.ts
echo "# test" >> docs/README.md
git add .
git commit -m "test: mixed change"
git push

# Verify workflow runs (code change takes precedence)
gh pr checks
# Expected: Test workflow running
```

**Success Criteria**:
- ✅ Code changes trigger tests
- ✅ Docs-only changes skip tests
- ✅ Mixed changes trigger tests
- ✅ Workflow file changes trigger tests

**Edge Cases to Test**:
- `.github/workflows/` changes (should skip except test.yml)
- `.agents/` changes (should skip)
- Migration file changes (should trigger)

---

### Phase 2: Reusable Workflow Testing

#### Test 2.1: Reusable Rust Build Workflow

**Isolation Testing**:

**Step 1 - Syntax Validation**:
```bash
yamllint .github/workflows/reusable-rust-build.yml

# Check for common mistakes:
# - workflow_call trigger defined
# - Required inputs specified
# - Outputs defined
# - Matrix configuration valid
```

**Step 2 - Create Test Caller**:
```yaml
# .github/workflows/test-reusable-rust.yml
name: Test Reusable Rust Build

on:
  workflow_dispatch:

jobs:
  test-build:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: test-cli
      binary_name: crewchief-maproom
      crate_path: crates/maproom
```

**Step 3 - Trigger and Monitor**:
```bash
gh workflow run test-reusable-rust.yml

# Monitor for:
# - All 4 platforms start
# - Rust caching works
# - Binaries built
# - Artifacts uploaded with correct names
```

**Step 4 - Verify Artifacts**:
```bash
# Download artifacts
gh run download <run-id>

# Verify:
ls -la test-cli-linux-x64/crewchief-maproom
ls -la test-cli-linux-arm64/crewchief-maproom
ls -la test-cli-darwin-x64/crewchief-maproom
ls -la test-cli-darwin-arm64/crewchief-maproom

# Check binaries are executable and stripped
file test-cli-darwin-x64/crewchief-maproom
# Expected: "Mach-O 64-bit executable ..." (stripped)
```

**Success Criteria**:
- ✅ Reusable workflow callable
- ✅ All platforms build
- ✅ Caching works per-platform
- ✅ Artifacts uploaded correctly
- ✅ Outputs accessible to caller

---

#### Test 2.2: Reusable TypeScript Build Workflow

**Testing Strategy**:

**Create Test Caller**:
```yaml
# .github/workflows/test-reusable-typescript.yml
name: Test Reusable TypeScript Build

on:
  workflow_dispatch:
    inputs:
      filter:
        description: 'Workspace filter'
        default: './packages/*'

jobs:
  test-build:
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: ${{ github.event.inputs.filter }}
      artifact_name: 'test-typescript-dist'
```

**Test Cases**:

**Case 1 - Build All Packages**:
```bash
gh workflow run test-reusable-typescript.yml \
  --field filter='./packages/*'

# Verify all packages built
gh run download <run-id>
ls -la test-typescript-dist/packages/*/dist/
```

**Case 2 - Build Specific Package**:
```bash
gh workflow run test-reusable-typescript.yml \
  --field filter='@crewchief/cli...'

# Verify only CLI and its dependencies built
```

**Success Criteria**:
- ✅ pnpm caching works
- ✅ Workspace filter respected
- ✅ All dist/ directories uploaded
- ✅ No node_modules in artifact

---

### Phase 3: Integration Testing

#### Test 3.1: Unified CLI Workflow

**End-to-End Test** (dry-run):

```bash
# Create test tag
git tag @crewchief/cli@v0.0.0-test
git push origin @crewchief/cli@v0.0.0-test

# Monitor workflow
gh run watch

# Verify sequence:
# 1. build-rust job completes (uses reusable)
# 2. build-typescript job completes (uses reusable)
# 3. publish job starts (depends on both)
# 4. Artifacts downloaded correctly
# 5. Validation passes
# 6. Dry-run skips actual publish
```

**Validation Checks**:
```bash
# Download workflow run artifacts
gh run download <run-id>

# Verify binaries present
for platform in linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
  test -f cli-$platform/crewchief-maproom || echo "Missing: $platform"
done

# Verify TypeScript dist
test -d cli-typescript/packages/cli/dist || echo "Missing: TypeScript dist"

# Verify package structure
test -f npm-package/crewchief-cli-*.tgz || echo "Missing: npm package"
```

**Success Criteria**:
- ✅ Reusable workflows called correctly
- ✅ Artifacts shared between jobs
- ✅ Binaries verified before publish
- ✅ Package created successfully
- ✅ Dry-run prevents actual publish

---

#### Test 3.2: Unified Maproom-MCP Workflow

**Testing Sequence**:

**Step 1 - Workflow Dispatch Test**:
```bash
gh workflow run release-maproom-mcp.yml \
  --field version=0.0.0-test \
  --field push_docker=false

# Monitor parallel execution:
# - build-rust (4 min)
# - build-typescript (2 min)
# - publish-npm starts after builds
# - publish-docker starts after builds (parallel with npm)
```

**Step 2 - Artifact Verification**:
```bash
# Download all artifacts
gh run download <run-id>

# Verify Rust binaries
ls -la maproom-mcp-*/crewchief-maproom

# Verify TypeScript
ls -la maproom-mcp-typescript/packages/maproom-mcp/dist/
ls -la maproom-mcp-typescript/packages/daemon-client/dist/

# Verify npm package
test -f npm-package/crewchief-maproom-mcp-*.tgz
```

**Step 3 - Docker Build Test** (local):
```bash
# Download artifacts locally
gh run download <run-id> --dir /tmp/ci-artifacts

# Copy to expected locations
cp /tmp/ci-artifacts/maproom-mcp-*/crewchief-maproom packages/cli/bin/

# Rebuild TypeScript locally
pnpm build

# Test Docker build uses artifacts
docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:test \
  --progress=plain \
  .

# Verify image builds
docker images | grep maproom-mcp:test
```

**Success Criteria**:
- ✅ Single workflow for npm + Docker
- ✅ Binaries built once, used twice
- ✅ TypeScript built once, used twice
- ✅ npm and Docker jobs run in parallel
- ✅ Docker build succeeds with pre-built artifacts

---

### Gradual Rollout Strategy

#### Phase 1 Rollout: Caching + Filters

**Week 1 - Deploy to Feature Branch**:
```bash
# Create feature branch
git checkout -b ci-optimization-phase1

# Make all Phase 1 changes
# Test via workflow_dispatch

# Monitor for 1 week:
# - Cache hit rates
# - Build times
# - Path filter effectiveness
```

**Week 2 - Merge to Main**:
```bash
# After successful testing, merge
git checkout main
git merge ci-optimization-phase1

# Monitor first few PRs for issues
# Verify caching works in production
```

**Metrics to Monitor**:
- Build time trend (should decrease)
- Test run frequency (should decrease 80%)
- Cache hit percentage (should be 80%+)

---

#### Phase 2 Rollout: Reusable Workflows

**Week 3 - Test Reusables in Isolation**:
```bash
# Add reusable workflows
# Test with dedicated test callers
# Do NOT update production workflows yet
```

**Week 4 - Integrate with CLI Workflow**:
```bash
# Update CLI workflow to use reusables
# Test with dry-run releases
# Monitor first real release closely
```

**Fallback Plan**:
```bash
# If reusables fail, can revert quickly
mv .github/workflows/release-cli.yml.old .github/workflows/release-cli.yml
git add .github/workflows/release-cli.yml
git commit -m "rollback: revert to old CLI workflow"
```

---

#### Phase 3 Rollout: Consolidation

**Week 5-6 - Unified Maproom-MCP Workflow**:
```bash
# Create new unified workflow
# Test extensively with workflow_dispatch
# Run dry-run releases

# When confident, tag for real release
git tag @crewchief/maproom-mcp@v2.2.2-test
git push origin @crewchief/maproom-mcp@v2.2.2-test
```

**Validation Checklist**:
- [ ] Workflow triggers on correct tag
- [ ] Rust binaries built once
- [ ] TypeScript built once
- [ ] npm package published successfully
- [ ] Docker image published successfully
- [ ] Both use same artifacts (verified via SHA)

**Delete Old Workflows** (only after success):
```bash
# Keep backups
mv .github/workflows/build-and-publish-maproom-mcp.yml \
   .github/workflows/build-and-publish-maproom-mcp.yml.old

mv .github/workflows/publish-maproom-mcp-image.yml \
   .github/workflows/publish-maproom-mcp-image.yml.old

# Commit
git add .github/workflows/*.old
git commit -m "backup: archive old maproom-mcp workflows"
```

---

## Regression Prevention

### Automated Checks

#### Workflow Linting (Pre-commit)

**Add to .github/workflows/lint.yml**:
```yaml
name: Lint Workflows

on: [push, pull_request]

jobs:
  yamllint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install yamllint
        run: pip install yamllint

      - name: Lint workflow files
        run: yamllint .github/workflows/

      - name: Validate workflow syntax
        uses: docker://rhysd/actionlint:latest
        with:
          args: -color
```

**Benefits**:
- Catch syntax errors before merge
- Validate workflow references
- Check for common mistakes

---

#### Cache Monitoring

**Add workflow to track cache effectiveness**:
```yaml
# .github/workflows/monitor-cache.yml
name: Monitor Cache Effectiveness

on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly
  workflow_dispatch:

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - name: Get cache stats
        uses: actions/github-script@v7
        with:
          script: |
            const caches = await github.rest.actions.getActionsCacheList({
              owner: context.repo.owner,
              repo: context.repo.repo
            });

            console.log('Cache Statistics:');
            console.log('Total caches:', caches.data.total_count);
            console.log('Total size:', caches.data.total_count > 0 ?
              caches.data.actions_caches[0].size_in_bytes : 0);

            // Log cache keys and sizes
            for (const cache of caches.data.actions_caches || []) {
              console.log(`- ${cache.key}: ${cache.size_in_bytes} bytes`);
            }
```

---

### Performance Regression Detection

**Metrics Dashboard** (GitHub Actions already tracks):
- Workflow duration trends
- Job duration breakdown
- Cache hit rates
- Artifact sizes

**Manual Checks** (weekly):
```bash
# Get recent workflow run times
gh run list --workflow=test.yml --limit 20 --json conclusion,durationMs

# Average duration
gh run list --workflow=test.yml --limit 20 --json durationMs \
  | jq '[.[].durationMs] | add / length'

# Compare to baseline (should be <5 min)
```

**Alert Thresholds**:
- Test workflow >6 min (was 3-5 min after optimization)
- Release workflow >10 min (was 6-8 min)
- Cache hit rate <70% (should be 80%+)

---

## Risk Mitigation

### Known Risks and Mitigations

#### Risk 1: Reusable Workflow Breaking Change

**Scenario**: Update reusable workflow breaks all callers

**Mitigation**:
- Version reusable workflows if API changes needed
- Test with dedicated caller before updating production
- Have rollback plan (git revert)

**Detection**:
- Workflow failures across multiple packages
- Same error in CLI and Maproom-MCP workflows

**Rollback**:
```bash
# Revert to previous version
git revert <commit-sha>
git push

# OR rename old workflow back
mv .github/workflows/reusable-rust-build.yml.old \
   .github/workflows/reusable-rust-build.yml
```

---

#### Risk 2: Cache Corruption

**Scenario**: Cached artifacts corrupt, cause builds to fail

**Mitigation**:
- Use `cache-on-failure: true` to not save bad caches
- Set `max-cache-size` to prevent bloat
- Document cache invalidation procedure

**Detection**:
- Builds fail with "corrupted archive" or similar
- Cache hit but build fails (cache miss succeeds)

**Resolution**:
```bash
# Clear specific cache
gh cache delete <cache-key>

# OR clear all caches (requires REST API)
gh api repos/:owner/:repo/actions/caches -X DELETE
```

---

#### Risk 3: Artifact Download Failure

**Scenario**: Job fails to download artifact from previous job

**Mitigation**:
- Set `if-no-files-found: error` on uploads
- Validate artifacts downloaded before using
- Keep artifact retention long enough (7 days minimum)

**Detection**:
- "artifact not found" errors
- Jobs skip due to missing dependencies

**Resolution**:
- Re-run failed workflow
- Check artifact expiration (90 days for releases)

---

#### Risk 4: Path Filter Too Restrictive

**Scenario**: Code change doesn't trigger tests due to overly strict paths

**Mitigation**:
- Include workflow file itself in paths (always triggers)
- Document path patterns clearly
- Review path changes carefully

**Detection**:
- Code merged without tests running
- PR shows "Skipped" for test workflow

**Resolution**:
```bash
# Fix path filter
vim .github/workflows/test.yml
# Add missing path pattern
git commit -m "fix: add missing path to test trigger"
```

---

## Success Metrics

### Primary Metrics

**Performance**:
- ✅ Test workflow: 5-8 min → 3-5 min (40% faster)
- ✅ Release workflow: 25-30 min → 8-10 min (67% faster)
- ✅ Docker build: Blocked → 5-6 min (unblocked)

**Efficiency**:
- ✅ Test run frequency: 100% → 20% of commits (80% reduction)
- ✅ Cache hit rate: <10% → 80%+ (8x improvement)
- ✅ Workflow code: 600 lines → 300 lines (50% reduction)

**Reliability**:
- ✅ Docker publish success rate: 0% → 100%
- ✅ Workflow failure rate: <5% (acceptable)
- ✅ Rollback time: <5 minutes (if needed)

### Secondary Metrics

**Developer Experience**:
- Time to feedback (PR created → tests complete)
- Confidence in releases (dry-run success rate)
- Documentation clarity (contributor questions)

**Cost**:
- CI minutes per release (106 min → ~40 min)
- Storage used by caches (<1GB expected)
- Maintenance time per workflow change

---

## Testing Checklist

### Pre-Merge Checklist

Before merging any workflow changes:

- [ ] yamllint passes
- [ ] Syntax validated with actionlint
- [ ] Tested via workflow_dispatch
- [ ] Dry-run successful (if release workflow)
- [ ] Cache behavior verified (miss → hit → hit)
- [ ] Artifacts downloaded successfully in dependent jobs
- [ ] Documentation updated (if API changed)
- [ ] Rollback plan documented

### Post-Merge Monitoring

After merging workflow changes:

- [ ] First PR triggers workflow correctly
- [ ] Path filters work as expected
- [ ] Cache created and reused
- [ ] Build times within expected range
- [ ] No errors in workflow logs
- [ ] Artifacts retained for expected duration

### Release Testing

Before tagging a release:

- [ ] workflow_dispatch dry-run successful
- [ ] All platforms build
- [ ] Artifacts validated
- [ ] npm pack succeeds
- [ ] Docker build succeeds (if applicable)
- [ ] All checks green

---

## Conclusion

This quality strategy ensures:
- ✅ Safe testing before production
- ✅ Gradual rollout minimizes risk
- ✅ Metrics track improvements
- ✅ Rollback plan for every change
- ✅ Confidence to ship aggressively

**Risk Level**: Low - comprehensive testing at every phase.

**Next**: Security review of workflow architecture.
