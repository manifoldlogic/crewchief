# Ticket: CLIREL-8001: Execute Production Release

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Execute the first production release of `@crewchief/cli@1.0.0` with automated GitHub Actions workflow. This is the culmination of all previous phases - the moment we ship the new scoped package with multi-platform binaries to npm.

## Background

### What This Ticket Does
- Creates the real production tag: `@crewchief/cli@v1.0.0`
- Triggers the full automated workflow
- Publishes to npm registry (not dry-run)
- Validates post-release
- Monitors for issues

### Prerequisites
All previous phases must be complete:
- ✅ Phase 1: Old package deprecated
- ✅ Phase 2: Package configured
- ✅ Phase 3: Release scripts updated
- ✅ Phase 4: CLI workflow created
- ✅ Phase 5: MCP workflow updated
- ✅ Phase 6: Security baseline implemented
- ✅ Phase 7: Dry-run successful

**This is irreversible** - once published to npm, cannot unpublish.

## Acceptance Criteria
- [x] Tag `@crewchief/cli@v1.0.0` created and pushed
- [x] GitHub Actions workflow completes successfully
- [x] Package `@crewchief/cli@1.0.0` published to npm
- [x] Package appears on npm registry (npmjs.com/@crewchief/cli)
- [x] Package contains all 4 platform binaries
- [x] Installation test passes: `npm install -g @crewchief/cli`
- [x] Execution test passes: `crewchief --version`
- [x] Post-release validation complete
- [x] Release monitoring setup (first 24 hours)

## Technical Requirements

### 1. Pre-Release Checklist

**Before creating tag, verify**:
```bash
# Check package.json
cd packages/cli
cat package.json | grep -E '"name"|"version"'
# Should show:
#   "name": "@crewchief/cli",
#   "version": "1.0.0",

# Check git status (should be clean)
git status
# Should show: nothing to commit, working tree clean

# Check on main branch
git branch --show-current
# Should show: main

# Pull latest
git pull origin main

# Verify dry-run passed (check ticket CLIREL-7001)
# Should have validation report showing success
```

### 2. Create Production Tag

**Using release script**:
```bash
cd packages/cli

# Run release script
pnpm release:major  # Creates v1.0.0

# Script will:
# 1. Bump version in package.json (already 1.0.0, no change)
# 2. Create commit (if version changed)
# 3. Create tag: @crewchief/cli@v1.0.0
# 4. Push commit: git push
# 5. Push tag: git push origin @crewchief/cli@v1.0.0
```

**OR manually** (if script issues):
```bash
# Create tag
git tag @crewchief/cli@v1.0.0

# Push commit first
git push

# Push tag separately (avoid race condition)
git push origin @crewchief/cli@v1.0.0
```

**Verify tag created**:
```bash
git ls-remote --tags origin | grep @crewchief/cli@v1.0.0
# Should show: refs/tags/@crewchief/cli@v1.0.0
```

### 3. Monitor Workflow Execution

**Via GitHub Actions UI**:
1. Go to repository → Actions
2. Find "Build and Publish CLI" workflow run
3. Watch progress in real-time

**Via CLI**:
```bash
# Watch latest run
gh run watch

# Get run URL
gh run list --workflow=build-and-publish-cli.yml --limit 1
gh run view <run-id> --web
```

**Critical checkpoints**:
- ✅ All 4 matrix builds succeed
- ✅ Binaries validated
- ✅ TypeScript built
- ✅ Package structure validated
- ✅ npm publish succeeds (this is the critical step)
- ✅ Post-publish verification succeeds

**If workflow fails**:
- Check logs for error message
- Fix issue
- Delete tag: `git push origin :refs/tags/@crewchief/cli@v1.0.0`
- Fix problem
- Re-run (create tag again)

### 4. Post-Publish Verification

**Wait for workflow completion** (workflow does this automatically):
```yaml
# Workflow includes:
- name: Verify publication
  run: |
    sleep 10  # Wait for npm registry
    npm view @crewchief/cli@1.0.0 version
    # Should output: 1.0.0
```

**Manual verification** (after workflow completes):
```bash
# Check package exists
npm view @crewchief/cli@1.0.0

# Should show package metadata:
# @crewchief/cli@1.0.0 | MIT | deps: X | versions: 1
```

### 5. Installation Testing

