# Ticket: CLIREL-9001: Update Documentation and Archive Project

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update repository documentation to reflect the new `@crewchief/cli` package name and automated release process, create release workflow documentation for future maintainers, and archive the CLIREL project to mark completion.

## Background

### Why Documentation Matters
Future maintainers (including future you) will need:
- Clear installation instructions for new package name
- Migration guide for anyone still using old `crewchief` package
- Release process documentation (how to create releases)
- Troubleshooting guide for common issues
- Security incident response procedures

### What Gets Updated
- Repository README.md (installation, migration)
- Release process documentation (new or updated)
- Troubleshooting guide (common workflow issues)
- Project archived (knowledge transfer complete)

## Acceptance Criteria
- [x] Repository README.md updated with `@crewchief/cli` installation instructions
- [x] Migration guide added to README (or separate MIGRATION.md)
- [x] Release process documented (how future releases work)
- [x] Troubleshooting guide created (common workflow issues)
- [x] Security incident response documented (SECURITY.md already exists from Phase 6)
- [x] Project archived to `.crewchief/archive/projects/CLIREL_cli-github-actions-release/`
- [x] Archive README created with project summary
- [x] All tickets marked complete

## Technical Requirements

### 1. Update Repository README.md

**File**: `/workspace/README.md`

**Section to add/update: Installation**
```markdown
## Installation

### CLI Tool

Install the CrewChief CLI globally:

```bash
npm install -g @crewchief/cli
```

Verify installation:

```bash
crewchief --version
```

### Migrating from old `crewchief` package

If you previously installed `crewchief` (unscoped), uninstall it first:

```bash
npm uninstall -g crewchief
npm install -g @crewchief/cli
```

The package has been renamed to `@crewchief/cli` as of v1.0.0.
```

**Section to add: Release Process** (for maintainers)
```markdown
## Release Process

### CLI Package (`@crewchief/cli`)

1. Update version in `packages/cli/package.json`
2. Commit changes: `git commit -am "chore: bump CLI to vX.Y.Z"`
3. Push commit: `git push`
4. Run release script:
   ```bash
   cd packages/cli
   pnpm release:minor  # or :major, :patch
   ```
5. Script will:
   - Create tag: `@crewchief/cli@vX.Y.Z`
   - Push tag to GitHub
   - Trigger GitHub Actions workflow
6. Monitor workflow: https://github.com/OWNER/REPO/actions
7. Verify publication: `npm view @crewchief/cli@X.Y.Z`

### MCP Server (`@crewchief/maproom-mcp`)

Similar process with `@crewchief/maproom-mcp@vX.Y.Z` tags.

### Troubleshooting

**Workflow doesn't trigger**:
- Check tag format: `@crewchief/cli@v1.0.0` (exact)
- Verify tag pushed to remote: `git ls-remote --tags origin`

**Build fails**:
- Check workflow logs: GitHub Actions → failed run → logs
- Common issues: Rust compilation errors, missing dependencies

**Publish fails**:
- Check NPM_TOKEN secret is configured
- Verify npm account has publish permissions
- Check for npm registry outages: https://status.npmjs.org
```

### 2. Create MIGRATION.md (Optional)

**File**: `/workspace/MIGRATION.md`

