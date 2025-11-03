git # Manual Execution Guide for BINPKG-1901
# Canary Release Integration Test

## IMPORTANT: This Test Requires GitHub Actions Access

This ticket (BINPKG-1901) tests the GitHub Actions workflow end-to-end with a real canary release. It **cannot be fully automated** because it requires:

1. Triggering GitHub Actions workflows
2. Monitoring workflow execution in GitHub web UI
3. Access to npm registry to verify publication
4. Installing and testing the published package

This guide explains what has been prepared automatically and what you must do manually.

---

## What Has Been Prepared (Automated)

The following has been completed and is ready:

- [x] **Prerequisites verified**: All BINPKG tickets (1001-1007) are present and committed
- [x] **Current branch confirmed**: You are on `main` branch
- [x] **Current version identified**: Package is at version `1.3.0`
- [x] **Workflow file verified**: `.github/workflows/build-and-publish-maproom-mcp.yml` is complete
- [x] **Test report template created**: `/workspace/.agents/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`
- [x] **This guide created**: You're reading it!

---

## What You Must Do Manually

### RECOMMENDED: Start with Dry Run Test

This is the safest approach - test the workflow WITHOUT publishing to npm first.

#### Step 1: Navigate to GitHub Actions
1. Open your browser
2. Go to: https://github.com/danielbushman/crewchief/actions
3. Find workflow: "Build and Publish Maproom MCP"
4. Click "Run workflow" button (top right)

#### Step 2: Configure Dry Run
1. Select branch: `main` (or your current branch)
2. Check the box for "Dry run (skip publish)": **true**
3. Click green "Run workflow" button

#### Step 3: Monitor Workflow
1. Wait a few seconds, refresh the page
2. Click on the running workflow (should appear at top of list)
3. Watch all 5 jobs:
   - Build linux-x64 (should take ~5-10 min)
   - Build linux-arm64 (should take ~5-10 min)
   - Build darwin-x64 (should take ~5-10 min)
   - Build darwin-arm64 (should take ~5-10 min)
   - Validate and Publish (runs after builds, ~2-3 min)

#### Step 4: Check Results
If all jobs show green checkmarks:
- Workflow is working correctly
- Ready to proceed with real canary release
- Continue to "Full Canary Release Test" section below

If any job fails:
- Click on the failed job
- Expand the failed step
- Copy error messages to test report
- Create fix tickets as needed (BINPKG-19XX series)
- Fix issues and retry dry run
- **Do NOT proceed to real release until dry run passes**

---

### Full Canary Release Test (After Dry Run Passes)

Only proceed if the dry run test succeeded.

#### Step 1: Create Test Branch

```bash
cd /workspace
git checkout -b test/canary-release
```

#### Step 2: Bump Version to Canary

Edit `/workspace/packages/maproom-mcp/package.json`:

Change line 3 from:
```json
  "version": "1.3.0",
```

To:
```json
  "version": "1.3.1-canary.1",
```

Save the file.

#### Step 3: Commit Changes

```bash
git add packages/maproom-mcp/package.json
git commit -m "chore: bump version to 1.3.1-canary.1 for workflow test"
```

#### Step 4: Create and Push Tag

```bash
# Create tag
git tag v1.3.1-canary.1

# Push tag to trigger workflow
git push origin v1.3.1-canary.1

# Optional: Also push branch for reference
git push origin test/canary-release
```

**IMPORTANT**: The workflow will trigger automatically when the tag is pushed!

#### Step 5: Monitor Workflow

1. Immediately go to: https://github.com/danielbushman/crewchief/actions
2. You should see a new workflow run starting (triggered by tag push)
3. Click on it to watch progress
4. Monitor all 5 jobs (same as dry run, but this time publish will happen)

Fill in the test report (`canary-1.3.1-test-report.md`) as you go:
- Record start times
- Note any warnings or errors
- Copy build times from job logs
- Record binary sizes

#### Step 6: Wait for Workflow to Complete

Total expected time: 20-30 minutes

- 4 build jobs run in parallel: ~10 minutes
- Validation and publish job: ~5 minutes
- npm propagation: ~5 minutes

#### Step 7: Verify npm Publication

Once the "Validate and Publish" job shows green:

1. Wait 1-2 minutes for npm to propagate
2. Check npm registry: https://www.npmjs.com/package/@crewchief/maproom-mcp
3. You should see version `1.3.1-canary.1` listed
4. Note: It may be tagged as "latest" or "canary"

