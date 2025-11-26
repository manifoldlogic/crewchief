# Quality Strategy: CI Workflow Fixes

## Testing Philosophy

**Core Principle**: Infrastructure changes require validation, not unit tests.

These are **configuration fixes**, not new features. Quality comes from:
1. **Verification in actual CI environment** (not mocked locally)
2. **Smoke testing** Docker images (run and validate basic functionality)
3. **Rollback readiness** (fast revert if something breaks)

**No new test files needed** - existing project tests will validate the fixes work.

## Test Strategy

### Test Workflow Fix

**What needs testing**:
- ✅ pnpm/action-setup@v4 detects version from package.json
- ✅ Correct pnpm version installed (10.12.1)
- ✅ Dependencies install without errors
- ✅ Existing test suite runs

**How to test**:

**Local Pre-flight** (optional, informational only):
```bash
# Simulate GitHub Actions environment
export CI=true
cat package.json | jq -r '.packageManager'
# Expected: pnpm@10.12.1+sha512...

# Verify packageManager field is valid
echo '{"packageManager": "pnpm@10.12.1+sha512..."}' | jq -r '.packageManager | split("@")[1] | split("+")[0]'
# Expected: 10.12.1
```

**Actual validation** (GitHub Actions):
1. Create PR with test.yml change
2. Push commit
3. Watch Actions tab for workflow run
4. Verify "Setup pnpm" step succeeds
5. Verify "Install dependencies" step succeeds
6. Verify "Run tests" step executes

**Success criteria**:
- ✅ No "Multiple versions of pnpm specified" error
- ✅ pnpm --version output matches package.json (10.12.1)
- ✅ Workflow completes (pass or fail on test logic, not infra)

**Failure scenarios and recovery**:

| Failure | Root Cause | Fix |
|---------|-----------|-----|
| Still shows version error | Cache issue | Re-run workflow |
| Wrong pnpm version | packageManager field malformed | Check JSON syntax |
| Can't find package.json | Checkout step failed | Check actions/checkout@v4 |
| Install fails | Lockfile corrupt | Run `pnpm install` locally, commit lockfile |

---

### Docker Build Fix

**What needs testing**:
- ✅ pnpm installs correctly in Alpine
- ✅ Workspace dependencies resolve
- ✅ Multi-platform builds succeed (amd64 + arm64)
- ✅ Final image size acceptable
- ✅ Container runs and MCP server responds

**How to test**:

**Local pre-flight** (required):
```bash
# Ensure daemon-client is built
cd /workspace
pnpm build

# Test Docker build locally (amd64 only for speed)
cd /workspace
docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:test \
  --platform linux/amd64 \
  .

# Check image size
docker images maproom-mcp:test
# Expected: ~220MB (within 10% of current)

# Smoke test: Run container
docker run --rm maproom-mcp:test node -e "console.log('OK')"
# Expected: "OK"

# Verify pnpm not in final image
docker run --rm maproom-mcp:test which pnpm || echo "Good: pnpm not in runtime"
# Expected: "Good: pnpm not in runtime"

# Test MCP server loads
docker run --rm maproom-mcp:test node dist/index.js --help || echo "Check dist/index.js path"
# Expected: Help text or indication server code loads
```

**CRITICAL Pre-flight Validation**:
```bash
# ⚠️ BLOCKER: Verify daemon-client dist/ exists
if [ ! -d "packages/daemon-client/dist" ]; then
  echo "❌ ERROR: daemon-client dist/ not found"
  echo "Run 'pnpm build' before 'docker build'"
  exit 1
fi

# Verify dist/ has expected files
EXPECTED_FILES=("index.js" "index.d.ts" "client.js" "client.d.ts")
for file in "${EXPECTED_FILES[@]}"; do
  if [ ! -f "packages/daemon-client/dist/$file" ]; then
    echo "❌ ERROR: Missing $file in daemon-client/dist/"
    echo "daemon-client build may have failed"
    exit 1
  fi
done

echo "✅ daemon-client dist/ validated"

# ⚠️ Verify pnpm version sync between package.json and Dockerfile
PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')

if [ "$PACKAGE_PNPM" != "$DOCKERFILE_PNPM" ]; then
  echo "❌ ERROR: pnpm version mismatch"
  echo "package.json: $PACKAGE_PNPM"
  echo "Dockerfile: $DOCKERFILE_PNPM"
  echo "Update Dockerfile line 41 to: RUN npm install -g pnpm@$PACKAGE_PNPM"
  exit 1
fi

echo "✅ pnpm versions synced ($PACKAGE_PNPM)"
```

