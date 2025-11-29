# CLIREL-7001 Execution Guide: Dry-Run Validation

## Overview

This ticket requires **manual execution** via GitHub Actions UI. The dry-run test cannot be fully automated because it requires:
1. Human interaction with GitHub Actions UI to trigger workflow_dispatch
2. Manual monitoring of workflow execution
3. Manual inspection of artifacts
4. Manual creation of validation report

## Prerequisites

Before executing this dry-run, ensure:
- ✅ CLIREL-4001 completed - CLI workflow exists
- ✅ CLIREL-5001 completed - MCP workflow updated for package-scoped tags
- ⚠️ CLIREL-6001 manual config - NPM_TOKEN **should** be configured (but dry-run skips publish, so not strictly required)

## Why Manual Execution?

The `dry_run` input parameter is only available via `workflow_dispatch` (manual trigger). When a tag is pushed, the workflow triggers automatically **without** the dry_run flag, which means it will attempt to publish to npm.

**Options:**
1. **workflow_dispatch with dry_run=true** ✅ RECOMMENDED - Tests workflow logic without publishing
2. **Tag push** ❌ NOT RECOMMENDED - Will attempt real npm publish (fails without NPM_TOKEN)

## Execution Steps

### Step 1: Navigate to GitHub Actions

1. Go to: https://github.com/danielbushman/crewchief/actions
2. Click on "Build and Publish CLI" workflow (left sidebar)
3. Click "Run workflow" button (top right)

### Step 2: Configure Workflow Run

In the workflow dispatch dialog:
- **Branch**: Select `main` (or current branch)
- **Dry run**: Check the box (set to `true`)
- Click "Run workflow"

### Step 3: Monitor Execution

**Job 1: build-binaries (Matrix)**
Watch for 4 parallel builds to complete:
- ✅ Build linux-x64 (ubuntu-latest)
- ✅ Build linux-arm64 (ubuntu-latest with cross-compilation)
- ✅ Build darwin-x64 (macos-13)
- ✅ Build darwin-arm64 (macos-latest)

Expected duration: ~8-12 minutes (parallel)

**Job 2: validate-and-publish**
Watch for validation steps:
- ✅ Download artifacts
- ✅ Validate binaries (existence, size 5-20MB, execution test)
- ✅ Organize package structure
- ✅ Install dependencies (pnpm install)
- ✅ Build TypeScript (pnpm build)
- ✅ Validate TypeScript output
- ✅ Create npm tarball
- ✅ Verify tarball structure
- ⏭️ **Publish to npm** (SKIPPED - dry_run=true)
- ⏭️ **Verify on registry** (SKIPPED - dry_run=true)
- ✅ **Dry run summary** (DISPLAYED - explains what was skipped)

Expected duration: ~2-3 minutes

**Total expected duration**: ~10-15 minutes

### Step 4: Inspect Workflow Run

**Via GitHub UI:**
1. Click on the running workflow
2. Expand each job to see detailed logs
3. Check for green checkmarks ✓ on all steps
4. Read "Dry run summary" step for confirmation

**Via CLI (optional):**
```bash
# List recent runs
gh run list --workflow=build-and-publish-cli.yml --limit 5

# Watch latest run
gh run watch

# View specific run (use ID from list)
gh run view <run-id> --log
```

### Step 5: Download and Inspect Artifacts

**Via GitHub UI:**
1. Scroll to bottom of workflow run page
2. Find "Artifacts" section
3. Download each artifact:
   - `cli-darwin-arm64`
   - `cli-darwin-x64`
   - `cli-linux-arm64`
   - `cli-linux-x64`

**Via CLI:**
```bash
# Download all artifacts from a run
gh run download <run-id>

# Or download specific artifact
gh run download <run-id> --name cli-linux-x64
```

**Inspect binaries:**
```bash
cd artifacts/

# List all binaries
find . -name "crewchief-maproom*" -ls

# Check sizes (should be 5-20MB each)
ls -lh cli-*/crewchief-maproom*

# Verify file types
file cli-darwin-arm64/crewchief-maproom
# Expected: Mach-O 64-bit executable arm64

file cli-linux-x64/crewchief-maproom
# Expected: ELF 64-bit LSB executable, x86-64
```

### Step 6: Create Validation Report

Copy the template below and fill in actual values:

```markdown
## Dry-Run Validation Report - CLIREL-7001

**Date**: 2025-11-08
**Workflow Run**: https://github.com/danielbushman/crewchief/actions/runs/<RUN_ID>
**Triggered By**: workflow_dispatch (manual)
**Dry Run**: ✅ Enabled

### Matrix Builds

| Platform | Status | Duration | Artifact Size |
|----------|--------|----------|---------------|
| linux-x64 | ✅ Success | X min Y sec | X.X MB |
| linux-arm64 | ✅ Success | X min Y sec | X.X MB |
| darwin-x64 | ✅ Success | X min Y sec | X.X MB |
| darwin-arm64 | ✅ Success | X min Y sec | X.X MB |

### Binary Validation

- ✅ All 4 binaries exist
- ✅ All sizes in range (5-20MB)
- ✅ File types correct:
  - darwin-arm64: Mach-O 64-bit executable arm64
  - darwin-x64: Mach-O 64-bit executable x86_64
  - linux-arm64: ELF 64-bit LSB executable, ARM aarch64
  - linux-x64: ELF 64-bit LSB executable, x86-64
- ✅ Execution test passed (linux-x64 --version)

### TypeScript Build

- ✅ pnpm install succeeded
- ✅ pnpm build succeeded
- ✅ dist/cli/index.js exists
- ✅ No build errors in logs

### Package Structure Validation

- ✅ npm pack created tarball
- ✅ Tarball contains all 4 platform binaries:
  - bin/crewchief (script)
  - bin/darwin-arm64/crewchief-maproom
  - bin/darwin-x64/crewchief-maproom
  - bin/linux-arm64/crewchief-maproom
  - bin/linux-x64/crewchief-maproom
- ✅ Tarball contains dist/cli/index.js
- ✅ Tarball contains README.md
- ✅ Tarball does NOT contain src/ files (verified with grep)

### Workflow Execution

- ✅ Publish to npm: **SKIPPED** (dry_run=true)
- ✅ Verify on registry: **SKIPPED** (dry_run=true)
- ✅ Dry run summary displayed
- ✅ Total duration: ~X minutes
- ✅ No errors in logs
- ✅ Artifacts available for 1 day

### Issues Found

[None / List any issues discovered]

### Recommendation

- ✅ **READY FOR PRODUCTION RELEASE** (proceed to CLIREL-8001)
- OR
- ❌ **FIX ISSUES BEFORE PRODUCTION** (list blockers)

### Notes

[Any additional observations, performance notes, or recommendations]

### Reviewer

**Name**: [Your name]
**Date**: 2025-11-08

---

**Artifacts will auto-delete after 1 day** - No manual cleanup needed
```

### Step 7: Document Results

Save the completed validation report to:
`/workspace/.crewchief/projects/CLIREL_cli-github-actions-release/tickets/CLIREL-7001_VALIDATION_REPORT.md`

## Success Criteria

For the dry-run to be considered successful:

### Build Success
- ✅ All 4 platform builds complete without errors
- ✅ All binaries in size range (5-20MB)
- ✅ File types correct (Mach-O for macOS, ELF for Linux)

### Validation Success
- ✅ Binary existence checks pass
- ✅ Binary size checks pass
- ✅ Binary execution test passes (linux-x64 --version)
- ✅ TypeScript build completes
- ✅ Package structure validation passes

### Workflow Success
- ✅ Publish step correctly skipped
- ✅ Dry run summary displayed
- ✅ No errors in workflow logs
- ✅ Total time under 15 minutes

## Common Issues and Solutions

| Issue | Likely Cause | Solution |
|-------|--------------|----------|
| Workflow not found | CLIREL-4001 not merged | Merge CLI workflow PR first |
| Build fails on darwin-arm64 | Rust compilation error | Check Cargo.toml, review build logs |
| Build fails on linux-arm64 | Cross-compilation issue | Check `cross` tool configuration |
| Binary size validation fails | Strip command didn't run | Check workflow strip step logs |
| TypeScript build fails | Missing dependencies | Check pnpm install step |
| Publish step runs (not skipped) | dry_run not set to true | Re-run with dry_run checkbox checked |
| Workflow times out | Builds taking too long | Check if matrix builds are parallel |

## What This Tests

### ✅ Tested in Dry-Run
- Tag-triggered workflow syntax (indirectly - we use workflow_dispatch but same jobs)
- Multi-platform matrix builds
- Binary cross-compilation (Linux ARM on x64 runner)
- Binary validation logic
- TypeScript build process
- Package structure validation
- Artifact upload/download
- Conditional logic (dry_run skips publish)

### ❌ NOT Tested in Dry-Run
- Actual npm publish (dry_run skips this)
- NPM_TOKEN authentication
- Post-publish registry verification
- Tag protection rules (no tag created)
- Real-world npm installation

## Next Steps After Successful Dry-Run

1. **Review validation report** - Ensure all checks passed
2. **Address any issues found** - Fix before production
3. **Complete CLIREL-6001 manual config** (if not done):
   - Configure NPM_TOKEN in GitHub secrets
   - Enable tag protection
   - Enable branch protection
   - Enable npm 2FA
4. **Proceed to CLIREL-8001** - First production release

## Alternative: Test with Real Tag (NOT RECOMMENDED)

If you want to test the tag-triggered flow (not recommended for dry-run):

```bash
# Create test tag
git tag @crewchief/cli@v1.0.0-test

# Push tag (triggers workflow WITHOUT dry_run)
git push origin @crewchief/cli@v1.0.0-test
```

**⚠️ WARNING**: This will attempt to publish to npm! Only do this if:
- NPM_TOKEN is configured
- You're okay with publishing v1.0.0-test to npm
- You understand this is NOT a dry-run

**Cleanup after tag test:**
```bash
# Delete local tag
git tag -d @crewchief/cli@v1.0.0-test

# Delete remote tag
git push origin :refs/tags/@crewchief/cli@v1.0.0-test

# Unpublish from npm (if it published)
npm unpublish @crewchief/cli@1.0.0-test
```

## Why workflow_dispatch Is Better

For dry-run testing, workflow_dispatch is superior:
- ✅ Can explicitly set dry_run=true
- ✅ No tag pollution (no test tags created)
- ✅ No risk of accidental publish
- ✅ Tests exact same jobs and steps
- ❌ Doesn't test automatic tag trigger (but tag pattern is simple, low risk)

The tag-triggered flow will be tested during the first real release (CLIREL-8001).
