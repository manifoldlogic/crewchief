# Ticket: BINPKG-5001: Execute dry-run release test

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- test-runner
- verify-ticket
- commit-ticket

## Summary
Test the complete release workflow with dry-run mode (no actual changes) to verify all components work together before the canary release. This is the first real test of the entire system including validation, version bumping logic, git operations, and script integration.

## Background
Before attempting a canary release (BINPKG-1901) or production release (BINPKG-5002), we need to verify the release script works correctly in dry-run mode. The release script (BINPKG-3001) automates the critical workflow: version bump → commit → tag → push. This ticket tests all those components without making actual changes to ensure they're working correctly.

The dry-run test will verify:
- **Validation checks**: Clean working directory, correct branch, npm authentication
- **Version calculations**: Semver bump logic (patch, minor, major)
- **Git operations**: Commit message generation, tag creation, push simulation
- **Error handling**: Proper failure messages when validation fails
- **Script integration**: All components work together as expected

If this test reveals issues, we'll create bug tickets and fix them before proceeding to the canary release. This is a quality gate to prevent wasting time on a broken release process.

## Acceptance Criteria
- [ ] Create test branch: `test/binpkg-dry-run`
- [ ] Run patch dry-run: `node scripts/release.js patch --dry-run`
  - [ ] Current version detected correctly
  - [ ] New patch version calculated (e.g., 1.3.1 → 1.3.2)
  - [ ] All validation checks pass
  - [ ] Commit message shown: "chore(release): bump version to X.Y.Z"
  - [ ] Tag shown: vX.Y.Z
  - [ ] Push command shown with --follow-tags
  - [ ] GitHub Actions workflow URL displayed
- [ ] Run minor dry-run: `node scripts/release.js minor --dry-run`
  - [ ] New minor version calculated (e.g., 1.3.1 → 1.4.0)
  - [ ] All validation checks pass
- [ ] Run major dry-run: `node scripts/release.js major --dry-run`
  - [ ] New major version calculated (e.g., 1.3.1 → 2.0.0)
  - [ ] All validation checks pass
- [ ] Verify NO actual changes made:
  - [ ] `package.json` still has original version
  - [ ] No git commit created
  - [ ] No git tag created
  - [ ] Nothing pushed to remote
  - [ ] `git status` shows clean or only test branch changes
- [ ] Test validation failures:
  - [ ] Dirty working directory → script exits with error
  - [ ] Wrong branch (not main/master) → script exits with error
  - [ ] Invalid bump type → script exits with error
- [ ] Document test results with output examples

## Technical Requirements

### Test Environment Setup
1. **Create test branch from main**:
   ```bash
   git checkout main
   git pull
   git checkout -b test/binpkg-dry-run
   ```

2. **Ensure clean working directory** before each test:
   ```bash
   git status  # Should show "nothing to commit, working tree clean"
   ```

### Commands to Execute
```bash
# Test patch release
node scripts/release.js patch --dry-run

# Test minor release
node scripts/release.js minor --dry-run

# Test major release
node scripts/release.js major --dry-run
```

### Validation Tests to Run

#### Test 1: Dirty Working Directory Detection
```bash
# Create a dummy file
echo "test" > test-file.txt
git status  # Should show untracked file

# Attempt release - should fail
node scripts/release.js patch --dry-run

# Expected: Error message about dirty working directory
# Clean up
rm test-file.txt
```

#### Test 2: Wrong Branch Detection
```bash
# Create and switch to feature branch
git checkout -b feature/test-validation

# Attempt release - should fail
node scripts/release.js patch --dry-run

# Expected: Error message about wrong branch (not main/master)
# Switch back
git checkout test/binpkg-dry-run
git branch -D feature/test-validation
```

#### Test 3: Invalid Bump Type
```bash
# Use invalid bump type
node scripts/release.js invalid --dry-run

# Expected: Error message showing valid options (patch, minor, major)
```

### Verification Commands
After each successful dry-run:
```bash
# Check package.json version unchanged
cat packages/maproom-mcp/package.json | grep '"version"'

# Check no new commits
git log -1

# Check no new tags
git tag | tail -5

# Check nothing staged or committed
git status
```

### Expected Output Pattern
The dry-run should print output similar to:
```
🔍 DRY RUN MODE - No changes will be made

✓ Preconditions validated
  - Working directory clean
  - Branch: main
  - npm credentials: username@example.com

✓ Version bump: 1.3.1 → 1.3.2

Would create commit:
  git commit -m "chore(release): bump version to 1.3.2"

Would create tag:
  git tag v1.3.2

Would push:
  git push --follow-tags

GitHub Actions workflow:
  https://github.com/owner/repo/actions/workflows/build-and-publish-maproom-mcp.yml

DRY RUN COMPLETE - No changes were made
```

## Implementation Notes

### Test Execution Strategy
1. **Manual testing**: This is not automated - run commands manually and observe output
2. **Test branch**: Use `test/binpkg-dry-run` to isolate from main branch
3. **Clean state**: Start each test from a clean working directory
4. **Actual project**: Test on real project structure, not mock/fixture

### Test Documentation
Create a test results document at `.crewchief/projects/BINPKG_binary-packaging/testing/dry-run-results.md`:
- Include command output (copy/paste from terminal)
- Include verification command output
- Note any issues or unexpected behavior
- Include screenshots if helpful
- Document npm authentication check result

