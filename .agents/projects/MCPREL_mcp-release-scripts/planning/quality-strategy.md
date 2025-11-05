# Quality Strategy: MCP Release Scripts

## Testing Philosophy

**Pragmatic MVP approach**: Focus on confidence-building tests that prevent regressions and ensure core functionality works. Avoid ceremony and exhaustive testing.

User directive: "don't overdo it or overthink it"

## What Needs Testing

### Core Functionality
1. ✅ Version bumping (already tested in bump-version.js)
2. ✅ Git commit creation
3. ✅ Git tag creation
4. ✅ Git push operations
5. ✅ Error handling for failed operations

### What NOT to Test
- ❌ Git itself (well-tested external tool)
- ❌ File system operations (Node.js built-ins)
- ❌ Network reliability (not our concern)
- ❌ GitHub Actions integration (separate system)
- ❌ Edge cases that require mocking git (too complex)

## Testing Strategy

### Level 1: Manual Testing (Primary)
**Why**: This is a developer-facing script run manually. The developer IS the test.

**Approach**: Test on actual repository with real git operations.

**Test Cases**:
1. **Patch release**: `pnpm release:patch`
   - Verify version increments correctly
   - Check commit message format
   - Verify tag created with correct name
   - Confirm push to origin succeeds

2. **Minor release**: `pnpm release:minor`
   - Same verifications as patch

3. **Major release**: `pnpm release:major`
   - Same verifications as patch

4. **Error handling**: Intentionally cause failure
   - Run with no git credentials → Should fail with clear message
   - Create tag manually first → Should fail with clear message
   - Try to commit with nothing changed → Should handle gracefully

**Verification Commands**:
```bash
git log -1 --pretty=format:"%s"     # Check commit message
git tag -l | tail -1                # Check latest tag
git ls-remote --tags origin | tail -1  # Verify pushed
cat package.json | grep version     # Verify version bumped
```

### Level 2: Script Validation (Secondary)
**Why**: Catch syntax errors and basic logic issues.

**Approach**: Run scripts with various inputs to ensure they execute without crashing.

**Test Script** (create as `scripts/test-release.sh`):
```bash
#!/bin/bash
# Validation test for release scripts

set -e

echo "Testing bump-version.js..."
node scripts/bump-version.js patch > /dev/null
echo "✅ bump-version.js executes without errors"

echo "Testing release.js argument parsing..."
# Don't actually run release, just check it doesn't crash on startup
# Would require more complex mocking to test fully

echo "✅ All scripts validated"
```

**Decision**: Skip this. Manual testing is sufficient for this use case.