---

### Verification Testing

After the package is published to npm, test installation and functionality.

#### Test 1: Docker Installation (Primary Test)

This verifies the linux-x64 binary works correctly:

```bash
# Start clean Ubuntu container
docker run -it --rm ubuntu:latest bash

# Inside the container, run these commands:
apt-get update && apt-get install -y npm curl
npm install -g @crewchief/maproom-mcp@1.3.1-canary.1
npx @crewchief/maproom-mcp --version
npx @crewchief/maproom-mcp --help
```

**Expected output**:
- Installation succeeds
- `--version` shows: `1.3.1-canary.1` (or similar)
- `--help` shows usage information

**Record results** in test report under "Phase 3: Verification Testing > Test 1"

#### Test 2: Package Inspection

Outside the container, inspect the package contents:

```bash
# Download package tarball
npm pack @crewchief/maproom-mcp@1.3.1-canary.1

# List binary files in tarball
tar -tzf crewchief-maproom-mcp-1.3.1-canary.1.tgz | grep bin/

# Expected output (all 4 platforms):
# package/bin/linux-x64/crewchief-maproom
# package/bin/linux-arm64/crewchief-maproom
# package/bin/darwin-x64/crewchief-maproom
# package/bin/darwin-arm64/crewchief-maproom

# Extract and check sizes
tar -xzf crewchief-maproom-mcp-1.3.1-canary.1.tgz
ls -lah package/bin/*/crewchief-maproom
```

**Record results** in test report under "Phase 3: Verification Testing > Test 2"

#### Test 3: Binary Execution Test

Test the linux-x64 binary directly:

```bash
chmod +x package/bin/linux-x64/crewchief-maproom
./package/bin/linux-x64/crewchief-maproom --version
./package/bin/linux-x64/crewchief-maproom --help
```

**Record results** in test report under "Phase 3: Verification Testing > Test 3"

#### Test 4: macOS Testing (Optional)

If you have access to a Mac, test there too:

```bash
npm install -g @crewchief/maproom-mcp@1.3.1-canary.1
npx @crewchief/maproom-mcp --version
```

This verifies darwin-x64 or darwin-arm64 binary selection works correctly.

**Record results** in test report under "Phase 3: Verification Testing > Test 4"

---

### Complete the Test Report

Fill in all sections of: `/workspace/.agents/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`

Key sections to complete:
1. **Prerequisites Verification** - Check off completed items
2. **Phase 1: Setup** - Record what you did
3. **Phase 2: Workflow Monitoring** - Record times, sizes, issues
4. **Phase 3: Verification Testing** - Record test results
5. **Issues Found** - Document any problems
6. **Fixes Applied** - If you had to fix anything
7. **Performance Metrics** - Fill in times and sizes
8. **Recommendations** - Suggest improvements
9. **Conclusion** - Overall assessment
10. **Test Execution Log** - Chronological timeline

---

## Success Criteria

The test is **SUCCESSFUL** if:

- [x] All 4 build jobs complete successfully
- [x] Validation job passes (all binaries present and valid)
- [x] Publish job succeeds (package on npm)
- [x] Package installs in Docker (linux-x64 test)
- [x] Binary executes and shows correct version
- [x] All 4 binaries are in the tarball
- [x] No manual intervention was needed during workflow

The test **FAILS** if:

- Any build job fails
- Validation fails (missing/invalid binaries)
- Publish fails (npm errors)
- Package won't install
- Binary won't execute
- Missing binaries in tarball

---

## If Issues Are Found

1. **Don't panic** - This test is designed to find issues!

2. **Document thoroughly**:
   - Copy error messages
   - Note which job/step failed
   - Record context (what was happening)
   - Save to test report

3. **Create fix tickets**:
   - Use BINPKG-19XX numbering
   - One ticket per distinct issue
   - Link to test report

4. **Fix and retry**:
   - Fix the issue in workflow/code
   - Increment canary version (e.g., `1.3.1-canary.2`)
   - Repeat the test with new version
   - Keep all canary versions published

5. **Update test report**:
   - Document fixes applied
   - Show before/after for fixes
   - Verify fixes worked

---

## Time Estimate

**Dry run test**: 30-45 minutes
- 5 min: Trigger and start monitoring
- 25-30 min: Workflow execution
- 5-10 min: Review results

**Full canary test**: 60-90 minutes
- 10 min: Setup and tag push
- 30 min: Workflow execution
- 20 min: Verification testing
- 20-30 min: Complete test report