**CI validation** (GitHub Actions):
1. Push changes to feature branch
2. Manually trigger release workflow (or create test tag)
3. Monitor "Build and push Docker image" step
4. Wait for multi-platform build (5-10 minutes)
5. Pull published image
6. Run smoke tests

**Success criteria**:
- ✅ Docker build completes without "EUNSUPPORTEDPROTOCOL" error
- ✅ Both amd64 and arm64 builds succeed
- ✅ Image size ~220MB (±10MB)
- ✅ Container starts without errors
- ✅ `node dist/index.js` runs (even if MCP fails without DB, that's expected)

**Failure scenarios and recovery**:

| Failure | Root Cause | Fix |
|---------|-----------|-----|
| **daemon-client/dist not found** | **`pnpm build` not run before Docker** | **Run `pnpm build` at repo root first (BLOCKER)** |
| pnpm install fails | Wrong pnpm version in Dockerfile | Match package.json version (use validation script) |
| workspace: not resolved | Missing pnpm-workspace.yaml COPY | Add to COPY layer |
| daemon-client package not found | Forgot to COPY package.json | Add daemon-client/package.json to COPY |
| TypeScript build errors | Source code out of sync | Fix TypeScript errors in source |
| arm64 build hangs (>15 min) | QEMU emulation slow/flaky | Wait longer OR disable arm64, release amd64 only |
| Image too large (>230MB) | Copied node_modules or pnpm store | Check .dockerignore excludes node_modules |
| pnpm version mismatch warning | Dockerfile/package.json out of sync | Run version validation script, update Dockerfile |

## Quality Gates

### Pre-Merge Requirements

**Test workflow PR**:
- [x] Code review approved (1 maintainer)
- [x] CI passes on PR branch
- [ ] Manual verification: Check workflow logs for pnpm version

**Docker build PR**:
- [x] Code review approved (1 maintainer)
- [x] Local Docker build succeeds
- [x] Image size verified (<230MB)
- [x] Smoke test passes (container runs)
- [ ] CI build passes (optional: can fix post-merge if urgent)

### Post-Merge Validation

**Test workflow**:
- [ ] Next push to main succeeds
- [ ] pnpm version correct in logs
- [ ] Tests execute fully

**Docker build**:
- [ ] Next release tag triggers successful build
- [ ] Docker Hub shows new images (amd64 + arm64)
- [ ] Developers can pull and run image
- [ ] MCP server functional in production

## Risk Mitigation

### Test Workflow Risks

**Risk: pnpm version mismatch**
- **Likelihood**: Low (action explicitly designed for auto-detection)
- **Impact**: Medium (CI blocked until fixed)
- **Mitigation**: Pre-verify packageManager field syntax
- **Rollback**: Revert commit, CI back to broken state (acceptable - already broken)

**Risk: Upstream action breaking change**
- **Likelihood**: Very Low (v4 stable since 2024)
- **Impact**: High (CI blocked)
- **Mitigation**: Pin action to @v4 (not @latest)
- **Rollback**: Revert to explicit version temporarily

### Docker Build Risks

**Risk: Multi-platform build failure**
- **Likelihood**: Medium (QEMU flakiness on arm64)
- **Impact**: High (can't release)
- **Mitigation**: Test both platforms in CI before tagging release
- **Rollback**: Revert Dockerfile, republish previous image

**Risk: Image size bloat**
- **Likelihood**: Low (pnpm only in builder stage)
- **Impact**: Medium (slower pulls, storage costs)
- **Mitigation**: Verify size in local build before pushing
- **Rollback**: Revert Dockerfile

**Risk: Runtime dependency missing**
- **Likelihood**: Low (production deps unchanged)
- **Impact**: High (MCP server won't start)
- **Mitigation**: Smoke test container startup in CI
- **Rollback**: Revert Dockerfile, republish previous image

### Shared Risks

**Risk: Breaking existing features**
- **Likelihood**: Very Low (changes are infrastructure-only)
- **Impact**: Critical (users can't use MCP)
- **Mitigation**: No code changes, only build/CI configs
- **Rollback**: Fast revert for both changes

**Risk: Documentation drift**
- **Likelihood**: Medium (forgot to update README)
- **Impact**: Low (confusion, but not breakage)
- **Mitigation**: Update docs in same PR
- **Rollback**: Not needed (docs can be updated separately)

## Testing Checklist

### Before Opening PR

**Test workflow**:
- [ ] Verified packageManager field in package.json
- [ ] Removed `with:` block from pnpm/action-setup step
- [ ] Added comment explaining auto-detection
- [ ] Checked YAML syntax (no tabs, correct indentation)

**Docker build**:
- [ ] Ran `pnpm build` to ensure daemon-client dist/ exists
- [ ] Local Docker build succeeds (amd64)
- [ ] Image size is acceptable (~220MB)
- [ ] Container starts without errors
- [ ] Verified pnpm version matches package.json
- [ ] Updated .dockerignore if needed

### After PR Merge

**Test workflow**:
- [ ] Pushed commit to trigger CI
- [ ] Verified workflow completes
- [ ] Checked pnpm version in logs
- [ ] Confirmed tests run

**Docker build**:
- [ ] Created test tag or manually triggered release
- [ ] Monitored multi-platform build
- [ ] Pulled published image
- [ ] Ran smoke tests
- [ ] Verified image metadata (version, labels)

## Monitoring and Alerts

### Metrics to Track

**CI health**:
- Test workflow success rate (should jump to ~100% for infra errors)
- Average build time (should stay within 10% of baseline)
- pnpm installation time (<30s)

**Docker build health**:
- Release workflow success rate (should jump to 100%)
- Build time (should stay within 10-15% of baseline)
- Image size trend (~220MB, stable)
- Pull success rate from Docker Hub

### Alert Conditions

**Immediate alerts**:
- Test workflow fails with pnpm errors
- Docker build fails with workspace resolution errors
- Image size exceeds 250MB
- Container fails to start

**Warning alerts**:
- Build time increases >20% from baseline
- pnpm version drift (Dockerfile vs package.json)
- Docker Hub pull failures

## Non-Testing Validation

### Code Review Focus

**Reviewers should verify**:
- YAML syntax correct (test.yml)
- Dockerfile layer ordering optimal for caching
- pnpm version matches package.json exactly
- Documentation updated
- No unintended file changes (e.g., lockfile)

**Reviewers should NOT test locally**:
- GitHub Actions environment differs from local
- Docker build works differently on Mac vs Linux
- Trust CI for actual validation

### Manual Verification Steps

**After test workflow merge**:
```bash
# Check workflow logs
gh run list --workflow=test.yml --limit 1
gh run view <run-id>
# Look for: "Setup pnpm" step output
# Should show: "pnpm 10.12.1" or similar
```

**After Docker build merge**:
```bash
# Check release workflow
gh run list --workflow=publish-maproom-mcp-image.yml --limit 1
gh run view <run-id>
# Look for: "Build and push Docker image" step
# Should show: "successfully tagged ..."

# Pull and test image
docker pull danielbushman/crewchief_maproom-mcp:latest
docker run --rm danielbushman/crewchief_maproom-mcp:latest node -e "console.log('OK')"
```

## Definition of Done

### Test Workflow

✅ **Fixed when**:
- [ ] PR merged to main
- [ ] Next CI run completes without pnpm version errors
- [ ] Tests execute (pass/fail on merit, not infra)
- [ ] No manual intervention needed for future runs

### Docker Build

✅ **Fixed when**:
- [ ] PR merged to main
- [ ] Next release build completes successfully
- [ ] Images pushed to Docker Hub (amd64 + arm64)
- [ ] Image pulls and runs
- [ ] Image size within acceptable range
- [ ] No manual intervention needed for future releases

### Documentation

✅ **Complete when**:
- [ ] .github/workflows/test.yml has explanatory comment
- [ ] Dockerfile.combined has pnpm strategy comment
- [ ] .github/CLAUDE.md updated with troubleshooting
- [ ] packages/maproom-mcp/CLAUDE.md notes build requirements

## Testing Anti-Patterns to Avoid

❌ **Don't** write unit tests for GitHub Actions YAML
- Reason: YAML linting doesn't catch runtime issues
- Alternative: Validate in actual CI

❌ **Don't** try to mock Docker builds locally
- Reason: Local environment differs (arch, base images)
- Alternative: Test locally with real Docker, validate in CI

❌ **Don't** test every edge case
- Reason: These are config fixes, not new features
- Alternative: Focus on happy path + rollback

❌ **Don't** add E2E tests for CI infrastructure
- Reason: Adds complexity, maintenance burden
- Alternative: Rely on actual CI runs as "live tests"

## Success Metrics

**Immediate** (Day 1):
- Test workflow: 1 successful run after merge
- Docker build: 1 successful multi-platform build

**Short-term** (Week 1):
- Test workflow: 100% success rate (excluding code bugs)
- Docker build: 2-3 successful releases
- Zero rollbacks needed

**Long-term** (Month 1):
- No pnpm version conflicts reported
- No workspace dependency resolution errors
- Image size stable
- Zero manual interventions

**Qualitative**:
- Developers can merge PRs without CI friction
- Releases publish smoothly
- No debugging of build issues
