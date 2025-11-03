# Ticket: BINPKG-3001: Create automated release script with git workflow

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create `scripts/release.js` that automates version bumping, git commit, tagging, and pushing to trigger the CI workflow. This replaces the manual `scripts/bump-version.js` and integrates with the GitHub Actions pipeline.

## Background
Currently developers run `bump-version.js` and manually create/push tags. This is error-prone and doesn't integrate with our new CI workflow. The release script automates the entire flow: bump version → commit → tag → push, which triggers the GitHub Actions workflow (BINPKG-1001-1007) that builds and publishes.

Without this script, developers must:
1. Run `node scripts/bump-version.js <type>`
2. Manually inspect the changes
3. Manually commit: `git add . && git commit -m "chore(release): bump version to X.Y.Z"`
4. Manually create tag: `git tag vX.Y.Z`
5. Manually push: `git push && git push --tags`

Each manual step is an opportunity for errors (wrong commit message, forgotten tag push, etc.). The automated script ensures consistency and reliability, and provides proper rollback on failures.

## Acceptance Criteria
- [ ] Script exists at `scripts/release.js`
- [ ] Accepts argument: `patch`, `minor`, or `major`
- [ ] Validates preconditions:
  - [ ] Git working directory is clean
  - [ ] Current branch is `main` or `master`
  - [ ] npm whoami succeeds (npm credentials exist)
- [ ] Bumps version in `packages/maproom-mcp/package.json` using semver
- [ ] Creates git commit: `"chore(release): bump version to X.Y.Z"`
- [ ] Creates git tag: `vX.Y.Z`
- [ ] Pushes commit and tag: `git push --follow-tags`
- [ ] Prints success message with tag and workflow URL
- [ ] Supports `--dry-run` flag (show what would happen)
- [ ] Handles errors gracefully with rollback

## Technical Requirements

### File Location
- Create: `scripts/release.js` (Node.js, can use ES modules)

### Dependencies
- `semver`: For version calculation
- `execa`: For git command execution (better than child_process)
- `chalk` (optional): For colored output

### Usage
```bash
node scripts/release.js <patch|minor|major> [--dry-run]
```

### Version Bumping Logic
1. Read current version from `packages/maproom-mcp/package.json`
2. Use `semver.inc(currentVersion, bumpType)` to calculate new version
3. Write new version back to `package.json`
4. Preserve formatting and other fields in package.json

### Git Operations
1. **Check status**: `git status --porcelain`
   - Must be empty (no uncommitted changes)
2. **Check branch**: `git branch --show-current`
   - Must be `main` or `master`
3. **Check npm auth**: `npm whoami`
   - Must succeed (validates npm credentials)
4. **Commit**: `git add packages/maproom-mcp/package.json && git commit -m "chore(release): bump version to X.Y.Z"`
5. **Tag**: `git tag vX.Y.Z`
6. **Push**: `git push --follow-tags`

### Error Handling and Rollback
- Validate bump type is `patch`, `minor`, or `major`
- Check git working directory is clean before starting
- Check branch is `main` or `master`
- Check npm credentials exist
- If any git operation fails after commit:
  - Rollback commit: `git reset --hard HEAD~1`
  - Delete tag if created: `git tag -d vX.Y.Z`
  - Report error and manual fix instructions

### Dry Run Mode
- `--dry-run` flag shows what would happen without executing
- Print all commands that would run
- Show current version → new version
- Do not commit, tag, or push
- Still validate preconditions

### Output Messages
```
✓ Preconditions validated
  - Working directory clean
  - Branch: main
  - npm credentials: username@example.com

✓ Version bump: 1.2.3 → 1.3.0
✓ Git commit created
✓ Git tag created: v1.3.0
✓ Pushed to remote

SUCCESS: Release v1.3.0 published!

GitHub Actions workflow:
https://github.com/owner/repo/actions/workflows/build-and-publish-maproom-mcp.yml
```

## Implementation Notes

### Script Structure
```javascript
#!/usr/bin/env node
import { execa } from 'execa';
import semver from 'semver';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

// Parse arguments
// Validate preconditions
// Bump version
// Create commit
// Create tag
// Push (unless dry-run)
// Print success message
```

### Key Design Decisions

1. **Use execa for git commands**: More reliable than child_process, better error handling
2. **Print progress at each step**: Helps debugging and gives confidence script is working
3. **Add --dry-run support early**: Useful for testing and documentation
4. **Don't push on dry run**: Prevents accidental publishes
5. **Rollback on failure**: Use `git reset` and `git tag -d` to clean up after errors
6. **Allow branch override**: Optional `--branch` flag for flexibility

### Progress Indicators
Use chalk (or console.log) to show:
- Green checkmark (✓) for completed steps
- Red X (✗) for errors
- Yellow warning for dry-run mode
- Blue info for GitHub Actions URL

