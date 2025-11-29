# Ticket: CFGVER-6001: Version Bump and Package Preparation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Bump package version to 1.2.0 and prepare for npm release. This includes updating version in package.json, finalizing CHANGELOG.md, creating git tag, and verifying all pre-release quality gates are met.

## Background
Ready the package for npm release with proper version, changelog, and metadata. The version bump must be coordinated with git tagging and change log updates. All tests must pass and pre-release checklist must be complete before proceeding to npm publish.

Reference: `plan.md` lines 143-164 for Phase 6 release process.

## Acceptance Criteria
- [ ] Package version bumped to 1.2.0 in package.json
- [ ] CHANGELOG.md updated with release notes (from CFGVER-5003)
- [ ] Git tag created: v1.2.0
- [ ] All tests passing (unit + integration)
- [ ] No uncommitted changes in working directory
- [ ] Pre-release checklist complete and documented
- [ ] Package builds successfully (`npm run build`)

## Technical Requirements

**Version Bump Process:**

```bash
cd packages/maproom-mcp

# Update version in package.json
npm version patch  # 1.2.2 → 1.2.0

# Verify version
cat package.json | grep version

# Verify CHANGELOG.md is updated
cat CHANGELOG.md | head -20

# Create git tag
git tag -a v1.2.0 -m "Release v1.2.0: Config version management"

# Verify tag
git tag -l "v1.2.0"

# Push tag (DO NOT push yet, wait for approval)
# git push origin v1.2.0
```

**Pre-Release Checklist:**

All of these must be verified before version bump:
- [ ] All Phase 1-5 tickets complete
- [ ] All tests passing (unit + integration)
  - [ ] Unit tests: `npm test`
  - [ ] Integration tests: `npm run test:integration`
  - [ ] Coverage: `npm run test:coverage` >= 80%
- [ ] Manual testing complete (macOS + Linux)
  - [ ] First run on clean system
  - [ ] Update from version 1.2.2
  - [ ] Rollback on failure
  - [ ] Docker integration
- [ ] Documentation updated
  - [ ] README.md has Configuration Management section
  - [ ] TROUBLESHOOTING.md created
  - [ ] CHANGELOG.md has v1.2.0 entry
  - [ ] JSDoc comments added to config-manager.js
- [ ] Code reviewed and approved
  - [ ] All PRs merged to main
  - [ ] No outstanding code review comments
- [ ] No high-severity security issues
  - [ ] Path traversal prevention verified
  - [ ] Command injection prevention verified
  - [ ] File permissions hardened (0o600/0o700)
- [ ] CI pipeline green
  - [ ] GitHub Actions passing
  - [ ] No flaky tests

**Version Bump Validation:**

After version bump, verify:
```bash
# Version in package.json
cat packages/maproom-mcp/package.json | grep '"version"'

# Tag created
git tag -l "v1.2.0"

# CHANGELOG.md has entry
cat packages/maproom-mcp/CHANGELOG.md | grep "1.2.0"

# Clean working directory
git status
```

## Implementation Notes

**npm version Command:**
The `npm version patch` command:
- Updates version in package.json
- Creates git commit with message "1.2.0"
- Creates git tag v1.2.0 (if git repo clean)

**Git Tag Message:**
Use descriptive tag message:
```bash
git tag -a v1.2.0 -m "Release v1.2.0: Config version management

- Automatic configuration updates
- Version tracking with integrity checks
- Backup and rollback mechanisms
- Docker container management
- Fixes config drift issues (#123)"
```

**Pre-Release Checklist Documentation:**
Document checklist results in ticket or create:
`.crewchief/projects/CFGVER_config-version-management/pre-release-checklist.md`

**Rollback Plan:**
If issues discovered after version bump:
```bash
# Delete tag
git tag -d v1.2.0

# Reset version in package.json
git checkout packages/maproom-mcp/package.json

# Fix issues, repeat checklist
```

**Manual Testing Reference:**
See `quality-strategy.md` lines 186-197 for manual testing checklist.

## Dependencies
- ALL previous tickets complete (CFGVER-1001 through CFGVER-5004)

## Risk Assessment
- **Risk**: Premature release (tests not complete)
  - **Mitigation**: Verify all tests pass before version bump

- **Risk**: Breaking changes not documented
  - **Mitigation**: Review CHANGELOG.md for completeness

- **Risk**: Tag pushed but issues found
  - **Mitigation**: Don't push tag until npm publish succeeds (CFGVER-6002)

- **Risk**: Version bump conflicts with concurrent changes
  - **Mitigation**: Ensure main branch stable, no pending merges

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/package.json` (version bump)
- **Verify**: `packages/maproom-mcp/CHANGELOG.md` (release notes complete)
- **Create**: Git tag v1.2.0
- **Create**: `.crewchief/projects/CFGVER_config-version-management/pre-release-checklist.md` (optional documentation)
