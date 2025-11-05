# Ticket: MCPREL-1001: Create release.js script for git-based releases

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a new `release.js` script that orchestrates the release workflow: bump version, create git commit, create annotated tag, and push both to origin. This replaces the current direct npm publishing approach with a git-tag-triggered CI/CD workflow.

## Background
Current release scripts in `packages/maproom-mcp/package.json` call `bump-version.js` and then immediately run `pnpm publish --no-git-checks`, bypassing git integration. With GitHub Actions workflows now in place to handle building and publishing on tag push, we need to change the release scripts to only create and push git tags. The existing `bump-version.js` script works well and should be reused.

This ticket implements the first component of the MCP Release Scripts project, which transitions from manual npm publishing to automated CI/CD-driven releases triggered by git tags.

## Acceptance Criteria
- [ ] Script accepts command-line argument: `patch`, `minor`, or `major`
- [ ] Validates input and exits with error for invalid arguments
- [ ] Calls existing `bump-version.js` to increment version
- [ ] Reads new version from package.json after bump
- [ ] Creates git commit with message format: `chore(release): bump version to X.Y.Z`
- [ ] Creates annotated git tag with format: `vX.Y.Z` and message: `Release version X.Y.Z`
- [ ] Pushes commit to origin (current branch)
- [ ] Pushes tag to origin
- [ ] Provides clear console output for each step
- [ ] Exits with non-zero code and clear error message if any operation fails

## Technical Requirements
1. **File location**: `/workspace/packages/maproom-mcp/scripts/release.js`
2. **Module type**: ESM (import/export) to match existing scripts
3. **Node.js built-ins only**: Use `fs`, `path`, `child_process.execSync`
4. **Working directory**: All git commands should run from package root (`packages/maproom-mcp/`)
5. **Error handling**: Wrap git commands in try-catch, exit process on failure
6. **Git commands sequence**:
   ```javascript
   // After bump-version.js runs:
   execSync('git add package.json', { cwd: packageRoot, stdio: 'inherit' });
   execSync(`git commit -m "chore(release): bump version to ${version}"`, { cwd: packageRoot, stdio: 'inherit' });
   execSync(`git tag -a v${version} -m "Release version ${version}"`, { cwd: packageRoot, stdio: 'inherit' });
   execSync('git push origin HEAD', { cwd: packageRoot, stdio: 'inherit' });
   execSync(`git push origin v${version}`, { cwd: packageRoot, stdio: 'inherit' });
   ```

## Implementation Notes
- Reuse bump-version.js by spawning it as child process using `execSync('node scripts/bump-version.js ${type}', { cwd: packageRoot })`
- Read package.json after version bump to get new version number
- Use `execSync` with `stdio: 'inherit'` so git output appears in terminal
- Package root is `path.join(__dirname, '..')` relative to scripts/ directory (or use `fileURLToPath` like bump-version.js does)
- Commit message follows Conventional Commits: `chore(release): bump version to X.Y.Z`
- Tag format matches GitHub Actions trigger pattern: `v*.*.*` (e.g., `v1.2.3`)
- Annotated tags include release message for clarity
- Push commit first, then tag (logical sequence)
- Separate pushes are clearer than `--follow-tags` for this use case
- Script should be executable with shebang: `#!/usr/bin/env node`

**Example execution flow**:
```bash
node scripts/release.js minor
# 1. Runs bump-version.js minor
# 2. Reads new version (e.g., 1.2.0)
# 3. git add package.json
# 4. git commit -m "chore(release): bump version to 1.2.0"
# 5. git tag -a v1.2.0 -m "Release version 1.2.0"
# 6. git push origin HEAD
# 7. git push origin v1.2.0
```

## Dependencies
- None (first ticket in Phase 1)
- Requires existing `bump-version.js` (already present at `/workspace/packages/maproom-mcp/scripts/bump-version.js`, no changes needed)

## Risk Assessment
- **Risk**: Git command failures (network, auth, conflicts)
  - **Mitigation**: Use try-catch blocks around all git operations, provide clear error messages, let git's native errors show through with `stdio: 'inherit'`
- **Risk**: Version read fails after bump
  - **Mitigation**: Validate package.json structure after reading, fail fast with clear message if version field missing or malformed
- **Risk**: User runs script with uncommitted changes that conflict
  - **Mitigation**: Document expected usage (clean working tree), let git's native conflict messages appear naturally

## Files/Packages Affected
- **CREATE**: `/workspace/packages/maproom-mcp/scripts/release.js`
- **READ**: `/workspace/packages/maproom-mcp/scripts/bump-version.js` (to understand interface and reuse pattern)
- **READ**: `/workspace/packages/maproom-mcp/package.json` (to get version after bump)