### Reference Materials
- **Existing script**: `scripts/bump-version.js` - Shows package.json manipulation patterns
- **GitHub workflow**: `.github/workflows/build-and-publish-maproom-mcp.yml` - Triggered by this script
- **Planning doc**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 3, Release automation)

### GitHub Actions Workflow URL
After successful push, print:
```
https://github.com/owner/repo/actions/workflows/build-and-publish-maproom-mcp.yml
```

User can click this to watch the build/publish progress.

### Example Usage
```bash
# Patch release (1.2.3 → 1.2.4)
node scripts/release.js patch

# Minor release (1.2.3 → 1.3.0)
node scripts/release.js minor

# Major release (1.2.3 → 2.0.0)
node scripts/release.js major

# Dry run (test without executing)
node scripts/release.js minor --dry-run
```

### Error Scenarios to Handle
1. **No bump type provided**: Show usage and exit
2. **Invalid bump type**: Show valid options (patch, minor, major)
3. **Dirty working directory**: Show `git status` and ask to commit/stash
4. **Wrong branch**: Show current branch and expected branch
5. **No npm credentials**: Show `npm login` instructions
6. **Push fails**: Show manual recovery commands

### Future Enhancements (Out of Scope)
- Add `--branch` flag to override branch check
- Add `--skip-npm-check` flag for testing
- Add changelog generation
- Add GitHub release creation

## Dependencies

### Prerequisite Tickets
- **BINPKG-1001**: GitHub Actions workflow structure (provides trigger for this script)
- **BINPKG-1007**: npm publish step (this script triggers the workflow that publishes)

### Blocks These Tickets
- **BINPKG-3002**: Package.json scripts (will add `pnpm release:patch` etc.)
- **BINPKG-5001**: Dry run testing (will test this script in dry-run mode)

## Risk Assessment

- **Risk**: Push fails after commit/tag created
  - **Likelihood**: Low (network issues, permissions)
  - **Impact**: Medium (requires manual fix)
  - **Mitigation**: Allow retry with same tag, document manual fix commands, script detects existing tag

- **Risk**: Branch name detection fails on some systems
  - **Likelihood**: Low (git branch --show-current is standard)
  - **Impact**: Low (can override with flag)
  - **Mitigation**: Allow override with `--branch` flag in future enhancement

- **Risk**: Script runs on wrong directory/worktree
  - **Likelihood**: Low (script uses relative paths from repo root)
  - **Impact**: Medium (could bump wrong package.json)
  - **Mitigation**: Validate package.json path exists, add safety check for repo root

- **Risk**: Multiple developers run script simultaneously
  - **Likelihood**: Low (coordination issue)
  - **Impact**: Medium (conflicting tags)
  - **Mitigation**: Git push will fail for second developer, document team coordination

## Files/Packages Affected

### Files to Create
- `/workspace/scripts/release.js` - Main release script

### Files to Reference (Read Only)
- `/workspace/scripts/bump-version.js` - Existing version bump script (for patterns)
- `/workspace/packages/maproom-mcp/package.json` - Target file for version bump
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Workflow triggered by this script

### Files to Potentially Deprecate
- `/workspace/scripts/bump-version.js` - Can deprecate after this script is tested

### Packages Affected
- `packages/maproom-mcp` - Version field will be modified

## Estimated Effort
**2-3 hours**

Breakdown:
- 30 min: Review existing bump-version.js and understand patterns
- 45 min: Implement core script (argument parsing, version bump, git operations)
- 30 min: Add precondition validation (clean working dir, branch check, npm auth)
- 30 min: Add error handling and rollback logic
- 30 min: Add dry-run mode
- 15 min: Add colored output and success message
- 15 min: Test script manually with dry-run and actual run

## Priority
**Medium** - Improves developer experience and reduces errors, but not blocking other work

## Related Tickets

### Depends On
- **BINPKG-1001**: GitHub Actions workflow structure (must exist for script to be useful)

### Blocks
- **BINPKG-3002**: Package.json scripts (will add convenience commands using this script)
- **BINPKG-5001**: Dry run testing (will test this script)

### Related
- **BINPKG-1007**: npm publish (workflow step triggered by this script)
- **BINPKG-2001**: Local binary validation (similar script structure patterns)

### Sequence
This is ticket 1 of Phase 3 in the BINPKG project:
1. **BINPKG-3001** (this ticket) - Automated release script
2. BINPKG-3002 - Package.json convenience scripts
3. BINPKG-5001 - Dry run testing

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 3, Release automation)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (Developer workflow section)

### External References
- **semver npm package**: https://www.npmjs.com/package/semver
- **execa npm package**: https://www.npmjs.com/package/execa
- **Git tag documentation**: https://git-scm.com/docs/git-tag
- **Git push --follow-tags**: https://git-scm.com/docs/git-push#Documentation/git-push.txt---follow-tags
