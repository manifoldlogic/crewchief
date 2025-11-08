# Ticket: CLIREL-7001: Execute Dry-Run Validation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (dry-run workflow executed successfully)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Execute end-to-end dry-run test of the complete automation workflow without publishing to production npm. This validates that all components work together before the first real release.

## Background

### Why Dry-Run is Critical
This is our last chance to catch issues before production:
- Workflow syntax errors
- Binary build failures
- Validation logic bugs
- Package structure problems
- npm publish failures

**Better to fail in dry-run than in production.**

### What Gets Tested
- Tag creation and push
- GitHub Actions workflow trigger
- All 4 platform binary builds
- TypeScript build
- Binary validation (existence, size)
- Package structure validation
- Everything EXCEPT actual npm publish

### What Does NOT Get Tested
- Actual npm publish (dry_run flag skips this)
- Post-publish verification (no package to verify)
- User installation (no published package)

## Acceptance Criteria
- [x] Test tag created: N/A - Used workflow_dispatch instead (safer for dry-run)
- [x] Workflow triggered successfully via `gh workflow run` with dry_run=true
- [x] All 4 platform builds completed (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- [x] All binaries validated (existence, size checks passed)
- [x] TypeScript build completed successfully
- [x] Package structure validation passed
- [x] Publish step skipped (dry_run=true)
- [x] Artifacts available for inspection (1-day retention)
- [x] Test tag and artifacts cleaned up: N/A (no tag created, artifacts auto-expire)
- [x] Validation report documented (CLIREL-7001_VALIDATION_REPORT.md)

## Technical Requirements

### 1. Create Test Tag

**Using release script**:
```bash
cd packages/cli

# Temporarily modify package.json version to 1.0.0-test
# Or manually create tag:
git tag @crewchief/cli@v1.0.0-test
git push origin @crewchief/cli@v1.0.0-test
```

**Important**:
- Use `-test` suffix to avoid confusion with real releases
- Tag should match trigger pattern: `@crewchief/cli@v*.*.*`
- Must push tag (local tag won't trigger workflow)

### 2. Trigger Workflow with Dry-Run

**Option A: Use test tag** (recommended):
```bash
git tag @crewchief/cli@v1.0.0-test
git push origin @crewchief/cli@v1.0.0-test
# Workflow triggers automatically, but need to manually set dry_run=true in workflow
```

**Option B: Manual trigger**:
1. Go to GitHub Actions
2. Select "Build and Publish CLI" workflow
3. Click "Run workflow"
4. Set dry_run: true
5. Click "Run workflow"

**Option A is better** because it tests the full tag-triggered flow.

### 3. Monitor Workflow Execution

**Via GitHub UI**:
1. Go to repository → Actions tab
2. Find "Build and Publish CLI" run
3. Watch matrix builds complete (4 parallel jobs)
4. Watch validate-and-publish job

**Via CLI**:
```bash
# Start watching latest workflow run
gh run watch

# Or get specific run
gh run list --workflow=build-and-publish-cli.yml
gh run view <run-id> --log
```

**What to monitor**:
- Matrix builds: All 4 should succeed
- Binary sizes: Should be in 5-20MB range
- TypeScript build: Should complete without errors
- Validation: All checks should pass (green ✓)
- Publish: Should be skipped with message "Skipping publish (dry_run=true)"

### 4. Inspect Artifacts

**Download artifacts**:
```bash
# Via GitHub UI
# Actions → Run → Artifacts section → Download

# Via CLI
gh run download <run-id>
```

**Verify artifact contents**:
```bash
cd artifacts/

# Check all 4 binaries exist
ls -lh binary-darwin-arm64/
ls -lh binary-darwin-x64/
ls -lh binary-linux-arm64/
ls -lh binary-linux-x64/

# Check sizes
du -h binary-*/crewchief-maproom-*
# Should be 5-20MB each

# Verify executables
file binary-darwin-arm64/crewchief-maproom-darwin-arm64
# Should show: Mach-O 64-bit executable arm64

file binary-linux-x64/crewchief-maproom-linux-x64
# Should show: ELF 64-bit LSB executable, x86-64
```

### 5. Validation Report

**Create validation report** (document in ticket or separate file):
```markdown
## Dry-Run Validation Report

**Date**: YYYY-MM-DD
**Tag**: @crewchief/cli@v1.0.0-test
**Workflow Run**: https://github.com/OWNER/REPO/actions/runs/RUN_ID

### Matrix Builds
- ✅ linux-x64: Success (duration: Xm Ys)
- ✅ linux-arm64: Success (duration: Xm Ys)
- ✅ darwin-x64: Success (duration: Xm Ys)
- ✅ darwin-arm64: Success (duration: Xm Ys)

### Binary Validation
- ✅ All 4 binaries exist
- ✅ Sizes in range (5-20MB):
  - darwin-arm64: X.XMB
  - darwin-x64: X.XMB
  - linux-arm64: X.XMB
  - linux-x64: X.XMB
- ✅ File types correct (Mach-O for darwin, ELF for linux)

### TypeScript Build
- ✅ dist/ directory created
- ✅ dist/cli/index.js exists
- ✅ No build errors

### Package Structure
- ✅ Tarball created successfully
- ✅ bin/ directories included for all platforms
- ✅ dist/ directory included
- ✅ README.md and LICENSE included
- ✅ src/ files excluded (not in tarball)

### Workflow Execution
- ✅ Publish step skipped (dry_run=true)
- ✅ Total duration: X minutes
- ✅ No errors in logs
- ✅ Artifacts available for 1 day

### Issues Found
- None / [List any issues]

### Recommendation
- ✅ Ready for production release
- OR ❌ Fix issues before production
```

### 6. Cleanup

**Delete test tag**:
```bash
# Delete local tag
git tag -d @crewchief/cli@v1.0.0-test

# Delete remote tag
git push origin :refs/tags/@crewchief/cli@v1.0.0-test

# Verify deletion
git ls-remote --tags origin | grep test
# Should return nothing
```

**Artifacts auto-delete**:
- GitHub Actions deletes after 1 day (configured in workflow)
- No manual cleanup needed

## Implementation Notes

### Expected Duration
- Matrix builds: ~8-12 minutes (parallel)
- Validate-and-publish: ~2-3 minutes
- **Total**: ~10-15 minutes

### What Success Looks Like
```
✅ Build linux-x64 completed
✅ Build linux-arm64 completed
✅ Build darwin-x64 completed
✅ Build darwin-arm64 completed
✅ Binaries validated
✅ TypeScript built
✅ Package structure validated
⏭️  Publish skipped (dry_run=true)
```

### What Failure Looks Like
```
❌ Build darwin-arm64 failed
   Error: cargo build failed
   Exit code: 101
```

### Common Issues and Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| Workflow doesn't trigger | Tag pattern mismatch | Use exact format: `@crewchief/cli@v*.*.*` |
| Binary build fails | Rust compilation error | Check Cargo.toml, fix build errors |
| Size validation fails | Binary too large | Check if strip command ran |
| TypeScript build fails | Missing dependencies | Check pnpm install step |
| Publish not skipped | dry_run not set | Trigger manually with dry_run=true |

## Dependencies
- CLIREL-4001 (CLI Workflow) - Must be complete and merged
- CLIREL-6001 (Security) - Should be complete (NPM_TOKEN configured)

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Dry-run finds critical bug | Medium | This is the point - fix before production |
| Workflow times out | Low | 60-minute timeout should be sufficient |
| Artifact inspection reveals issues | Medium | Better to find now than in production |
| Test tag conflicts with real release | Low | Use `-test` suffix, cleanup after |

## Files/Packages Affected
- Git tags (temporary test tag)
- GitHub Actions (workflow execution)
- Artifacts (temporary, auto-deleted)

## Success Metrics
- All 4 binaries build successfully
- Validation passes (all green checks)
- Package structure correct
- Publish step skipped
- Total time <15 minutes
- Zero errors in workflow logs
- Ready to proceed to production release
