# Ticket: MCPREL-2001: Manual testing and validation of release scripts

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner (for executing tests)

## Summary
Manually test the new release script workflow on a feature branch to verify that version bumping, git commit, git tag, and git push all work correctly. Verify that GitHub Actions workflows trigger successfully and complete without errors.

## Background
MCPREL-1001 and MCPREL-1002 have implemented a new release workflow where `pnpm release:patch/minor/major` creates git commits and tags instead of directly publishing to npm. We need to verify this works end-to-end:

1. Scripts execute without errors
2. Version is bumped correctly
3. Git commit is created with correct message format
4. Git tag is created with correct format
5. Both are pushed to origin
6. GitHub Actions workflows trigger on tag push
7. Both workflows complete successfully

**Critical**: This test will trigger real GitHub Actions that publish to npm and Docker Hub. Test carefully on a feature branch or coordinate with project owner.

## Acceptance Criteria
- [ ] Test branch created for safe testing
- [ ] `pnpm release:patch` executes successfully from packages/maproom-mcp directory
- [ ] package.json version incremented correctly (e.g., 1.3.1 → 1.3.2)
- [ ] Git commit created with message: `chore(release): bump version to X.Y.Z`
- [ ] Git tag created with format: `vX.Y.Z` (e.g., v1.3.2)
- [ ] Both commit and tag pushed to origin successfully
- [ ] GitHub Actions workflow `build-and-publish-maproom-mcp.yml` triggers on tag push
- [ ] GitHub Actions workflow `publish-maproom-mcp-image.yml` triggers on tag push
- [ ] Both workflows complete successfully (green checkmarks in GitHub Actions UI)
- [ ] Error handling works: invalid argument (e.g., `pnpm release:invalid`) shows clear error
- [ ] Test results documented in this ticket

## Technical Requirements

### Test Environment Setup
```bash
# Create test branch
cd /workspace
git checkout -b test-mcprel-release-scripts

# Navigate to package
cd packages/maproom-mcp
```

### Test Execution
```bash
# Test 1: Successful patch release
pnpm release:patch

# Verify operations
git log -1 --pretty=format:"%s"        # Check commit message
git tag -l | tail -1                   # Check tag created
git show $(git describe --tags)        # Check tag details
```

### Verification Commands
After script runs, check:
```bash
# 1. Version was bumped
cat package.json | grep version

# 2. Commit message format
git log -1 --pretty=format:"%s"
# Expected: "chore(release): bump version to X.Y.Z"

# 3. Tag exists and matches version
git tag -l "v*" | tail -1
# Expected: vX.Y.Z

# 4. Tag is annotated
git show v1.3.2 --no-patch
# Should show tag message: "Release version X.Y.Z"

# 5. Push succeeded
git log origin/$(git branch --show-current) --oneline -1
git ls-remote --tags origin | grep v1.3.2

# 6. GitHub Actions triggered
gh run list --workflow=build-and-publish-maproom-mcp.yml --limit 3
gh run list --workflow=publish-maproom-mcp-image.yml --limit 3
```

### Monitor GitHub Actions
```bash
# Watch workflows
gh run watch <run-id>

# Or check in GitHub UI
# https://github.com/danielbushman/crewchief/actions
```

### Cleanup After Testing
```bash
# Delete test tag locally and remotely
git tag -d v1.3.2-test
git push origin :refs/tags/v1.3.2-test

# Reset branch if needed
git reset --hard HEAD~1

# Return to main branch
git checkout main
git branch -D test-mcprel-release-scripts
```

## Implementation Notes

### Testing Strategy
**Option 1: Test with pre-release version** (Safest)
- Modify package.json version to X.Y.Z-test.1 manually
- Run release script
- Tag will be v1.3.2-test.1 (won't match `v*.*.*` pattern exactly)
- Can delete from npm if published

**Option 2: Test on feature branch with real version** (Requires coordination)
- Use real version increment
- GitHub Actions will publish to npm and Docker Hub
- Coordinate with project owner first
- Clean up after test

**Option 3: Dry run first** (Not implemented)
- Would require adding --dry-run flag to release.js
- Not in scope for this ticket

### Error Test Cases
1. Invalid argument: `pnpm release:invalid`
   - Expected: Error message and exit code 1
2. No git repo: Run from /tmp
   - Expected: Git error message
3. Uncommitted changes: Make change, don't commit, run release
   - Expected: Git handles this (may warn or fail)

## Dependencies
- **BLOCKED BY**: MCPREL-1001 (release.js must be implemented)
- **BLOCKED BY**: MCPREL-1002 (package.json must be updated)

## Risk Assessment
- **Risk**: Test triggers real publish to npm and Docker Hub
  - **Mitigation**: Use test branch, consider pre-release version, coordinate with owner
  - **Impact**: Medium - published package can be unpublished if needed
- **Risk**: Test tag pollutes repository
  - **Mitigation**: Delete test tags after verification
  - **Impact**: Low - tags can be deleted easily
- **Risk**: GitHub Actions quota consumed
  - **Mitigation**: Test once, verify carefully before running
  - **Impact**: Low - minimal quota usage

## Files/Packages Affected
- **READ**: All files created in MCPREL-1001 and MCPREL-1002
- **EXECUTE**: `packages/maproom-mcp/scripts/release.js`
- **OBSERVE**: GitHub Actions workflows
- **DOCUMENT**: Test results in this ticket file

## Expected Results

### Successful Test Output
```
$ pnpm release:patch
Bumped version from 1.3.1 to 1.3.2
[main abc1234] chore(release): bump version to 1.3.2
 1 file changed, 1 insertion(+), 1 deletion(-)
Created tag v1.3.2
Pushing commit...
Pushing tag...
✅ Released version 1.3.2
```

### GitHub Actions
- Both workflows should appear in Actions tab within 30 seconds
- Build duration: ~10-15 minutes for binary builds + npm publish
- Docker build duration: ~10-15 minutes for multi-platform images
- Both should show green checkmarks when complete

## Documentation Requirements
After testing, update this ticket with:
- [ ] Actual test commands run
- [ ] Screenshots or logs of GitHub Actions workflows
- [ ] Any errors encountered and how they were resolved
- [ ] Confirmation that workflows completed successfully
- [ ] Links to published artifacts (npm, Docker Hub)

## Estimated Time
20-30 minutes (including GitHub Actions wait time)
