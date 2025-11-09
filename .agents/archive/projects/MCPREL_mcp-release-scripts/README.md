# Project: MCP Release Scripts Update

**Status**: Planning Complete
**SLUG**: MCPREL
**Duration**: 1-2 hours
**Complexity**: Simple

## Problem

The `@crewchief/maproom-mcp` package currently has release scripts that bump version and immediately publish to npm. However, the project now uses GitHub Actions for automated builds and publishing. The scripts need to be updated to create git commits and tags that trigger CI/CD, rather than publishing directly.

## Solution

Update release scripts to:
1. Bump version in package.json (existing functionality)
2. Create git commit with message: `chore(release): bump version to X.Y.Z`
3. Create annotated git tag: `vX.Y.Z`
4. Push both commit and tag to origin

GitHub Actions (existing or future workflow) will detect the tag push and handle building/publishing.

## Current State

**Existing Scripts** (`packages/maproom-mcp/package.json`):
```json
"release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
"release:minor": "node scripts/bump-version.js minor && pnpm publish --access public --no-git-checks",
"release:major": "node scripts/bump-version.js major && pnpm publish --access public --no-git-checks"
```

**Issue**: These scripts publish immediately, bypassing git integration and CI/CD.

## Desired State

**New Scripts**:
```json
"release:patch": "node scripts/release.js patch",
"release:minor": "node scripts/release.js minor",
"release:major": "node scripts/release.js major"
```

**New Script** (`scripts/release.js`):
- Calls existing `bump-version.js` to increment version
- Creates git commit with Conventional Commits format
- Creates annotated git tag with version (e.g., `v1.3.2`)
- Pushes both to origin
- Exits with clear error if any step fails

**What Happens Next** (Automated by GitHub Actions):
When the `v*.*.*` tag is pushed, two GitHub Actions workflows trigger automatically:

1. **`build-and-publish-maproom-mcp.yml`**:
   - Builds Rust binaries for 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
   - Packages them into npm package
   - Publishes to npm: `@crewchief/maproom-mcp@X.Y.Z`

2. **`publish-maproom-mcp-image.yml`**:
   - Builds multi-platform Docker images (linux/amd64, linux/arm64)
   - Publishes to Docker Hub: `manifoldlogic/crewchief_maproom-mcp`
   - Creates tags: `X.Y.Z`, `X.Y`, `X`, `latest`

**Result**: Single command (`pnpm release:patch`) triggers complete release to npm and Docker Hub.

## Implementation Approach

### Phase 1: Implementation (Core)
- **MCPREL-1001**: Create `release.js` script with git operations
- **MCPREL-1002**: Update package.json scripts to use new workflow

### Phase 2: Testing (Quality)
- **MCPREL-2001**: Manual testing on feature branch

### Phase 3: Documentation (Polish - Optional)
- **MCPREL-3001**: Update README if needed

**Philosophy**: Keep it simple. Don't overdo it.

## Key Design Decisions

1. **Reuse existing `bump-version.js`**: Don't reinvent version bumping logic
2. **Sequential script**: Simple linear flow, fail fast on errors
3. **No new dependencies**: Use Node.js built-ins only
4. **Manual testing**: Sufficient for this use case, no unit tests needed
5. **Clear error messages**: Let git provide detailed errors, don't obscure them

## Agents

- **general-purpose**: Handles all implementation and testing
  - JavaScript file creation (`release.js`)
  - JSON editing (package.json)
  - Manual testing execution
  - Documentation updates

No specialized agents needed for this simple task.

## Planning Documents

- [Analysis](./planning/analysis.md) - Problem space, industry patterns, technical considerations
- [Architecture](./planning/architecture.md) - Script design, flow, error handling
- [Quality Strategy](./planning/quality-strategy.md) - Testing approach (manual testing focus)
- [Security Review](./planning/security-review.md) - Risk assessment (very low risk)
- [Implementation Plan](./planning/plan.md) - Phases, tickets, timeline

## Complete Release Flow

```
┌─────────────────────────────────────┐
│ Developer runs: pnpm release:patch  │
└────────────┬────────────────────────┘
             │
             ▼
┌─────────────────────────────────────┐
│ release.js:                         │
│ 1. Bump version: 1.3.1 → 1.3.2     │
│ 2. Git commit: "chore(release)..."  │
│ 3. Git tag: v1.3.2                  │
│ 4. Git push origin (commit + tag)   │
└────────────┬────────────────────────┘
             │
             ▼
┌─────────────────────────────────────┐
│ GitHub detects tag push: v1.3.2     │
└────────┬───────────────┬────────────┘
         │               │
         ▼               ▼
┌──────────────┐  ┌────────────────┐
│ Workflow 1:  │  │ Workflow 2:    │
│ Build & npm  │  │ Docker images  │
└──────┬───────┘  └────────┬───────┘
       │                   │
       ▼                   ▼
┌──────────────┐  ┌────────────────┐
│ 4 platforms  │  │ 2 platforms    │
│ linux x64/arm│  │ linux amd/arm  │
│ darwin x64/arm│  │                │
└──────┬───────┘  └────────┬───────┘
       │                   │
       ▼                   ▼
┌──────────────┐  ┌────────────────┐
│ npm publish  │  │ Docker Hub     │
│ @crewchief/  │  │ manifoldlogic/ │
│ maproom-mcp  │  │ crewchief_...  │
└──────────────┘  └────────────────┘
```

## Success Criteria

Project is complete when:
1. ✅ `pnpm release:patch/minor/major` bumps version
2. ✅ Creates git commit with format: `chore(release): bump version to X.Y.Z`
3. ✅ Creates annotated git tag: `vX.Y.Z`
4. ✅ Pushes both commit and tag to origin
5. ✅ GitHub Actions workflows trigger successfully:
   - `build-and-publish-maproom-mcp.yml` completes
   - `publish-maproom-mcp-image.yml` completes
6. ✅ Provides clear error messages on failure
7. ✅ Tested manually and verified working

## Files Affected

- `packages/maproom-mcp/package.json` - Update scripts
- `packages/maproom-mcp/scripts/release.js` - Create new file
- `packages/maproom-mcp/scripts/bump-version.js` - No changes (keep as-is)

## Risk Level

**Very Low**

- Simple script changes
- Standard git operations
- No breaking changes
- Easy to rollback
- Developer-facing only

## Timeline

- **Phase 1**: 45-60 minutes (implementation)
- **Phase 2**: 15-30 minutes (testing)
- **Phase 3**: 15 minutes (documentation, if needed)
- **Total**: 1-2 hours

## Next Steps

1. Review planning documents
2. Generate tickets: `pnpm claude /create-project-tickets MCPREL`
3. Execute tickets: `pnpm claude /work-on-project MCPREL`
4. Test on feature branch
5. Merge when verified