**Content**:
```markdown
# Migration Guide: crewchief → @crewchief/cli

## Overview

The `crewchief` package has been renamed to `@crewchief/cli` as of v1.0.0.

## Why the Change?

- **Naming convention**: Aligns with org pattern (`@crewchief/*`)
- **Independent versioning**: CLI and MCP can release separately
- **Multi-platform binaries**: New workflow builds for all platforms

## Migration Steps

### 1. Uninstall Old Package

```bash
npm uninstall -g crewchief
```

### 2. Install New Package

```bash
npm install -g @crewchief/cli
```

### 3. Verify Installation

```bash
crewchief --version
```

## What Changed?

### Package Name
- Old: `crewchief`
- New: `@crewchief/cli`

### Binaries
- Old: Single platform (whoever ran release)
- New: All 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)

### Release Process
- Old: Manual local publish
- New: Automated GitHub Actions

## Breaking Changes

**v1.0.0**:
- Package name changed (requires reinstall)
- No functionality changes (same CLI commands)

## Support

**Old package** (`crewchief`):
- Deprecated as of v1.0.0
- No future updates
- Security fixes only (if critical)

**New package** (`@crewchief/cli`):
- Active development
- Regular updates
- Full platform support

## Questions?

Open an issue: https://github.com/OWNER/REPO/issues
```

### 3. Create RELEASE.md (Release Process Documentation)

**File**: `/workspace/RELEASE.md`

**Content**:
```markdown
# Release Process Documentation

## Overview

Both CLI and MCP packages use automated GitHub Actions workflows for releases.

## Prerequisites

- Write access to repository
- npm account with publish access to `@crewchief` scope
- 2FA enabled on npm account
- NPM_TOKEN configured in repository secrets

## Creating a Release

### 1. Prepare Release

```bash
# Ensure on main branch
git checkout main
git pull origin main

# Update version in package.json
cd packages/cli  # or packages/maproom-mcp
# Edit package.json, bump version

# Commit version bump
git commit -am "chore(cli): bump version to vX.Y.Z"
git push
```

### 2. Run Release Script

```bash
# CLI
cd packages/cli
pnpm release:minor  # or :major, :patch

# MCP
cd packages/maproom-mcp
pnpm release:minor
```

### 3. Monitor Workflow

```bash
# Via CLI
gh run watch

# Via UI
# https://github.com/OWNER/REPO/actions
```

### 4. Verify Publication

```bash
# CLI
npm view @crewchief/cli@X.Y.Z

# MCP
npm view @crewchief/maproom-mcp@X.Y.Z
```

### 5. Test Installation

```bash
# CLI
npm install -g @crewchief/cli@X.Y.Z
crewchief --version

# MCP
npm install @crewchief/maproom-mcp@X.Y.Z
```

## Workflow Details

### Trigger
- CLI: `@crewchief/cli@v*.*.*` tags
- MCP: `@crewchief/maproom-mcp@v*.*.*` tags

### Steps
1. Matrix build (4 platforms)
2. Build TypeScript
3. Validate binaries
4. Publish to npm
5. Verify publication

### Duration
- Typical: 10-15 minutes
- Maximum: 60 minutes (timeout)

## Troubleshooting

### Workflow Doesn't Trigger

**Check tag format**:
```bash
git ls-remote --tags origin | grep cli
# Should show: @crewchief/cli@vX.Y.Z
```

**Fix**: Delete and recreate tag with correct format

### Build Fails

**Check logs**:
```bash
gh run view <run-id> --log
```

**Common causes**:
- Rust compilation error → Fix Cargo.toml or source code
- Missing dependency → Update pnpm-lock.yaml
- Platform-specific issue → Check cross-compilation setup

### Publish Fails

**Check NPM_TOKEN**:
```bash
gh secret list
# Should show: NPM_TOKEN
```

**Check npm permissions**:
```bash
npm access ls-packages @crewchief
# Should show your account with write access
```

**Check npm status**:
- https://status.npmjs.org

### Rollback

**If package is broken**:
```bash
# Publish fixed version immediately
cd packages/cli
# Update version to X.Y.Z+1
pnpm release:patch

# OR deprecate broken version
npm deprecate @crewchief/cli@X.Y.Z "Broken release, use vX.Y.Z+1"
```

**Cannot unpublish** (npm policy):
- Can unpublish within 72 hours if zero downloads
- Better to publish fix than unpublish

## Security

### Incident Response

**If NPM_TOKEN compromised**:
1. Revoke token on npmjs.com immediately
2. Generate new token
3. Update GitHub secret
4. Check recent publications for tampering

**If malicious version published**:
1. Unpublish immediately (if within 72 hours)
2. OR deprecate with warning
3. Publish clean version
4. Issue security advisory
5. Investigate how token was compromised

### Best Practices

- Enable 2FA on npm account
- Rotate NPM_TOKEN annually
- Monitor npm downloads for spikes
- Review all workflow changes (CODEOWNERS enforced)
- Tag protection prevents unauthorized releases

## Monitoring

**First 24 hours after release**:
- Check npm downloads: npmjs.com/@crewchief/cli
- Monitor GitHub issues
- Watch for error reports
- Verify platform coverage (all 4 platforms work)

**Ongoing**:
- Weekly: Check for new issues
- Monthly: Review download trends
- Quarterly: Security audit
```

### 4. Archive Project

**Move project to archive**:
```bash
mv /workspace/.crewchief/projects/CLIREL_cli-github-actions-release \
   /workspace/.crewchief/archive/projects/CLIREL_cli-github-actions-release
```

**Create archive README**:

**File**: `/workspace/.crewchief/archive/projects/CLIREL_cli-github-actions-release/ARCHIVE_README.md`

**Content**:
```markdown
# CLIREL Project Archive

## Project Summary

**Name**: CLI GitHub Actions Release Automation
**Slug**: CLIREL
**Duration**: [Start Date] - [End Date]
**Status**: ✅ Completed

## Objective

Migrate `@crewchief/cli` package from manual local releases to automated GitHub Actions releases with multi-platform binary builds and independent versioning.

## Deliverables

- ✅ Old `crewchief` package deprecated
- ✅ Package renamed to `@crewchief/cli@1.0.0`
- ✅ GitHub Actions workflow for CLI builds
- ✅ Multi-platform binaries (4 platforms)
- ✅ MCP workflow updated for package-scoped tags
- ✅ Security baseline implemented
- ✅ Release automation working
- ✅ Documentation complete

## Key Outcomes

**Before**:
- Manual releases (4-step process)
- Single-platform binaries
- No validation
- Error-prone

**After**:
- Automated releases (1 command)
- All 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- Multi-layer validation
- Fail-safe workflows

## Technical Highlights

- Package-scoped tags prevent conflicts
- Two-step push fixes race condition
- Matrix builds for parallelism
- Comprehensive validation gates
- Dry-run testing capability

## Tickets Completed

1. CLIREL-1001: Deprecate old package
2. CLIREL-2001: Package configuration
3. CLIREL-3001: Release scripts + race condition fix
4. CLIREL-4001: CLI GitHub Actions workflow
5. CLIREL-5001: MCP workflow update
6. CLIREL-6001: Security baseline
7. CLIREL-7001: Dry-run validation
8. CLIREL-8001: Production release
9. CLIREL-9001: Documentation and archive

## Lessons Learned

**What Worked Well**:
- Copying proven MCP workflow pattern
- Comprehensive dry-run testing
- Package-scoped tags for clarity
- Two-step push solved race condition

**Challenges**:
- Cross-compilation setup for ARM
- Binary size validation ranges
- npm registry eventual consistency

**Future Improvements**:
- Reusable workflow to reduce duplication
- Automated changelog generation
- Binary signing (if needed)

## References

- Planning docs: `planning/`
- Tickets: `tickets/`
- Repository docs: `/workspace/README.md`, `/workspace/RELEASE.md`
- Security: `/workspace/SECURITY.md`

## Maintenance

**Owner**: [Team/Person]
**Ongoing**: Release process now operational
**Support**: See RELEASE.md for troubleshooting

## Archive Date

[YYYY-MM-DD]
```

## Implementation Notes

### Order of Operations
1. Update repository README.md
2. Create MIGRATION.md (optional but recommended)
3. Create RELEASE.md
4. Test documentation (follow instructions, verify they work)
5. Move project to archive
6. Create ARCHIVE_README.md
7. Mark all tickets complete
8. Commit all documentation changes

### Testing Documentation
- Follow installation steps in README
- Walk through release process in RELEASE.md
- Verify links work
- Check for typos and clarity

## Dependencies
- CLIREL-8001 (Production Release) - Must complete successfully
- All previous phases complete

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Documentation outdated | Medium | Review during each release |
| Instructions unclear | Low | Test by following them exactly |
| Missing edge cases | Low | Update as issues discovered |

## Files/Packages Affected
- `/workspace/README.md` (update)
- `/workspace/MIGRATION.md` (create)
- `/workspace/RELEASE.md` (create)
- `/workspace/.crewchief/archive/projects/CLIREL_*/ARCHIVE_README.md` (create)
- `/workspace/.crewchief/projects/CLIREL_*` (move to archive)

## Success Metrics
- README clearly explains new package name
- Migration guide helps users switch packages
- Release process documented for future releases
- Troubleshooting covers common scenarios
- Project successfully archived
- All knowledge transferred to permanent docs
