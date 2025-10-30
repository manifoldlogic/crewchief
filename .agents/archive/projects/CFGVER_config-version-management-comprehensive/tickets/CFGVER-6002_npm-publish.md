# Ticket: CFGVER-6002: npm Publish and Release Notes

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- documentation-engineer
- verify-ticket
- commit-ticket

## Summary
Publish package to npm registry and create GitHub release with comprehensive release notes. This makes the new version available to users via `npx -y @crewchief/maproom-mcp@latest` and communicates changes through GitHub release notes.

## Background
Ship the new version to npm registry and communicate changes to users via GitHub release. The npm publish process requires registry access and proper package build. GitHub release provides user-facing documentation of what's new and how to upgrade.

Reference: `plan.md` lines 143-164 for Phase 6 release objectives.

## Acceptance Criteria
- [ ] Package built successfully (`npm run build`)
- [ ] Package published to npm registry
- [ ] `npm install @crewchief/maproom-mcp@1.2.0` works
- [ ] GitHub release created at correct tag (v1.2.0)
- [ ] Release notes comprehensive and accurate
- [ ] Release notes link to CHANGELOG.md and TROUBLESHOOTING.md
- [ ] Old versions deprecated if security issues found (if applicable)
- [ ] Git tag pushed to remote

## Technical Requirements

**npm Publish Process:**

```bash
cd packages/maproom-mcp

# Verify working directory clean
git status

# Verify version is correct
cat package.json | grep version

# Build package
npm run build

# Verify build succeeded
ls -la dist/

# Dry run (see what would be published)
npm publish --dry-run

# Review files to be published
# Should include: dist/, bin/, config/, README.md, package.json

# Publish to npm (requires npm registry access)
npm publish

# Verify package available
npm view @crewchief/maproom-mcp version
npm view @crewchief/maproom-mcp dist-tags

# Push git tag to remote
git push origin v1.2.0
```

**GitHub Release:**

Create release at: `https://github.com/{org}/{repo}/releases/new`

**Tag:** v1.2.0

**Title:** v1.2.0 - Config Version Management

**Body:**
```markdown
## What's New

This release adds automatic configuration version management to prevent config drift issues.

### Features

- ✅ **Automatic Config Updates** - Running `npx -y @crewchief/maproom-mcp@latest` now automatically updates cached configs
- ✅ **Version Tracking** - Explicit version markers replace brittle pattern detection
- ✅ **Safe Updates** - Automatic backup and rollback on failure
- ✅ **Docker Integration** - Graceful container management during updates
- ✅ **Clear Messaging** - Progress updates and actionable error messages

### Fixed

- Config drift causing MCP connection failures (#123)
- Stale cached configs after npm package updates

### Security Improvements

- Path traversal prevention in config file operations
- Command injection prevention in Docker operations
- File permission hardening (0o600 for config files, 0o700 for directories)

### Upgrade Instructions

1. Stop MCP server if running
2. Run: `npx -y @crewchief/maproom-mcp@latest`
3. Config will automatically update on first run

For troubleshooting, see: [TROUBLESHOOTING.md](packages/maproom-mcp/docs/TROUBLESHOOTING.md)

### What's Changed

**Full Changelog:** [CHANGELOG.md](packages/maproom-mcp/CHANGELOG.md)

### Breaking Changes

None - this is a backward-compatible patch release.

### Contributors

Thank you to everyone who reported config drift issues and helped test this release!
```

**npm Registry Requirements:**

Must have:
- npm account with publish access to @crewchief scope
- Two-factor authentication enabled
- Logged in via `npm login`

**Package Verification:**

After publish, verify:
```bash
# Check package page
npm view @crewchief/maproom-mcp

# Install globally and test
npx -y @crewchief/maproom-mcp@1.2.0

# Check version
npx @crewchief/maproom-mcp@1.2.0 --version
```

## Implementation Notes

**Pre-Publish Checks:**

1. **Verify CFGVER-6001 Complete**
   - Version bumped to 1.2.0
   - Git tag created
   - Pre-release checklist complete

2. **Verify Build**
   - Run `npm run build`
   - Check `dist/` directory contains compiled files
   - Verify no TypeScript errors

3. **Verify Package Contents**
   - Run `npm publish --dry-run`
   - Review file list to be published
   - Ensure no sensitive files included (.env, secrets)

**npm Publish Dry Run:**
Always run dry run first:
```bash
npm publish --dry-run

# Review output:
# - package: @crewchief/maproom-mcp@1.2.0
# - files: dist/, bin/, config/, README.md, package.json
# - tarball size: should be < 1MB
```

**GitHub Release Creation:**
1. Go to: https://github.com/{org}/{repo}/releases/new
2. Select tag: v1.2.0
3. Copy release notes from template above
4. Attach binaries if needed (not typical for npm packages)
5. Mark as "Latest release"
6. Publish release

**Post-Publish Verification:**
```bash
# Wait 1-2 minutes for npm registry propagation

# Verify package available
npm view @crewchief/maproom-mcp@1.2.0

# Test install
npm install -g @crewchief/maproom-mcp@1.2.0

# Test CLI
maproom-mcp --version
```

**Rollback Plan:**
If critical issues discovered immediately after publish:
```bash
# Deprecate version (can't unpublish after 24 hours)
npm deprecate @crewchief/maproom-mcp@1.2.0 "Critical bug, use 1.2.4"

# Publish hotfix as 1.2.4
npm version patch
npm publish
```

**Old Version Deprecation:**
If security issues found in old versions:
```bash
npm deprecate @crewchief/maproom-mcp@1.2.0 "Security issue, upgrade to 1.2.0+"
npm deprecate @crewchief/maproom-mcp@1.2.1 "Security issue, upgrade to 1.2.0+"
npm deprecate @crewchief/maproom-mcp@1.2.2 "Config drift issues, upgrade to 1.2.0+"
```

## Dependencies
- CFGVER-6001 (version bump complete, tag created)

## Risk Assessment
- **Risk**: npm registry issues during publish
  - **Mitigation**: Have rollback plan, can deprecate and republish if needed

- **Risk**: Breaking changes discovered post-release
  - **Mitigation**: Monitor GitHub issues (CFGVER-6003), prepare hotfix if needed

- **Risk**: Package doesn't work in user environments
  - **Mitigation**: Test in clean environment before publish, comprehensive testing in CFGVER-5004

- **Risk**: GitHub release notes inaccurate
  - **Mitigation**: Documentation-engineer reviews release notes for accuracy

## Files/Packages Affected
- **Publish**: `packages/maproom-mcp/` to npm registry
- **Create**: GitHub release at tag v1.2.0
- **Push**: Git tag v1.2.0 to remote