**If issues found**: Add 1-3 hours for fixes and retesting

---

## What Happens Next

### If Test PASSES:
1. Complete test report
2. Mark ticket BINPKG-1901 as complete
3. Proceed to Phase 5 tickets:
   - BINPKG-5001: Document dry run testing
   - BINPKG-5002: Execute first production release

### If Test FAILS:
1. Complete test report with issues documented
2. Create fix tickets (BINPKG-19XX series)
3. Work on fixes
4. Retry canary test with incremented version
5. Repeat until test passes
6. Then proceed to Phase 5

---

## Important Notes

### About Canary Versions
- Canary versions are pre-releases: `1.3.1-canary.1`, `1.3.1-canary.2`, etc.
- They are published to npm and available for users to test
- They don't affect the "latest" tag (usually)
- Production version `1.3.1` will supersede all canary versions
- Keep canary versions published - they're useful for testing

### About the Test Branch
- Branch `test/canary-release` is temporary
- Don't merge it to main
- Can delete after test completes
- Tags are permanent (keep them for history)

### About NPM_TOKEN
- Must be configured as GitHub secret
- Check repository Settings > Secrets and Variables > Actions
- Must have publish access to @crewchief scope
- If token is missing/invalid, publish job will fail

### About Workflow Triggers
- Workflow triggers on ANY tag matching `v*.*.*`
- This includes canary tags like `v1.3.1-canary.1`
- Manual dispatch requires repository write access
- Dry run mode prevents accidental publishes

---

## Troubleshooting

### "Workflow didn't trigger"
- Check tag format: Must be `v1.3.1-canary.1` (with 'v' prefix)
- Check tag was pushed to GitHub: `git ls-remote --tags origin`
- Check workflow file is on the branch with the tag
- Wait 1-2 minutes and refresh Actions page

### "Build job failed"
- Check error in job logs
- Common issues:
  - Rust toolchain installation failed (retry usually works)
  - Cross tool installation timeout (retry)
  - Build dependencies missing (workflow may need update)
  - Out of disk space (rare)

### "Validation failed - binary not found"
- Check artifact upload/download names match
- Check artifact paths in workflow
- Verify all build jobs completed successfully

### "Publish failed - authentication"
- Verify NPM_TOKEN secret is set
- Check token hasn't expired
- Verify token has publish permissions for @crewchief scope
- May need to regenerate token

### "Package installs but binary missing"
- Check package.json "files" array includes "bin"
- Check bin/ directory structure matches workflow
- Download and inspect tarball manually

### "Binary won't execute"
- Check file permissions (should be executable)
- Check binary is for correct platform
- Linux: Check GLIBC compatibility
- macOS: Check architecture (x64 vs arm64)

---

## Reference

### Files Involved
- **Workflow**: `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml`
- **Package**: `/workspace/packages/maproom-mcp/package.json`
- **Test Report**: `/workspace/.agents/projects/BINPKG_binary-packaging/test-reports/canary-1.3.1-test-report.md`
- **Ticket**: `/workspace/.agents/work-tickets/BINPKG-1901_canary-release-integration-test.md`

### Commands Summary
```bash
# Dry run (recommended first)
# 1. Go to GitHub Actions UI
# 2. Click "Run workflow"
# 3. Select "dry_run: true"

# Full canary test
git checkout -b test/canary-release
# Edit package.json version to 1.3.1-canary.1
git add packages/maproom-mcp/package.json
git commit -m "chore: bump version to 1.3.1-canary.1 for workflow test"
git tag v1.3.1-canary.1
git push origin v1.3.1-canary.1

# Wait for workflow, then test
docker run -it --rm ubuntu:latest bash
# Inside container:
apt-get update && apt-get install -y npm
npm install -g @crewchief/maproom-mcp@1.3.1-canary.1
npx @crewchief/maproom-mcp --version

# Inspect package
npm pack @crewchief/maproom-mcp@1.3.1-canary.1
tar -tzf crewchief-maproom-mcp-1.3.1-canary.1.tgz | grep bin/
```

---

## Questions or Issues?

If you encounter problems not covered in this guide:

1. Check the test report template for additional guidance
2. Review workflow logs in GitHub Actions
3. Check npm package page for publication status
4. Review planning documents in `.agents/projects/BINPKG_binary-packaging/planning/`

---

**Good luck with the test!**

This is the critical validation step before production releases. Take your time, document everything, and don't hesitate to create fix tickets for any issues found.
