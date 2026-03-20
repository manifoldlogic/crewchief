# Release Process Documentation

## Overview

Both CLI and MCP packages use automated GitHub Actions workflows for releases. This document covers the complete release process for maintainers.

## Prerequisites

Before you can create releases, ensure you have:

- ✅ Write access to the repository
- ✅ npm account with publish access to `@crewchief` scope
- ✅ 2FA enabled on npm account
- ✅ `NPM_TOKEN` configured in repository secrets (already done)
- ✅ Familiarity with semantic versioning

## Quick Reference

```bash
# CLI Release
cd packages/cli
pnpm release:minor  # Bumps minor version (X.Y.0 → X.Y+1.0)
# or
pnpm release:major  # Bumps major version (X.Y.Z → X+1.0.0)
# or
pnpm release:patch  # Bumps patch version (X.Y.Z → X.Y.Z+1)

# MCP Release
cd packages/maproom-mcp
pnpm release:minor  # Same versioning scheme
```

That's it! The GitHub Actions workflow handles everything else.

## Creating a Release

### Step 1: Prepare the Release

```bash
# Ensure you're on main branch
git checkout main
git pull origin main

# Update version in package.json
cd packages/cli  # or packages/maproom-mcp

# Edit package.json and increment version
# Example: "version": "1.0.0" → "version": "1.1.0"

# Commit version bump
git add package.json
git commit -m "chore(cli): bump version to v1.1.0"
git push origin main
```

### Step 2: Run Release Script

```bash
# For CLI
cd packages/cli
pnpm release:minor  # Creates tag and pushes

# For MCP
cd packages/maproom-mcp
pnpm release:minor  # Creates tag and pushes
```

The release script will:
1. Create a git tag: `@crewchief/cli@v1.1.0`
2. Push commits to remote (if any)
3. Push the tag to remote
4. Trigger the GitHub Actions workflow automatically

### Step 3: Monitor Workflow

The workflow takes ~10-15 minutes to complete.

**Via GitHub UI**:
1. Go to https://github.com/manifoldlogic/crewchief/actions
2. Find "Build and Publish CLI" (or MCP) workflow run
3. Watch progress in real-time

**Via CLI**:
```bash
# Watch latest workflow run
gh run watch

# View specific run
gh run list --workflow=build-and-publish-cli.yml --limit 1
gh run view <run-id> --web
```

**What happens during the workflow**:
1. ✅ Matrix build (4 platforms in parallel)
   - linux-x64 (~2-3 min)
   - linux-arm64 (~6-7 min, cross-compilation)
   - darwin-x64 (~3-4 min)
   - darwin-arm64 (~2-3 min)
2. ✅ Binary validation (size, existence, execution tests)
3. ✅ TypeScript build
4. ✅ Package structure validation
5. ✅ Publish to npm
6. ✅ Post-publish verification

### Step 4: Verify Publication

Once the workflow completes:

```bash
# Check package on npm registry
npm view @crewchief/cli@1.1.0

# Should show:
# - Correct version number
# - Package metadata
# - Recent publish time
```

### Step 5: Test Installation

```bash
# Test global installation
npm install -g @crewchief/cli@1.1.0

# Verify version
crewchief --version
# Should output: 1.1.0

# Test basic functionality
crewchief --help
```

## Workflow Details

### Trigger Patterns

The workflows are triggered by package-scoped tags:

- **CLI**: `@crewchief/cli@v*.*.*`
- **MCP**: `@crewchief/maproom-mcp@v*.*.*`

Example tags:
```
@crewchief/cli@v1.0.0
@crewchief/cli@v1.2.3
@crewchief/maproom-mcp@v2.0.0
```

### Workflow Steps

#### 1. Matrix Build (Parallel)

Builds Rust binaries for all 4 platforms simultaneously:

