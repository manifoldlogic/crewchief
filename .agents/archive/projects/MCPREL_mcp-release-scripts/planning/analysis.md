# Analysis: MCP Release Script Update

## Problem Statement

The `@crewchief/maproom-mcp` package currently has release scripts (`release:patch`, `release:minor`, `release:major`) that were designed for manual npm publishing. However, the project has moved to GitHub Actions for automated builds and Docker Hub publishing. The current scripts bump version and publish immediately, which is no longer the correct workflow.

## Current State

### Existing Scripts (package.json:31-33)
```json
"release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
"release:minor": "node scripts/bump-version.js minor && pnpm publish --access public --no-git-checks",
"release:major": "node scripts/bump-version.js major && pnpm publish --access public --no-git-checks"
```

### What They Do Now
1. Run `bump-version.js` to increment version in package.json
2. Immediately publish to npm with `--no-git-checks` flag

### What's Wrong
- No git commit for version change
- No git tag created
- No push to trigger GitHub Actions
- The `--no-git-checks` flag bypasses git integration entirely

### GitHub Actions Status
**CONFIRMED**: Two production GitHub Actions workflows are already in place and operational:

1. **`build-and-publish-maproom-mcp.yml`** - npm Publishing Workflow
   - Triggers on: `v*.*.*` tags (e.g., v1.3.2)
   - Builds: Rust binaries for 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
   - Publishes: `@crewchief/maproom-mcp` to npm registry
   - Includes: Binary validation, tarball verification, retry logic
   - Uses: `NPM_TOKEN` secret for authentication

2. **`publish-maproom-mcp-image.yml`** - Docker Image Workflow
   - Triggers on: Same `v*.*.*` tags
   - Builds: Multi-platform Docker images (linux/amd64, linux/arm64)
   - Publishes: `manifoldlogic/crewchief_maproom-mcp` to Docker Hub
   - Tags: Full version (1.3.2), minor (1.3), major (1), latest
   - Uses: `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` secrets

**Critical Insight**: When developer pushes a version tag, BOTH workflows trigger automatically, resulting in:
- npm package published with cross-platform Rust binaries
- Docker images published to Docker Hub with proper tagging

This validates our approach: release scripts only need to create and push the tag. All building, packaging, and publishing is handled by CI/CD.

## Desired Workflow

Modern npm package release with CI/CD follows this pattern:

1. **Developer runs**: `pnpm release:patch` (or minor/major)
2. **Script performs**:
   - Bump version in package.json
   - Commit with message like "chore(release): bump version to 1.3.2"
   - Create git tag (e.g., `v1.3.2`)
   - Push commit and tag to origin
3. **GitHub Actions detects**: Tag push triggers workflow
4. **CI/CD performs**:
   - Build Docker images
   - Publish to Docker Hub
   - Publish to npm registry
   - Create GitHub release

## Industry Patterns

### Standard Release Tools
Popular packages use tools like:
- **semantic-release**: Fully automated based on commits
- **np**: Interactive release with checks
- **release-it**: Configurable release workflow
- **Custom scripts**: Simple bash/node scripts for small projects

For this project, custom scripts are appropriate because:
- Simple, single package (not monorepo)
- Clear semver versioning
- Minimal release complexity
- Already has `bump-version.js` foundation

### Git Tag Conventions
- Format: `v{major}.{minor}.{patch}` (e.g., `v1.3.2`)
- Annotated tags are better than lightweight (include message, date, tagger)
- Tag message typically: "Release version 1.3.2"

### Commit Message Conventions
Common patterns:
- `chore(release): 1.3.2` (simple)
- `chore: bump version to 1.3.2` (clear)
- `Release v1.3.2` (minimal)

For this project: "chore(release): bump version to X.Y.Z" follows existing Conventional Commits pattern.

## Technical Considerations

