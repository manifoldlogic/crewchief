# Quick Start: Canary Release Test

**Ticket**: BINPKG-1901
**Status**: Ready for manual execution
**Estimated Time**: 60-90 minutes (including dry run)

---

## TL;DR - What You Need to Do

This ticket tests the GitHub Actions workflow by publishing a canary version `1.3.1-canary.1` to npm. It requires manual execution because it involves GitHub Actions and npm publication.

---

## Option 1: Dry Run First (RECOMMENDED)

**Safest approach - tests without publishing to npm**

1. Go to: https://github.com/danielbushman/crewchief/actions
2. Click "Build and Publish Maproom MCP" workflow
3. Click "Run workflow" button
4. Set "Dry run (skip publish)": **true**
5. Click "Run workflow"
6. Wait ~25-30 minutes, monitor jobs
7. If all green: proceed to full test
8. If any red: check logs, create fix tickets

**Time**: ~30-45 minutes

---

## Option 2: Full Canary Test

**Only after dry run passes (or if feeling adventurous)**

```bash
# Step 1: Create test branch
cd /workspace
git checkout -b test/canary-release

# Step 2: Bump version
# Edit packages/maproom-mcp/package.json
# Change "version": "1.3.0" to "version": "1.3.1-canary.1"

# Step 3: Commit and tag
git add packages/maproom-mcp/package.json
git commit -m "chore: bump version to 1.3.1-canary.1 for workflow test"
git tag v1.3.1-canary.1

# Step 4: Push tag (triggers workflow)
git push origin v1.3.1-canary.1
```

**Time**: ~60-90 minutes (workflow + testing)

---

## After Workflow Completes

### Test the Published Package

```bash
# Test in Docker (primary test)
docker run -it --rm ubuntu:latest bash

# Inside container:
apt-get update && apt-get install -y npm
npm install -g @crewchief/maproom-mcp@1.3.1-canary.1
npx @crewchief/maproom-mcp --version
# Should show: 1.3.1-canary.1 (or similar)
```

### Verify Package Contents

```bash
# Download and inspect
npm pack @crewchief/maproom-mcp@1.3.1-canary.1
tar -tzf crewchief-maproom-mcp-1.3.1-canary.1.tgz | grep bin/

# Should show all 4 binaries:
# package/bin/linux-x64/crewchief-maproom
# package/bin/linux-arm64/crewchief-maproom
# package/bin/darwin-x64/crewchief-maproom
# package/bin/darwin-arm64/crewchief-maproom
```

---

## Fill Out Test Report

As you execute the test, fill in:

**File**: `.crewchief/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`

Key sections:
- Record build times for each platform
- Note any errors or warnings
- Document binary sizes
- Record verification test results
- Add any issues found
- Include recommendations

---

## Success Criteria

**PASS** if:
- All 4 builds complete (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- Validation passes (all binaries present and valid)
- Publish succeeds (package on npm)
- Docker test passes (install and run)
- All 4 binaries in tarball

**FAIL** if:
- Any build fails
- Validation fails
- Publish fails
- Can't install/run package
- Missing binaries

---

## What Was Prepared for You

1. **Test Report Template**:
   - Location: `.crewchief/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`
   - Comprehensive template with all sections ready to fill in

2. **Manual Execution Guide**:
   - Location: `.crewchief/projects/BINPKG_binary-packaging/MANUAL_EXECUTION_GUIDE.md`
   - Detailed step-by-step instructions, troubleshooting, FAQ

3. **Prerequisites Verified**:
   - On main branch
   - All BINPKG tickets (1001-1007) present
   - Workflow file complete and ready
   - Current version: 1.3.0

---

## Need More Details?

- **Full instructions**: See `MANUAL_EXECUTION_GUIDE.md`
- **Test report template**: See `test-reports/canary-1.3.1-test-report.md`
- **Ticket details**: See `/workspace/.crewchief/work-tickets/BINPKG-1901_canary-release-integration-test.md`
- **Workflow file**: `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml`

---

## Quick Troubleshooting

### Workflow didn't trigger
- Check tag format: `v1.3.1-canary.1` (with 'v')
- Check tag was pushed: `git ls-remote --tags origin`

### Build failed
- Check job logs for errors
- Most common: toolchain timeout (retry)

### Publish failed
- Check NPM_TOKEN secret is set in repo
- Verify token has publish permissions

### Package won't install
- Wait 1-2 minutes for npm to propagate
- Check version: `npm view @crewchief/maproom-mcp versions`

---

**Ready to start? Follow Option 1 (Dry Run) first!**