**Test on current platform**:
```bash
# Install globally
npm install -g @crewchief/cli@1.0.0

# Verify installation
which crewchief
# Should show: /usr/local/bin/crewchief (or similar)

# Test execution
crewchief --version
# Should show version

crewchief --help
# Should show help output
```

**Test on multiple platforms** (if possible):
- Test on macOS (both Intel and ARM if available)
- Test on Linux (x64 and/or ARM64)
- Verify binary executes on each platform

**Expected binary locations**:
```
/usr/local/lib/node_modules/@crewchief/cli/
├── bin/
│   ├── darwin-arm64/crewchief-maproom
│   ├── darwin-x64/crewchief-maproom
│   ├── linux-arm64/crewchief-maproom
│   └── linux-x64/crewchief-maproom
```

### 6. npm Registry Validation

**Check npm page**:
1. Go to https://www.npmjs.com/package/@crewchief/cli
2. Verify package shows v1.0.0
3. Check README renders correctly
4. Verify package metadata is correct

**Check tarball contents**:
```bash
# Download published package
npm pack @crewchief/cli@1.0.0

# Inspect contents
tar -tzf crewchief-cli-1.0.0.tgz | less

# Verify all platforms
tar -tzf crewchief-cli-1.0.0.tgz | grep "bin/"
# Should show all 4 platform directories

# Verify dist/ included
tar -tzf crewchief-cli-1.0.0.tgz | grep "dist/"

# Verify src/ excluded
tar -tzf crewchief-cli-1.0.0.tgz | grep "src/"
# Should show nothing (src/ excluded)
```

### 7. Post-Release Monitoring

**First 24 hours**:
- Monitor npm download counts (npmjs.com/@crewchief/cli)
- Watch GitHub issues for installation problems
- Check workflow runs (no accidental re-runs)
- Monitor for security advisories

**Create monitoring checklist**:
```markdown
## Post-Release Monitoring

**Day 1** (first 24 hours):
- [ ] npm downloads: X (check npmjs.com)
- [ ] GitHub issues: 0 new installation issues
- [ ] Security advisories: None
- [ ] Workflow status: No failed runs

**Day 3**:
- [ ] npm downloads: X
- [ ] Platform coverage: Verify installs on multiple platforms

**Week 1**:
- [ ] Migration from old package: Check if users found new package
- [ ] Deprecation effective: Old package showing warnings
```

## Implementation Notes

### Timeline
- Tag creation: 1 minute
- Workflow execution: 10-15 minutes
- npm publish + verification: 1 minute
- Installation testing: 5 minutes
- **Total**: ~20 minutes

### Point of No Return
Once `npm publish` succeeds, the package is live. Cannot unpublish unless <72 hours old (npm policy).

**If issues found after publish**:
- Publish v1.0.1 with fix immediately
- Document issue in GitHub release notes
- Optional: deprecate v1.0.0 (but v1.0.1 is better approach)

### Success Indicators
- ✅ Workflow completes with all green checks
- ✅ npm shows package published
- ✅ Installation works without errors
- ✅ CLI executes on test platform
- ✅ All 4 binaries present in package

### Failure Scenarios

| Scenario | Response |
|----------|----------|
| Workflow fails before publish | Fix issue, delete tag, retry |
| Publish fails (npm auth) | Check NPM_TOKEN, retry |
| Publish succeeds but package broken | Publish v1.0.1 fix immediately |
| Binary missing for platform | Critical - publish v1.0.1 with all binaries |
| Installation fails | Investigate, publish v1.0.1 if needed |

## Dependencies
- CLIREL-7001 (Dry-Run) - MUST complete successfully before this ticket
- All previous phases (1-6) - Must be complete

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Broken package published | High | Dry-run testing (Phase 7) catches most issues |
| npm publish fails | Medium | Workflow retries, clear error messages |
| Binary doesn't work on platform | Medium | Validation testing, can hotfix with v1.0.1 |
| Unexpected download spike | Low | Monitor first 24 hours |
| Old package deprecation missed | Low | Phase 1 handled this |

## Files/Packages Affected
- Git tags (@crewchief/cli@v1.0.0)
- npm registry (@crewchief/cli@1.0.0 published)
- GitHub Actions workflow runs

## Success Metrics
- Package published successfully: `@crewchief/cli@1.0.0`
- All 4 platform binaries work
- Installation successful on 2+ platforms
- Zero critical bugs in first 24 hours
- Download count > 0 (even if just us testing)
- Workflow completed in <15 minutes
- Post-release validation 100% pass
