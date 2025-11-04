# Quality Strategy: Binary Packaging Integration

## Testing Philosophy

**Core Principle**: Test what matters, skip what doesn't.

**What Matters**:
1. Binaries exist for all platforms before publish
2. Binaries are executable and work
3. Release process doesn't break existing workflows
4. Failures are caught before they reach users

**What Doesn't Matter** (for MVP):
- Perfect test coverage
- Testing every edge case
- Complex integration test suites
- Performance benchmarks

## Test Pyramid

```
         ┌─────────────┐
         │   Manual    │  ← Verify first release
         │   Testing   │
         └─────────────┘
              ▲
         ┌─────────────┐
         │ Integration │  ← npm install test
         │   Tests     │
         └─────────────┘
              ▲
         ┌─────────────┐
         │ Validation  │  ← Binary existence checks
         │   Scripts   │
         └─────────────┘
              ▲
         ┌─────────────┐
         │   CI Build  │  ← Matrix builds
         │    Tests    │
         └─────────────┘
```

## Layer 1: CI Build Tests (Automated, Every Build)

### Purpose
Verify each platform binary builds successfully.

### Tests

**Test 1.1: Binary Compilation**
- **When**: During matrix build
- **What**: `cargo build --release --target <target>` exits 0
- **Pass Criteria**: Build succeeds
- **Failure**: GitHub Actions job fails

**Test 1.2: Binary Exists**
- **When**: After build
- **What**: Check binary file exists at expected path
- **Pass Criteria**: File exists
- **Failure**: GitHub Actions job fails

