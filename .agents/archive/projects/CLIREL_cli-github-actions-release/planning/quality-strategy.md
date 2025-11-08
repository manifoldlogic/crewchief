# Quality Strategy: CLI GitHub Actions Release Automation

## Testing Philosophy

This is **infrastructure automation** - the code we're writing orchestrates existing, well-tested components (GitHub Actions, npm, Cargo). Our testing strategy should focus on **integration validation** and **preventing regressions**, not exhaustive unit testing of workflow logic.

**Key insight**: The workflow itself is declarative YAML - we can't unit test it. Our quality assurance comes from:
1. **Workflow validation** (syntax, structure)
2. **Integration testing** (dry-run releases)
3. **Production monitoring** (verify published packages)

## Risk-Based Testing Priorities

### High Risk (Must Test)

**1. Binary cross-compilation correctness**
- **Risk**: Wrong binary shipped for platform (e.g., ARM binary in x64 package)
- **Test**: Download published package, inspect tarball, verify each binary is correct arch
- **Automation**: Validation script in workflow checks `file` output for each binary

**2. Package name and scope**
- **Risk**: Publish to wrong package name (e.g., `crewchief` instead of `@crewchief/cli`)
- **Test**: Inspect package.json before publish, verify `name` field
- **Automation**: Validation script checks package.json contains `"name": "@crewchief/cli"`

**3. Complete platform coverage**
- **Risk**: Missing platform binary (e.g., forget darwin-x64)
- **Test**: Check all 4 binaries exist before publish
- **Automation**: Validation script verifies existence of all bin/{platform}/ directories

**4. Tag-triggered workflow isolation**
- **Risk**: CLI tag triggers MCP workflow or vice versa
- **Test**: Create test tags, verify only correct workflow runs
- **Automation**: Dry-run test with synthetic tags

### Medium Risk (Should Test)

**5. Binary size sanity**
- **Risk**: Bloated binary (forgot to strip) or corrupted binary (too small)
- **Test**: Check binary size is in expected range (5MB-20MB)
- **Automation**: Validation script checks `stat` output for each binary

**6. TypeScript build completeness**
- **Risk**: Partial TypeScript build (missing dist/ files)
- **Test**: Check dist/ directory has expected files
- **Automation**: Validation script counts files in dist/, checks for index.js

**7. npm authentication**
- **Risk**: Missing or invalid NPM_TOKEN secret
- **Test**: Attempt dry-run publish
- **Automation**: Manual verification during initial setup, then trust

**8. Version bumping**
- **Risk**: Release script creates wrong version number
- **Test**: Manual code review of release.mjs changes
- **Automation**: None needed (simple logic, low change frequency)

### Low Risk (Nice to Have)

**9. Deprecation messaging**
- **Risk**: Users don't see migration warning
- **Test**: Manual testing of old package install
- **Automation**: None (one-time operation)