| Platform | Runner | Typical Duration | Notes |
|----------|--------|------------------|-------|
| linux-x64 | ubuntu-latest | 2-3 min | Native build |
| linux-arm64 | ubuntu-latest | 6-7 min | Cross-compiled with `cross` |
| darwin-x64 | macos-13 | 3-4 min | Intel Mac runner |
| darwin-arm64 | macos-latest | 2-3 min | Apple Silicon runner |

#### 2. Binary Validation

Each binary is validated for:
- ✅ File existence
- ✅ Size (5-20MB range)
- ✅ File type (Mach-O for macOS, ELF for Linux)
- ✅ Execution test (`--version` flag)

#### 3. TypeScript Build

- Install dependencies with pnpm
- Build with `pnpm build`
- Validate `dist/cli/index.js` exists

#### 4. Package Validation

- Create tarball with `npm pack`
- Verify tarball contents:
  - All 4 platform binaries present
  - TypeScript dist/ included
  - Source files excluded (not in package)

#### 5. Publish to npm

```bash
npm publish --access public
```

Authentication handled via `NPM_TOKEN` secret.

#### 6. Post-Publish Verification

```bash
npm view @crewchief/cli@X.Y.Z
```

Confirms package is visible on registry.

### Duration

- **Typical**: 10-12 minutes (limited by slowest platform build)
- **Maximum**: 60 minutes (workflow timeout)
- **Average**: ~8-10 minutes for CLI, ~5-7 minutes for MCP

## Troubleshooting

### Workflow Doesn't Trigger

**Symptom**: You pushed a tag but no workflow run appears

**Check tag format**:
```bash
git ls-remote --tags origin | grep cli
# Should show: refs/tags/@crewchief/cli@v1.1.0
```

**Fix**: Delete incorrect tag and recreate with exact format
```bash
# Delete remote tag
git push origin :refs/tags/@crewchief/cli@v1.1.0

# Delete local tag
git tag -d @crewchief/cli@v1.1.0

# Recreate with correct format
git tag @crewchief/cli@v1.1.0
git push origin @crewchief/cli@v1.1.0
```

### Build Fails

**Symptom**: One or more platform builds fail

**Check workflow logs**:
```bash
gh run view <run-id> --log
```

**Common causes**:

| Error | Cause | Solution |
|-------|-------|----------|
| Cargo build failed | Rust compilation error | Fix code in `crates/maproom/` |
| Missing dependency | pnpm-lock.yaml out of sync | Run `pnpm install` and commit |
| Binary too large | Debug symbols not stripped | Check `strip` command in workflow |
| Cross-compilation fail | `cross` tool issue | Update `cross` version or config |

**Fix and retry**:
1. Fix the issue in code
2. Commit and push fix
3. Delete the failed tag
4. Re-run release script

### Publish Fails

**Symptom**: Builds succeed but npm publish fails

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
- Visit https://status.npmjs.org
- Ensure npm registry is operational

**Common errors**:

| Error Code | Cause | Solution |
|------------|-------|----------|
| ENEEDAUTH | Missing/invalid NPM_TOKEN | Regenerate token, update secret |
| EPUBLISHCONFLICT | Version already published | Bump version number |
| E403 | No publish permission | Contact npm org admin |
| ETIMEOUT | npm registry slow/down | Wait and retry |

### Post-Publish Verification Fails

**Symptom**: Package published but verification step fails

**Cause**: npm registry eventual consistency

**Solution**: This is usually transient. The package is published, just not immediately visible. Wait 30-60 seconds and verify manually:

```bash
npm view @crewchief/cli@X.Y.Z
```

## Rollback and Hotfixes

### Publishing a Hotfix

If a broken version is published:

```bash
# Immediately publish a fixed version
cd packages/cli

# Bump patch version
# Edit package.json: "1.1.0" → "1.1.1"

git commit -am "fix: hotfix for v1.1.0 issue"
git push

pnpm release:patch  # Publishes v1.1.1
```

**DO NOT** try to unpublish - it's better to publish a fix.