### Git Operations Required
```bash
# 1. Commit version change
git add packages/maproom-mcp/package.json
git commit -m "chore(release): bump version to X.Y.Z"

# 2. Create annotated tag
git tag -a "vX.Y.Z" -m "Release version X.Y.Z"

# 3. Push with tags
git push origin main --follow-tags
# Or separately:
git push origin main
git push origin vX.Y.Z
```

### Working Directory
Scripts run from package directory (`packages/maproom-mcp/`), so git commands need proper paths or must change to repository root.

### Error Handling
Scripts should:
- Check if working directory is clean before starting
- Verify git operations succeed before proceeding
- Exit with clear error messages if anything fails
- Avoid partial state (version bumped but not committed)

### Safety Checks
Should implement (but keep simple per user's "don't overdo it" directive):
- ✅ Check git is available
- ✅ Check if working directory is clean (warn if dirty, but allow override)
- ✅ Verify successful tag creation
- ❌ Skip: Branch protection checks (YAGNI)
- ❌ Skip: Changelog generation (not requested)
- ❌ Skip: Pre-release testing (CI handles this)

## Implementation Options

### Option 1: Extend bump-version.js
Modify existing `scripts/bump-version.js` to also perform git operations.

**Pros:**
- Single file to maintain
- Atomic operation
- Existing code works

**Cons:**
- Mixes concerns (version bumping + git operations)
- Harder to test individually

### Option 2: Separate Scripts
Create new script `scripts/release.js` that calls `bump-version.js` then does git operations.

**Pros:**
- Separation of concerns
- bump-version.js stays focused
- Easier to test

**Cons:**
- More files
- Need to coordinate between scripts

### Option 3: Shell Script
Replace with bash script that does everything.

**Pros:**
- Git operations natural in bash
- Simple and direct

**Cons:**
- Less portable (Windows compatibility)
- Existing code is Node.js (consistency)

### Recommendation: Option 2 (Separate Scripts)
- Keep `bump-version.js` focused on version bumping
- Create `release.js` that orchestrates the workflow
- Update package.json scripts to call `release.js`
- Maintains separation of concerns without overengineering

## Scope Definition

### In Scope
1. Update `bump-version.js` to bump version only (already does this)
2. Create `release.js` to orchestrate: bump → commit → tag → push
3. Update package.json scripts to use new workflow
4. Basic error handling and user feedback

### Out of Scope (Per "Don't Overdo It")
- Changelog generation
- GitHub release creation (CI handles this)
- Interactive confirmations
- Pre-release testing
- Rollback mechanisms
- Dry-run mode
- Branch protection checks
- NPM publishing (GitHub Actions handles this now)

### Edge Cases to Handle
- Dirty working directory: Warn but proceed (developer responsibility)
- Git push failure: Exit with clear error
- Tag already exists: Exit with clear error
- Network failure during push: User can manually retry

## Success Criteria

Release scripts are successful when:
1. `pnpm release:patch` (or minor/major) bumps version in package.json
2. Creates a git commit with message "chore(release): bump version to X.Y.Z"
3. Creates annotated git tag "vX.Y.Z"
4. Pushes both commit and tag to origin
5. All operations complete or script exits with clear error message
6. Can be tested locally without affecting production

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Accidental publish during testing | Low | High | Remove `pnpm publish` from scripts entirely |
| Partial state (commit but no tag) | Medium | Medium | Check each git operation, exit on failure |
| Push to wrong remote | Low | Medium | Use `origin` explicitly, document |
| Version conflict (tag exists) | Low | Low | Check tag existence before creating |
| Breaking existing workflow | Low | Medium | Test thoroughly before committing |

## References

- Current package.json: `/workspace/packages/maproom-mcp/package.json`
- Current bump script: `/workspace/packages/maproom-mcp/scripts/bump-version.js`
- Conventional Commits: https://www.conventionalcommits.org/
- Git tagging: https://git-scm.com/book/en/v2/Git-Basics-Tagging