**10. Binary execution on target platforms**
- **Risk**: Binary built but doesn't run (missing system dependencies)
- **Test**: Run `--version` on each platform binary
- **Automation**: Native platform only (can't execute cross-arch in CI)

**11. Package metadata**
- **Risk**: Missing README, LICENSE, or important metadata
- **Test**: Inspect package tarball contents
- **Automation**: Validation script lists tarball contents, checks for key files

## Testing Strategy by Phase

### Phase 1: Deprecation (Old Package)

**Goal**: Ensure users see migration message and can find new package.

**Manual tests**:
- [ ] Install `crewchief@1.0.0` → See postinstall warning
- [ ] Check npm page shows deprecation notice
- [ ] Verify deprecation message includes new package name

**No automation needed**: One-time operation, low risk.

### Phase 2: Package Rename

**Goal**: Ensure new package configuration is correct before any automation.

**Manual tests**:
- [ ] package.json has `"name": "@crewchief/cli"`
- [ ] package.json has `"version": "1.0.0"`
- [ ] package.json has `"publishConfig": { "access": "public" }`
- [ ] .npmignore excludes src/, includes bin/
- [ ] Local `npm pack` produces tarball with expected structure

**Automated test** (local script):
```bash
#!/bin/bash
# test-package-config.sh

cd packages/cli

# Check package name
NAME=$(node -p "require('./package.json').name")
if [ "$NAME" != "@crewchief/cli" ]; then
  echo "ERROR: Package name is $NAME, expected @crewchief/cli"
  exit 1
fi

# Check version format
VERSION=$(node -p "require('./package.json').version")
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "ERROR: Invalid version format: $VERSION"
  exit 1
fi

# Check publishConfig
ACCESS=$(node -p "require('./package.json').publishConfig?.access || 'missing'")
if [ "$ACCESS" != "public" ]; then
  echo "ERROR: publishConfig.access is $ACCESS, expected public"
  exit 1
fi

echo "✓ Package configuration valid"
```

### Phase 3: Release Script Updates

**Goal**: Ensure release script creates correct tags and doesn't accidentally publish.

**Manual tests**:
- [ ] Run release script → creates `@crewchief/cli@v*` tag (not `crewchief@v*`)
- [ ] Release script doesn't call `pnpm publish`
- [ ] Tag is pushed to remote

**Automated test** (dry-run):
```bash
#!/bin/bash
# test-release-script.sh

cd packages/cli

# Mock git commands to test tag format
export GIT_TEST_MODE=1  # Would need to modify release.mjs to support this

# Run release script
node scripts/release.mjs patch

# Check that tag would be correct format
# (Requires release.mjs modification to support dry-run)
```

**Pragmatic approach**: Manual code review + single test run. Script is small and changes rarely.

### Phase 4: GitHub Actions Workflow

**Goal**: Ensure workflow builds all platforms and validates correctly before publish.

**Workflow validation** (pre-deployment):
```bash
# Validate YAML syntax
yamllint .github/workflows/build-and-publish-cli.yml

# Check workflow logic with act (local GitHub Actions runner)
act -n  # Dry-run mode
```

**Integration test** (dry-run release):
```bash
# 1. Create test tag locally
git tag @crewchief/cli@v1.0.0-test
git push origin @crewchief/cli@v1.0.0-test

# 2. Trigger workflow with dry-run
gh workflow run build-and-publish-cli.yml -f dry_run=true

# 3. Monitor workflow run
gh run watch

# 4. Verify:
# - All 4 platform jobs succeeded
# - Validation job succeeded
# - Publish step was skipped (dry-run)

# 5. Cleanup
git tag -d @crewchief/cli@v1.0.0-test
git push origin :refs/tags/@crewchief/cli@v1.0.0-test
```

**Automated validation** (in workflow):
```yaml
- name: Validate binaries
  run: |
    #!/bin/bash
    set -e

    # Check existence
    for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
      binary="packages/cli/bin/$platform/crewchief-maproom"
      if [ ! -f "$binary" ]; then
        echo "ERROR: Missing binary for $platform"
        exit 1
      fi
      echo "✓ Binary exists: $platform"
    done

    # Check sizes
    for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
      binary="packages/cli/bin/$platform/crewchief-maproom"
      size=$(stat -f%z "$binary" 2>/dev/null || stat -c%s "$binary")
      if [ $size -lt 5000000 ] || [ $size -gt 20000000 ]; then
        echo "ERROR: Binary size $size out of range [5MB-20MB] for $platform"
        exit 1
      fi
      echo "✓ Binary size valid: $platform ($size bytes)"
    done

    # Check TypeScript build
    if [ ! -d "packages/cli/dist" ]; then
      echo "ERROR: dist/ directory missing"
      exit 1
    fi
    if [ ! -f "packages/cli/dist/cli/index.js" ]; then
      echo "ERROR: Missing dist/cli/index.js"
      exit 1
    fi
    echo "✓ TypeScript build valid"

    # Check package structure
    cd packages/cli
    npm pack --dry-run > /tmp/pack-output.txt 2>&1
    for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
      if ! grep -q "bin/$platform" /tmp/pack-output.txt; then
        echo "ERROR: Platform $platform not in package"
        exit 1
      fi
    done
    echo "✓ Package structure valid"
```

**Production verification** (post-publish):
```yaml
- name: Verify publication
  if: ${{ !inputs.dry_run }}
  run: |
    #!/bin/bash
    set -e

    VERSION=$(node -p "require('./packages/cli/package.json').version")

    # Wait for registry to update
    echo "Waiting for npm registry..."
    sleep 10

    # Check package exists
    if ! npm view @crewchief/cli@$VERSION version > /dev/null 2>&1; then
      echo "ERROR: Package not found on npm registry"
      exit 1
    fi
    echo "✓ Package published to registry"

    # Download and inspect
    npm pack @crewchief/cli@$VERSION
    tar -tzf crewchief-cli-$VERSION.tgz | grep -q "bin/darwin-arm64" || exit 1
    tar -tzf crewchief-cli-$VERSION.tgz | grep -q "bin/darwin-x64" || exit 1
    tar -tzf crewchief-cli-$VERSION.tgz | grep -q "bin/linux-arm64" || exit 1
    tar -tzf crewchief-cli-$VERSION.tgz | grep -q "bin/linux-x64" || exit 1
    echo "✓ Published package contains all platforms"

    # Test installation
    npm install -g @crewchief/cli@$VERSION
    crewchief --version
    echo "✓ Package installs and runs"
```

### Phase 5: MCP Workflow Update

**Goal**: Ensure MCP workflow still works with new tag pattern.

**Manual tests**:
- [ ] Update MCP workflow trigger to `@crewchief/maproom-mcp@v*`
- [ ] Update MCP release script to create scoped tags
- [ ] Create test MCP tag → Verify only MCP workflow runs
- [ ] Create test CLI tag → Verify only CLI workflow runs

**Isolation test**:
```bash
# 1. Create both tag types
git tag @crewchief/cli@v1.0.0-test
git tag @crewchief/maproom-mcp@v1.3.6-test

# 2. Push both
git push origin @crewchief/cli@v1.0.0-test
git push origin @crewchief/maproom-mcp@v1.3.6-test

# 3. Verify:
# - CLI workflow runs for CLI tag only
# - MCP workflow runs for MCP tag only
# - No cross-triggering

# 4. Cleanup
git tag -d @crewchief/cli@v1.0.0-test @crewchief/maproom-mcp@v1.3.6-test
git push origin :refs/tags/@crewchief/cli@v1.0.0-test
git push origin :refs/tags/@crewchief/maproom-mcp@v1.3.6-test
```

## Test Implementation Plan

### Pre-Deployment Tests (Manual)

**Checklist**:
- [ ] Run `test-package-config.sh` → PASS
- [ ] Code review release.mjs changes → Correct tag format
- [ ] `yamllint` workflow YAML → Valid syntax
- [ ] Create dry-run tag → Workflow runs successfully
- [ ] Inspect dry-run artifacts → All 4 binaries present
- [ ] Check dry-run validation → All checks pass

### Deployment Tests (Automated)

**Built into workflow**:
- ✅ Binary existence checks (fail-fast)
- ✅ Binary size validation (fail-fast)
- ✅ TypeScript build validation (fail-fast)
- ✅ Package structure inspection (fail-fast)
- ✅ Post-publish verification (alerting)

### Post-Deployment Tests (Manual)

**Production validation checklist**:
- [ ] Install `@crewchief/cli@1.0.0` → Works
- [ ] Run `crewchief --version` → Correct version
- [ ] Test on linux-x64 → Binary executes
- [ ] Test on darwin-arm64 → Binary executes
- [ ] Check npm page → Metadata correct
- [ ] Check package tarball → All platforms included

## Monitoring Strategy

### Workflow Health

**Metrics to track**:
- Build success rate (target: 100%)
- Build duration (target: <15 minutes)
- Binary sizes (trend monitoring)
- Publish success rate (target: 100%)

**GitHub Actions built-in**:
- Workflow run history
- Job duration tracking
- Artifact sizes
- Failure notifications (email)

**Custom monitoring** (optional):
```yaml
- name: Report metrics
  if: always()
  run: |
    echo "Build duration: ${{ steps.build.duration }}"
    echo "Binary sizes:"
    du -h packages/cli/bin/*/crewchief-maproom
```

### Package Health

**npm registry checks** (manual, post-release):
- Package download counts (weekly)
- Deprecation of old package (`crewchief`) working
- Version tags correct on npm
- Package size trend (should stay under 100MB)

**User feedback monitoring**:
- GitHub issues mentioning installation problems
- Platform-specific binary issues
- Migration confusion (old → new package)

## Regression Prevention

### Common Failure Modes

**1. Binary mismatch** (wrong binary for platform)
- **Prevention**: Validation script checks each binary with `file` command
- **Detection**: Size checks (different archs have different sizes)
- **Recovery**: Delete tag, fix workflow, re-release

**2. Missing platform binary**
- **Prevention**: Explicit check for all 4 platform directories
- **Detection**: Validation fails before publish
- **Recovery**: Fix matrix config, re-run

**3. TypeScript build incomplete**
- **Prevention**: Check dist/ directory contents
- **Detection**: Validation script checks for index.js
- **Recovery**: Fix tsup config, re-release

**4. Wrong package name published**
- **Prevention**: Validation checks package.json name field
- **Detection**: Post-publish verification
- **Recovery**: Unpublish (if caught quickly), or publish corrected version

**5. Tag triggers wrong workflow**
- **Prevention**: Isolated trigger patterns in both workflows
- **Detection**: Manual tag isolation testing
- **Recovery**: Fix workflow trigger, delete tag, re-tag

### Safety Mechanisms

**Fail-fast validation**:
```yaml
- name: Validate (fail before publish)
  run: ./scripts/validate-package.sh
  # If this fails, publish step never runs
```

**Dry-run support**:
```yaml
workflow_dispatch:
  inputs:
    dry_run:
      description: 'Skip publish step'
      type: boolean
      default: false
```

**Manual approval** (optional, not MVP):
```yaml
- name: Publish
  environment: production  # Requires manual approval in GitHub
  run: npm publish
```

## Testing Anti-Patterns to Avoid

**❌ Don't**: Write unit tests for workflow YAML
- **Why**: Can't unit test declarative config
- **Instead**: Integration test with dry-run releases

**❌ Don't**: Test every edge case of binary compilation
- **Why**: Cargo/Rust toolchain already tested
- **Instead**: Trust toolchain, validate output

**❌ Don't**: Mock npm registry interactions
- **Why**: Need real registry behavior to catch issues
- **Instead**: Use dry-run mode and test accounts

**❌ Don't**: Test cross-platform binary execution in CI
- **Why**: Can't run ARM binaries on x64 runner
- **Instead**: Validate binary exists and has correct size

**❌ Don't**: Build comprehensive test suite for one-time operations
- **Why**: Deprecation is manual, low-risk, won't repeat
- **Instead**: Manual checklist and code review

## Quality Gates

### Gate 1: Package Configuration (Pre-Workflow)
- ✅ Package name correct
- ✅ Version format valid
- ✅ publishConfig present
- ✅ files array includes bin/
- ✅ .npmignore excludes source

**Who**: Developer (manual checklist)
**When**: Before creating workflow

### Gate 2: Workflow Dry-Run (Pre-Production)
- ✅ All 4 platform builds succeed
- ✅ All binaries exist and correct size
- ✅ TypeScript build succeeds
- ✅ Package structure valid
- ✅ Validation passes

**Who**: Automated (workflow validation)
**When**: Before first real release

### Gate 3: Production Release (First Release)
- ✅ Tag triggers correct workflow only
- ✅ Binaries build successfully
- ✅ Package publishes to npm
- ✅ Post-publish verification passes
- ✅ Manual installation test succeeds

**Who**: Automated + developer verification
**When**: v1.0.0 release

### Gate 4: Post-Release Monitoring (Ongoing)
- ✅ Package installs on all platforms
- ✅ No user reports of missing binaries
- ✅ Deprecation working (old package)
- ✅ npm metadata correct

**Who**: Developer (manual checks)
**When**: 24 hours after release

## Success Criteria

**Functional**:
- [ ] CLI package publishes with all 4 platform binaries
- [ ] Package name is `@crewchief/cli`
- [ ] Independent tagging works (CLI and MCP separate)
- [ ] Old package properly deprecated

**Quality**:
- [ ] Zero manual steps after tag creation
- [ ] All platform binaries validated before publish
- [ ] Broken releases prevented by validation gates
- [ ] Dry-run testing possible

**Process**:
- [ ] Workflow completes in <15 minutes
- [ ] No manual intervention required
- [ ] Clear failure messages guide debugging
- [ ] Validation catches issues before publish

## Risk Acceptance

**Acceptable risks** (won't test):
- Binary execution on non-native platforms (can't test ARM on x64 runner)
- npm registry downtime (external service, out of control)
- Rare edge cases in tag format (manual code review sufficient)
- Deprecation message formatting (one-time, manual test okay)

**Unacceptable risks** (must test):
- Wrong binaries shipped for platforms
- Missing platform binaries
- Package published to wrong name
- Workflows triggering incorrectly
