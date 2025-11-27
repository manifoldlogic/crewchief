# Quality Strategy: VSCode Workflow Testing

## Testing Philosophy

**Goal**: Ship with confidence, not ceremonial coverage
**Focus**: Critical paths that prevent user-facing failures
**Approach**: Layered testing from syntax to end-to-end

## Test Layers

### Layer 1: Syntax Validation (Pre-commit)

**What**: YAML syntax correctness
**How**:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-vscode-maproom.yml'))"
```

**Success Criteria**: YAML parses without errors
**Automated**: Run before git commit
**Purpose**: Catch obvious syntax errors before push

### Layer 2: GitHub Actions Validation (On Push)

**What**: GitHub-specific workflow validation
**How**: Push to main, check for validation errors
**Success Criteria**: No "workflow file issue" errors
**Automated**: Happens on every push
**Purpose**: Ensure GitHub can parse and validate workflow

### Layer 3: Dry Run Testing (Manual)

**What**: End-to-end without publishing
**How**:
```bash
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=true
```

**Success Criteria**:
- ✅ Build completes
- ✅ Package creates valid .vsix
- ✅ Smoke tests pass
- ✅ No publishing occurs
- ✅ Workflow completes successfully

**Automated**: Can be run in CI on workflow file changes
**Purpose**: Validate packaging without marketplace interaction

### Layer 4: Pre-release Testing (Staging)

**What**: Real publishing to marketplaces with pre-release
**How**:
```bash
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0-rc.1 \
  --field pre_release=true
```

**Success Criteria**:
- ✅ Publishes to VS Code Marketplace
- ✅ Publishes to Open VSX Registry
- ✅ Creates GitHub pre-release
- ✅ Extension installable from marketplace
- ✅ Pre-release badge visible

**Automated**: No (requires cleanup after test)
**Purpose**: Validate marketplace integration

### Layer 5: Production Validation (Post-release)

**What**: Verify production release
**How**: Manual checks after real release

**Checklist**:
- [ ] Version appears in VS Code Marketplace
- [ ] Version appears in Open VSX Registry
- [ ] GitHub release created with .vsix
- [ ] Extension installs via `code --install-extension`
- [ ] Extension loads in VS Code
- [ ] No errors in extension host logs

**Automated**: Partially (can check marketplace APIs)
**Purpose**: Confirm release succeeded end-to-end

## Critical Test Scenarios

### Scenario 1: Happy Path

**Setup**: All secrets configured, valid version
**Steps**:
1. Run workflow with `dry_run=false`
2. Monitor job progress
3. Check marketplace listings
4. Verify GitHub release

**Expected**:
- All jobs succeed
- Both marketplaces updated
- GitHub release created
- .vsix downloadable

**Risk if fails**: Complete failure to release

### Scenario 2: Missing VS Code PAT

**Setup**: Remove VSCE_PAT secret
**Steps**:
1. Run workflow
2. Observe VS Code publish step

**Expected**:
- VS Code publish skipped (env var empty)
- Open VSX publishes successfully
- GitHub release still created
- Logs show VS Code was skipped

**Risk if fails**: Workflow fails entirely instead of graceful degradation

### Scenario 3: Missing Open VSX PAT

**Setup**: Remove OVSX_PAT secret
**Steps**:
1. Run workflow
2. Observe Open VSX publish step

**Expected**:
- Open VSX publish skipped
- VS Code publishes successfully
- GitHub release still created

**Risk if fails**: Workflow fails entirely

### Scenario 4: Both PATs Missing

**Setup**: Remove both secrets
**Steps**:
1. Run workflow
2. Observe publish job

**Expected**:
- Both publish steps skipped
- No GitHub release created (no successful publishes)
- Workflow completes with warning
- .vsix artifact still available

**Risk if fails**: Workflow crashes instead of graceful exit

### Scenario 5: Version Mismatch

**Setup**: Input version different from package.json
**Steps**:
1. Run workflow with version=0.2.0
2. package.json has version=0.1.0

**Expected**:
- package-extension job fails
- Clear error message about mismatch
- No publishing occurs
- Workflow stops early

**Risk if fails**: Wrong version published

### Scenario 6: Invalid .vsix

**Setup**: Corrupt build artifact
**Steps**:
1. Manually corrupt dist/extension.js
2. Run workflow

**Expected**:
- Smoke tests fail
- Workflow stops before publishing
- Error message indicates which test failed

**Risk if fails**: Broken extension published

### Scenario 7: Marketplace API Failure

**Setup**: vsce publish command fails (bad PAT, network issue)
**Steps**:
1. Run workflow
2. Simulate marketplace unavailability

**Expected**:
- Step fails with continue-on-error
- Error logged
- Other marketplace proceeds
- Release created if one succeeds

**Risk if fails**: Entire workflow fails on transient error

## Smoke Tests

### Package Validation

```bash
# Test 1: Verify package.json exists in .vsix
unzip -l "$VSIX_FILE" | grep -q "extension/package.json"

# Test 2: Verify dist/ directory exists
unzip -l "$VSIX_FILE" | grep -q "extension/dist/"

# Test 3: Verify extension.js exists
unzip -l "$VSIX_FILE" | grep -q "extension/dist/extension.js"

# Test 4: Verify package is not too large (>10MB suspicious)
SIZE=$(stat -f%z "$VSIX_FILE" 2>/dev/null || stat -c%s "$VSIX_FILE")
if [ "$SIZE" -gt 10485760 ]; then
  echo "WARNING: Package size $SIZE > 10MB"