### If Dry-Run Fails
If the dry-run test reveals issues:
1. **Document the failure**: Exact error message and reproduction steps
2. **Create bug ticket**: BINPKG-5001-BUG-X with details
3. **Assign to general-purpose agent**: Fix the issue
4. **Re-run dry-run test**: After fixes applied
5. **Don't proceed to canary**: Until dry-run passes completely

### Success Criteria
The test is successful when:
- All three bump types (patch, minor, major) work in dry-run
- No actual changes made to repository
- All validation checks work correctly (fail when they should)
- Output is clear and informative
- Test results documented thoroughly

### Key Things to Verify
1. **Version calculation accuracy**: Check semver math is correct
2. **No side effects**: Absolutely nothing changed in repo
3. **Clear messaging**: Output is easy to understand
4. **Error messages**: Validation failures provide helpful guidance
5. **Script safety**: Dry-run flag is respected everywhere

### Reference Materials
- **Release script**: `/workspace/scripts/release.js` (created in BINPKG-3001)
- **Package.json scripts**: `/workspace/packages/maproom-mcp/package.json` (updated in BINPKG-3002)
- **Planning doc**: `.crewchief/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 5, Testing)

## Dependencies

### Prerequisite Tickets
- **BINPKG-3001**: Automated release script (the script being tested)
- **BINPKG-3002**: Package.json scripts (provides npm run commands)
- **BINPKG-2001**: Local binary validation script (similar validation patterns)

### Blocks These Tickets
- **BINPKG-1901**: Canary release integration test (don't proceed until dry-run passes)
- **BINPKG-5002**: Production release (final gate before production)

### Related Tickets
- **BINPKG-2901**: Test local validation script (similar testing approach)
- **BINPKG-4001**: Document release process (will reference dry-run test results)

## Risk Assessment

- **Risk**: Dry-run accidentally makes changes despite flag
  - **Likelihood**: Low (explicit flag check in script)
  - **Impact**: Medium (would need to revert changes)
  - **Mitigation**: Test on branch, verify no changes after each test, use `git status` and `git diff`

- **Risk**: Test doesn't reveal hidden issues in release script
  - **Likelihood**: Medium (dry-run can't test everything)
  - **Impact**: Medium (issues found during canary release)
  - **Mitigation**: Follow with canary release test (BINPKG-1901) for real execution

- **Risk**: Validation checks pass incorrectly (false positives)
  - **Likelihood**: Low (validation logic is straightforward)
  - **Impact**: High (could allow broken releases)
  - **Mitigation**: Test both success and failure cases, verify error messages

- **Risk**: npm authentication check fails in test environment
  - **Likelihood**: Medium (depends on local npm config)
  - **Impact**: Low (doesn't affect dry-run functionality)
  - **Mitigation**: Document npm login requirement, test checks npm auth correctly

## Files/Packages Affected

### Files to Create
- `/workspace/.crewchief/projects/BINPKG_binary-packaging/testing/dry-run-results.md` - Test results document

### Files to Read/Execute
- `/workspace/scripts/release.js` - Script being tested
- `/workspace/packages/maproom-mcp/package.json` - Version source

### Files to Verify Unchanged
- `/workspace/packages/maproom-mcp/package.json` - Should not be modified
- All git state (commits, tags, branches)

### Packages Affected
- `packages/maproom-mcp` - Version checked but not modified

## Estimated Effort
**1-2 hours**

Breakdown:
- 10 min: Setup test branch
- 20 min: Run dry-run tests (patch, minor, major)
- 20 min: Run validation failure tests (dirty dir, wrong branch, invalid type)
- 15 min: Verify no changes made
- 20 min: Document test results with output examples
- 10 min: Clean up test branch

## Priority
**High** - This is a critical quality gate before canary release. Must pass before proceeding to BINPKG-1901.

## Related Tickets

### Depends On
- **BINPKG-3001**: Automated release script (must exist to test)
- **BINPKG-3002**: Package.json scripts (provides npm commands)
- **BINPKG-2001**: Local binary validation script (validation patterns)

### Blocks
- **BINPKG-1901**: Canary release integration test (don't proceed until this passes)
- **BINPKG-5002**: Production release (final gate)

### Related
- **BINPKG-2901**: Test local validation script (similar testing methodology)
- **BINPKG-4001**: Document release process (will include dry-run test results)

### Sequence
This is the testing ticket for Phase 3 (Release Automation) in the BINPKG project:
1. BINPKG-3001 - Create release script
2. BINPKG-3002 - Add package.json scripts
3. **BINPKG-5001** (this ticket) - Test release script in dry-run mode
4. BINPKG-1901 - Canary release (only after this passes)
5. BINPKG-5002 - Production release

## Reference Documentation

### Planning Documents
- **Project plan**: `.crewchief/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 5, Testing)
- **Architecture**: `.crewchief/projects/BINPKG_binary-packaging/planning/architecture.md` (Release workflow)

### External References
- **semver documentation**: https://docs.npmjs.com/about-semantic-versioning
- **Git dry-run patterns**: https://git-scm.com/docs/git-push#Documentation/git-push.txt---dry-run
- **Testing practices**: Focus on both positive and negative test cases