### Level 3: Unit Tests (Not Needed)
**Why Skip**:
- Scripts are thin wrappers around git commands
- Mocking git is complex and provides little value
- Manual testing is faster and more reliable
- Changes are infrequent (release automation doesn't change often)
- Developer can easily verify output

**If We Were to Unit Test** (We Won't):
Would need to mock:
- `fs.readFileSync` for package.json
- `child_process.execSync` for git commands
- Verify correct arguments passed

Too much ceremony for simple scripts.

## Confidence Building

### How to Gain Confidence
1. **Test on feature branch first**
   - Create test branch: `git checkout -b test-release-scripts`
   - Run `pnpm release:patch`
   - Verify all operations complete correctly
   - Delete branch after testing: `git branch -D test-release-scripts`
   - Delete test tag: `git tag -d vX.Y.Z && git push origin :vX.Y.Z`

2. **Check each operation manually**
   - After running script, verify:
     - `git log -1` shows correct commit
     - `git tag -l` shows new tag
     - `git ls-remote --tags origin` shows tag pushed
     - GitHub shows new tag in UI

3. **Document expected behavior**
   - Add examples to README or script comments
   - Show example output in documentation

## Error Handling Quality

### Script Should Handle
1. ✅ **Missing argument**: Print usage, exit with error
2. ✅ **Invalid argument**: Print valid options, exit with error
3. ✅ **Git command failure**: Print clear error, exit non-zero
4. ✅ **Version bump failure**: Propagate error, exit non-zero

### Script Should NOT Handle
1. ❌ **Dirty working directory**: Let git handle this naturally
2. ❌ **Uncommitted changes**: Developer responsibility
3. ❌ **Network failures during push**: Git provides clear errors
4. ❌ **Authentication failures**: Git provides clear errors

**Rationale**: Git already provides excellent error messages. Don't add layers of error handling that obscure useful details.

## Quality Checklist

### Before Implementation
- [x] Architecture is simple and focused
- [x] No unnecessary dependencies
- [x] Error handling strategy defined
- [x] Testing approach agreed upon

### During Implementation
- [ ] Script executes without syntax errors
- [ ] Each git command includes error checking
- [ ] Error messages are clear and actionable
- [ ] Script exits with appropriate exit codes

### Before Merge
- [ ] Manual test: patch release completes successfully
- [ ] Manual test: minor release completes successfully
- [ ] Manual test: major release completes successfully
- [ ] Manual test: error handling works (try invalid argument)
- [ ] Verify commit message format follows Conventional Commits
- [ ] Verify tag format is `vX.Y.Z`
- [ ] Verify both commit and tag are pushed to origin
- [ ] Documentation updated (if any)

## Test Data

### Safe Testing
Since this modifies git history, testing should be done carefully:

1. **Option 1: Feature Branch** (Recommended)
   - Test on branch, delete after
   - Clean up: `git push origin :refs/tags/vX.Y.Z` (delete remote tag)

2. **Option 2: Local Repository Clone**
   - Clone repo to temporary location
   - Test scripts there
   - Discard clone after testing

3. **Option 3: Dry Run First** (If We Implemented It)
   - Would show what would happen without doing it
   - Not implementing per "don't overdo it"

## Regression Prevention

### What Could Break
1. **Git command changes**: Unlikely, git is stable
2. **Package.json format changes**: Unlikely, well-defined format
3. **Node.js API changes**: Unlikely, using stable built-ins
4. **Working directory assumptions**: Test from different directories

### How to Prevent
- Keep scripts simple (less to break)
- Don't add features we don't need
- Test manually before each npm release
- Document expected usage

## Performance Considerations

### Speed
Not a concern. Script runs in < 1 second:
- Read file: instant
- Write file: instant
- Git operations: < 1 second
- Network push: depends on network, but non-blocking

### Optimization
None needed. This is not a hot path.

## Success Criteria

Quality is sufficient when:
1. ✅ Developer can run `pnpm release:patch` and it works
2. ✅ Version is bumped correctly
3. ✅ Git commit is created with correct message
4. ✅ Git tag is created with correct format
5. ✅ Both are pushed to origin
6. ✅ Errors are caught and reported clearly
7. ✅ Script can be run multiple times without issues (idempotent where possible)

## Maintenance Testing

### When to Re-test
- Before releasing new version (ironically, test the release script)
- If Node.js version changes significantly
- If git command-line interface changes (extremely rare)
- If package.json format changes (extremely rare)

### Future Testing (Not Now)
If project grows and needs more rigor:
- Add integration test suite
- Add smoke test in CI
- Create test fixture repository
- Implement dry-run mode for safer testing

But for now: **Manual testing is sufficient.**

## Comparison to Enterprise Testing

### What Enterprise Would Do (We Won't)
- Unit tests with mocked git commands
- Integration tests with test repositories
- End-to-end tests in CI
- Snapshot testing for git output
- Property-based testing
- Security scanning
- Performance benchmarking

### Why We're Not Doing That
- **Overkill**: Scripts are simple and change rarely
- **Ceremony**: More testing overhead than script complexity
- **Developer-facing**: Failures are immediately obvious
- **Low risk**: Worst case is bad git tag (easily fixed)
- **User directive**: "Don't overdo it"

## Summary

**Testing approach**: Manual testing on feature branch is sufficient.

**Quality gates**: Developer verifies output manually before merging.

**Confidence**: Comes from simplicity and manual verification, not exhaustive tests.

**Philosophy**: Ship working code, test what matters, skip ceremony.