fi
```

**Purpose**: Catch obvious packaging issues before publishing
**Runtime**: <1 second
**Failure mode**: Fail workflow immediately

## Integration Test Plan

### Phase 1: Local Validation (Day 1)

**Tests**:
1. YAML syntax validation
2. Workflow dispatch dry-run
3. .vsix creation
4. Smoke tests

**Environment**: Development branch
**Success Criteria**: All tests pass locally

### Phase 2: CI Validation (Day 1)

**Tests**:
1. Push workflow to main
2. Verify no validation errors
3. Trigger dry-run via workflow_dispatch
4. Download and inspect .vsix artifact

**Environment**: Main branch
**Success Criteria**: GitHub validates workflow, dry-run succeeds

### Phase 3: Staging Test (Day 2)

**Tests**:
1. Publish pre-release version (0.1.0-rc.1)
2. Verify both marketplaces
3. Install and test extension
4. Delete pre-release

**Environment**: Production marketplaces, pre-release mode
**Success Criteria**: Extension installs and loads

### Phase 4: Production Release (Day 3+)

**Tests**:
1. Release actual version (0.1.0)
2. Verify marketplaces
3. Verify GitHub release
4. Monitor for user issues

**Environment**: Production
**Success Criteria**: Extension available to users

## Regression Testing

### When to Test

**Trigger Events**:
- Workflow file modified
- Reusable workflows updated
- Package.json changed
- vsce/ovsx CLI updated

**Test Suite**: Run Layer 3 (dry-run) minimum

### Test Automation

**GitHub Actions Workflow** (`.github/workflows/test-vscode-workflow.yml`):
```yaml
name: Test VSCode Workflow

on:
  pull_request:
    paths:
      - '.github/workflows/release-vscode-maproom.yml'
      - 'packages/vscode-maproom/package.json'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate YAML
        run: python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-vscode-maproom.yml'))"

      - name: Dry run test
        run: gh workflow run release-vscode-maproom.yml --field version=0.1.0 --field dry_run=true
```

**Purpose**: Catch regressions before merge

## Performance Testing

### Workflow Duration Targets

| Job | Target | Acceptable | Unacceptable |
|-----|--------|------------|--------------|
| build-extension | <3 min | <5 min | >7 min |
| package-extension | <1 min | <2 min | >3 min |
| publish-extension | <2 min | <4 min | >6 min |
| **Total** | **<6 min** | **<11 min** | **>16 min** |

**Monitoring**: Check workflow duration after each run
**Action if slow**: Investigate caching, optimize builds

### Artifact Size

**Target**: <500 KB
**Acceptable**: <2 MB
**Unacceptable**: >5 MB

**Monitoring**: Check .vsix size in smoke tests
**Action if large**: Investigate bundled dependencies

## Error Handling Tests

### Expected Errors (Workflow Continues)

1. **Missing secret**: Publish step skipped
2. **Marketplace timeout**: Retry or skip
3. **Partial success**: Release created

**Test Method**: Remove secrets, check logs

### Fatal Errors (Workflow Stops)

1. **Version mismatch**: Package job fails
2. **Build failure**: Build job fails
3. **Invalid .vsix**: Smoke tests fail

**Test Method**: Inject failures, verify early exit

## Test Documentation

### Test Results Location

```
.agents/projects/VSCODFIX_vscode-workflow-repair/
└── test-results/
    ├── 2025-11-23-dry-run.md
    ├── 2025-11-24-prerelease.md
    └── 2025-11-25-production.md
```

**Format**:
```markdown
# Test Run: 2025-11-23 Dry Run

## Configuration
- Version: 0.1.0
- dry_run: true
- pre_release: false

## Results
- Build: ✅ 2m 15s
- Package: ✅ 45s
- Publish: ⏭️ Skipped (dry_run)

## Artifacts
- .vsix: 420 KB
- Smoke tests: All passed

## Issues
None
```

### Runbook

**Location**: `.github/VSCODE_PUBLISHING.md`
**Contents**:
- How to trigger workflow
- How to verify success
- Common failure scenarios
- Troubleshooting steps

## Risk Mitigation

| Risk | Severity | Mitigation | Test Coverage |
|------|----------|------------|---------------|
| Validation failure | High | Syntax tests, manual review | Layer 1, 2 |
| Wrong version published | High | Version verification step | Scenario 5 |
| Broken extension | High | Smoke tests | Layer 3, Scenario 6 |
| Missing secrets | Medium | Step-level checks | Scenarios 2, 3, 4 |
| Marketplace failure | Medium | continue-on-error | Scenario 7 |
| Slow workflow | Low | Performance monitoring | Performance tests |

## Success Metrics

1. **Zero validation errors** on push to main
2. **95%+ dry-run success rate**
3. **<6 minute workflow duration**
4. **100% smoke test pass rate** before publishing
5. **Graceful degradation** on partial failures

## MVP Testing Scope

**In Scope**:
- ✅ Syntax validation
- ✅ GitHub validation
- ✅ Dry-run testing
- ✅ Basic smoke tests
- ✅ Version verification
- ✅ Secret handling

**Out of Scope** (Future):
- ❌ Automated marketplace verification
- ❌ Extension functionality tests
- ❌ User acceptance testing
- ❌ Performance benchmarking
- ❌ Load testing

**Rationale**: MVP focuses on workflow reliability, not extension quality