### Deprecating a Broken Version

```bash
npm deprecate @crewchief/cli@1.1.0 "Broken release, use v1.1.1 instead"
```

Users will see deprecation warnings when installing v1.1.0.

### Unpublishing (Last Resort)

npm's unpublish policy:
- Can only unpublish within 72 hours
- Only if package has zero downloads
- Requires npm support for popular packages

**Better approach**: Publish a fixed version immediately.

## Security

### NPM_TOKEN Management

The `NPM_TOKEN` secret provides publish access to `@crewchief` packages.

**Best practices**:
- ✅ Enable 2FA on npm account (automation tokens work with 2FA)
- ✅ Use automation tokens (not your personal token)
- ✅ Rotate token annually
- ✅ Audit npm package access regularly

**If NPM_TOKEN is compromised**:

1. **Immediately** revoke the token on npmjs.com:
   - Go to npm → Access Tokens
   - Delete the compromised token

2. Generate a new automation token:
   - npm → Access Tokens → Generate New Token
   - Select "Automation" type
   - Copy token

3. Update GitHub secret:
   ```bash
   gh secret set NPM_TOKEN
   # Paste new token when prompted
   ```

4. Check recent publications:
   ```bash
   npm view @crewchief/cli versions --json
   npm view @crewchief/maproom-mcp versions --json
   ```

5. If unauthorized versions published:
   - Unpublish if within 72 hours
   - Or deprecate with security warning
   - Issue security advisory
   - Publish clean versions

### Tag Protection

Repository has tag protection enabled:

- Tags matching `@crewchief/*@v*.*.*` require push access
- Prevents unauthorized releases
- Configured in repository settings

### Code Review

The `.github/CODEOWNERS` file requires review for:
- Workflow changes (`.github/workflows/`)
- Release scripts (`packages/*/scripts/release.*`)

All workflow modifications require approval.

## Monitoring

### First 24 Hours After Release

Check these metrics:

- ✅ **npm downloads**: Visit npmjs.com/package/@crewchief/cli
- ✅ **GitHub issues**: Watch for installation problems
- ✅ **Workflow runs**: Ensure no accidental re-triggers
- ✅ **Security advisories**: Check npm security alerts

### Ongoing Monitoring

- **Weekly**: Review GitHub issues for bugs or feature requests
- **Monthly**: Check download trends and platform coverage
- **Quarterly**: Security audit of dependencies and workflow

## Dry-Run Testing

Before major releases, you can test the workflow without publishing:

```bash
# Trigger workflow manually with dry-run mode
gh workflow run build-and-publish-cli.yml --field dry_run=true
```

This runs the full workflow but skips the npm publish step.

## Versioning Strategy

Follow semantic versioning (semver):

- **Major** (X.0.0): Breaking changes
- **Minor** (X.Y.0): New features, backwards compatible
- **Patch** (X.Y.Z): Bug fixes, backwards compatible

Examples:
```bash
pnpm release:major  # 1.2.3 → 2.0.0 (breaking changes)
pnpm release:minor  # 1.2.3 → 1.3.0 (new features)
pnpm release:patch  # 1.2.3 → 1.2.4 (bug fixes)
```

## Support

- **Documentation**: This file (RELEASE.md)
- **Security**: See SECURITY.md
- **Issues**: https://github.com/manifoldlogic/crewchief/issues
- **Workflows**: `.github/workflows/build-and-publish-*.yml`
- **Scripts**: `packages/*/scripts/release.*`

## Quick Cheat Sheet

```bash
# Pre-flight checks
git checkout main && git pull
cd packages/cli  # or maproom-mcp

# Create release
pnpm release:minor  # or :major, :patch

# Monitor
gh run watch

# Verify
npm view @crewchief/cli@X.Y.Z

# Test
npm install -g @crewchief/cli@X.Y.Z
crewchief --version

# Hotfix (if needed)
pnpm release:patch  # Immediately publish fixed version
```