**Test 1.3: Binary Size Sanity**
- **When**: After build
- **What**: Check binary size >1MB and <100MB
- **Pass Criteria**: Size in range
- **Failure**: GitHub Actions job warns (doesn't fail)

**Test 1.4: Binary Execution**
- **When**: After build
- **What**: Run `./crewchief-maproom --version`
- **Pass Criteria**: Exits 0 and outputs version
- **Failure**: GitHub Actions job fails

### Implementation

```bash
# In GitHub Actions workflow
- name: Test binary
  run: |
    BINARY=target/${{ matrix.target }}/release/crewchief-maproom

    # Test exists
    if [ ! -f "$BINARY" ]; then
      echo "❌ Binary not found"
      exit 1
    fi

    # Test size
    SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY")
    if [ "$SIZE" -lt 1000000 ]; then
      echo "❌ Binary too small: ${SIZE} bytes"
      exit 1
    fi
    echo "✓ Binary size: $((SIZE / 1000000))MB"

    # Test execution
    chmod +x "$BINARY"
    "$BINARY" --version
    echo "✓ Binary executes successfully"
```

## Layer 2: Validation Scripts (Automated, Pre-Publish)

### Purpose
Verify all required binaries are present before publishing to npm.

### Tests

**Test 2.1: All Platforms Present**
- **When**: Before npm publish (prepublishOnly hook)
- **What**: Check all 4 platform directories exist with binaries
- **Pass Criteria**: linux-x64, linux-arm64, darwin-x64, darwin-arm64 all present
- **Failure**: npm publish aborted with clear error

**Test 2.2: Binary Executables**
- **When**: Before npm publish
- **What**: Check each binary has execute permissions
- **Pass Criteria**: All binaries are executable
- **Failure**: npm publish aborted

**Test 2.3: Package Contents**
- **When**: After npm pack (in CI)
- **What**: Extract tarball and verify binaries inside
- **Pass Criteria**: All 4 binaries in tarball
- **Failure**: CI publish job fails

### Implementation

Script: `scripts/validate-binaries.js` (as designed in architecture.md)

```javascript
// Simplified version
const PLATFORMS = ['linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64'];
let allValid = true;

for (const platform of PLATFORMS) {
  const binaryPath = `packages/maproom-mcp/bin/${platform}/crewchief-maproom`;

  if (!fs.existsSync(binaryPath)) {
    console.error(`❌ Missing: ${platform}`);
    allValid = false;
  } else {
    const stats = fs.statSync(binaryPath);
    console.log(`✓ ${platform}: ${(stats.size / 1e6).toFixed(1)}MB`);
  }
}

if (!allValid) {
  console.error('\n❌ Cannot publish: Missing binaries');
  console.error('Run: gh workflow run build-and-publish-maproom-mcp.yml');
  process.exit(1);
}
```

### Test Execution

```bash
# Automatic (prepublishOnly hook)
npm publish → validates binaries → publishes

# Manual
node scripts/validate-binaries.js
```

## Layer 3: Integration Tests (Automated, Post-Publish)

### Purpose
Verify published package installs and works on each platform.

### Scope
**MVP**: Optional post-publish verification
**Post-MVP**: Automated test matrix

### Tests

**Test 3.1: Package Install**
- **When**: After npm publish (optional)
- **Platforms**: linux-x64, linux-arm64, darwin-x64, darwin-arm64
- **What**: `npm install -g @crewchief/maproom-mcp@latest`
- **Pass Criteria**: Installs without error

**Test 3.2: Binary Detection**
- **When**: After install
- **What**: Verify correct platform binary is used
- **Pass Criteria**: `npx @crewchief/maproom-mcp --version` runs correct binary

**Test 3.3: Basic Functionality**
- **When**: After install
- **What**: Run `npx @crewchief/maproom-mcp setup --provider=ollama`
- **Pass Criteria**: Setup completes without crash

### Implementation (Post-MVP)

```yaml
# .github/workflows/test-published-package.yml
name: Test Published Package

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to test'
        required: true

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-13, macos-latest]
        include:
          - os: ubuntu-latest
            platform: linux-x64
          - os: macos-13
            platform: darwin-x64
          - os: macos-latest
            platform: darwin-arm64

    runs-on: ${{ matrix.os }}

    steps:
      - name: Install package
        run: npm install -g @crewchief/maproom-mcp@${{ inputs.version }}

      - name: Test version
        run: npx @crewchief/maproom-mcp --version

      - name: Test binary detection
        run: |
          BINARY=$(which crewchief-maproom)
          echo "Binary: $BINARY"
          echo "Platform: ${{ matrix.platform }}"
```

**Note**: This is optional for MVP, can be added later.

## Layer 4: Manual Testing (One-Time, First Release)

### Purpose
Validate new release process works end-to-end.

### Test Procedure

**Pre-Flight Checks**:
1. [ ] Clean git working directory
2. [ ] On main branch
3. [ ] All tests passing
4. [ ] npm login successful
5. [ ] GitHub CLI authenticated

**Test Release** (Dry Run):
1. [ ] Create test branch
2. [ ] Run `pnpm release:patch --dry-run`
3. [ ] Verify output shows:
   - Version bump
   - Git commit message
   - Git tag
   - Push commands
4. [ ] No actual changes made

**First Real Release**:
1. [ ] Run `pnpm release:patch`
2. [ ] Verify git commit and tag created
3. [ ] Verify tag pushed to GitHub
4. [ ] Monitor GitHub Actions workflow
5. [ ] Verify all 4 builds complete
6. [ ] Verify validation passes
7. [ ] Verify npm publish succeeds
8. [ ] Verify package on npm registry

**Post-Release Verification**:
1. [ ] Install on linux-x64: `npm install -g @crewchief/maproom-mcp@latest`
2. [ ] Test on linux-x64: `npx @crewchief/maproom-mcp --version`
3. [ ] Install on macOS (if available)
4. [ ] Test on macOS: `npx @crewchief/maproom-mcp --version`
5. [ ] Check package size on npm
6. [ ] Verify binaries present in tarball

**Rollback Test** (if issues found):
1. [ ] Unpublish bad version: `npm unpublish @crewchief/maproom-mcp@X.Y.Z`
2. [ ] Fix issue
3. [ ] Repeat release process

## Quality Gates

### Gate 1: Pre-Commit
- Local build succeeds
- ESLint passes
- TypeScript compiles

### Gate 2: Pre-Release
- Git working directory clean
- On main/master branch
- Tests passing (if any)

### Gate 3: CI Build
- All 4 platform builds succeed
- Binaries execute successfully
- Binary sizes reasonable

### Gate 4: Pre-Publish
- All 4 binaries present
- All binaries executable
- Tarball verification passes

### Gate 5: Post-Publish (Optional)
- Package appears on npm
- Install test succeeds
- Binary detection works

## Failure Scenarios and Testing

### Scenario 1: Missing Binary for One Platform

**Test**: Delete `bin/linux-x64/` before publish
**Expected**: prepublishOnly hook fails, publish blocked
**Verification**: Error message clearly indicates missing platform

### Scenario 2: Corrupted Binary

**Test**: Replace binary with empty file
**Expected**: Size check fails in CI or validation
**Verification**: Build fails or validation fails

### Scenario 3: Wrong Binary Uploaded

**Test**: Copy darwin binary to linux directory
**Expected**: Binary execution test fails in CI
**Verification**: CI detects platform mismatch

### Scenario 4: Network Failure During Publish

**Test**: (Cannot simulate easily)
**Expected**: npm publish fails
**Verification**: CI shows error, can re-run

### Scenario 5: Concurrent Publish

**Test**: Two developers run release:patch simultaneously
**Expected**: Second publish fails (version conflict)
**Verification**: npm registry rejects duplicate version

## Success Metrics

### Quantitative

1. **Reliability**: 100% of releases include all 4 binaries
   - **Measure**: Count releases with all binaries / total releases
   - **Target**: 100%

2. **Build Success Rate**: >95% of builds succeed
   - **Measure**: Successful builds / total builds
   - **Target**: >95%

3. **Build Time**: <15 minutes per release
   - **Measure**: Time from tag push to npm publish
   - **Target**: <15 minutes

4. **Package Size**: <100MB
   - **Measure**: npm package tarball size
   - **Target**: <100MB (ideally <60MB)

### Qualitative

1. **Developer Experience**: "pnpm release:x works every time"
2. **Failure Detection**: "Failures caught before npm publish"
3. **Error Messages**: "Clear guidance when something fails"

## Risk-Based Testing

### High Risk, High Impact → Test Thoroughly
- Binary missing for platform (Test 2.1) ✓
- Binary not executable (Test 1.4, 2.2) ✓
- Publish with incomplete binaries (Gate 4) ✓

### High Risk, Low Impact → Test Minimally
- Build time exceeds target (Monitor, don't test)
- Package size slightly over target (Warn, don't fail)

### Low Risk, High Impact → Test Eventually
- Platform compatibility issues (Post-MVP integration tests)
- Binary corruption (CI build tests catch most cases)

### Low Risk, Low Impact → Don't Test
- Specific error message wording
- GitHub Actions runner selection
- Binary optimization effectiveness

## Test Maintenance

### What to Update When

**Adding a New Platform**:
1. Update PLATFORMS array in validate-binaries.js
2. Add platform to GitHub Actions matrix
3. Update manual test checklist
4. Update documentation

**Changing Build Process**:
1. Update CI build tests
2. Update validation scripts if needed
3. Run manual test procedure
4. Update documentation

**Updating Dependencies**:
1. Verify CI still works
2. Verify binaries still build
3. No test changes needed (abstracted)

## Testing Tools

### Required
- GitHub Actions (CI infrastructure)
- Node.js built-in `fs`, `path` (validation scripts)
- Bash/shell scripts (CI tests)
- npm CLI (publish, pack)

### Optional (Post-MVP)
- Docker (multi-platform testing)
- GitHub CLI (workflow monitoring)
- npm API (package verification)

## Acceptance Criteria for "Done"

A release process change is complete when:

1. ✅ All 4 platform binaries build in CI
2. ✅ Validation scripts prevent incomplete publishes
3. ✅ `pnpm release:x` triggers full pipeline
4. ✅ Manual test procedure passes
5. ✅ At least one successful production release
6. ✅ Documentation updated

## Known Limitations (Acceptable for MVP)

1. **No Windows Support**: Not a current requirement
2. **No Automated Post-Publish Tests**: Can add later
3. **No Binary Signing**: Not critical for open source
4. **No Reproducible Builds**: Nice to have, not essential
5. **No Build Caching**: Acceptable build times without it

## Post-MVP Testing Enhancements

**Priority 1** (If issues found):
- Automated post-publish verification
- Binary integrity checks (checksums)

**Priority 2** (Quality improvements):
- Build caching to reduce CI time
- Automated rollback on test failure

**Priority 3** (Nice to have):
- Windows platform support
- Binary signing
- Reproducible builds verification

## Summary

The quality strategy focuses on **preventing the specific failure mode that occurred**: publishing packages without required binaries.

**Key Testing Layers**:
1. **CI builds**: Ensure binaries compile for each platform
2. **Validation scripts**: Block publish if binaries missing
3. **Manual verification**: Confirm first release works
4. **Post-MVP**: Automated post-publish tests

**Success Criteria**:
- Zero incomplete releases
- Clear failure messages
- Fast feedback (<15min)
- Low maintenance burden

This pragmatic approach provides confidence without excessive ceremony or maintenance overhead.
