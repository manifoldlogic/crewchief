# Analysis: CI Workflow Fixes

## Problem Definition

The CrewChief project has **two critical CI workflow failures** blocking development:

1. **Test workflow fails on every push to main** - Blocks all continuous integration
2. **Docker image publishing fails on releases** - Blocks production deployments

Both failures stem from **tool version conflicts** and **monorepo workspace dependency resolution** issues that arose after recent package manager and dependency updates.

## Current State

### Test Workflow Failure (Critical)

**File**: `.github/workflows/test.yml`
**Status**: ❌ Failing on every run since Nov 22, 2025
**Impact**: Blocks all CI validation for PRs and main branch pushes

**Error**:
```
Error: Multiple versions of pnpm specified:
  - version 10 in the GitHub Action config with the key "version"
  - version pnpm@10.12.1+sha512... in the package.json with the key "packageManager"
```

**Root Cause**:
- Workflow explicitly sets `version: 10` in `pnpm/action-setup@v4` (line 59)
- Root `package.json` declares `packageManager: "pnpm@10.12.1+sha512..."`
- The pnpm action detects both and rejects the configuration as ambiguous
- This is a **new behavior** in pnpm/action-setup@v4 that enforces single source of truth

**Timeline**:
- Package.json updated with specific pnpm version (packageManager field)
- Test workflow still had legacy explicit version
- pnpm/action-setup@v4 now validates consistency
- All CI runs failing since then

---

### Docker Build Failure (Release Blocker)

**File**: `packages/maproom-mcp/config/Dockerfile.combined`
**Status**: ❌ Failing on release builds (tag pushes)
**Impact**: Cannot publish Docker images to registry

**Error**:
```
npm error code EUNSUPPORTEDPROTOCOL
npm error Unsupported URL Type "workspace:": workspace:*
```

**Root Cause**:
- `@crewchief/maproom-mcp` depends on `@maproom/daemon-client` via workspace protocol
- Dockerfile copies only `maproom-mcp/package.json` in isolation
- Uses `npm install` which doesn't understand pnpm's `workspace:` protocol
- npm fails when it encounters the unresolved workspace dependency

**Why this happens**:
```json
// packages/maproom-mcp/package.json
{
  "dependencies": {
    "@maproom/daemon-client": "workspace:*"  // ← pnpm-specific
  }
}
```

```dockerfile
# Dockerfile.combined (Stage 2)
COPY packages/maproom-mcp/package.json ./
RUN npm install  # ← npm doesn't understand workspace:
```

**Timeline**:
- daemon-client package extracted from maproom-mcp (recent refactor)
- package.json updated with workspace dependency
- Docker build not updated to handle monorepo structure
- Release builds failing since extraction

## Industry Solutions

### pnpm Version Management in CI

**Best Practice**: Use packageManager field as single source of truth

Modern approach (pnpm@8+):
```yaml
# Let action auto-detect from packageManager field
- uses: pnpm/action-setup@v4
  # No version specified - reads from package.json
```

Legacy approach (error-prone):
```yaml
# Explicit version (can drift from package.json)
- uses: pnpm/action-setup@v4
  with:
    version: 10  # ← Must stay in sync manually
```

**Ecosystem consensus**: packageManager field introduced in npm@7, pnpm@6, widely adopted 2023+

### Monorepo Docker Builds

**Common Patterns**:

1. **Full Context Copy** (simple, cache-inefficient)
   - Copy entire monorepo into builder
   - Let package manager resolve workspaces
   - Pro: Works with any package manager
   - Con: Invalidates cache on any workspace change

2. **pnpm in Docker** (matches dev environment)
   - Install pnpm in builder image
   - Copy workspace configuration + affected packages
   - Use `pnpm install --filter`
   - Pro: Exact dev/prod parity, efficient caching
   - Con: Adds pnpm to image (minimal impact in multi-stage)

3. **Pre-bundle Dependencies** (complex, optimal)
   - Pack workspace deps as tarballs before Docker build
   - Replace workspace: with file: references
   - Pro: Smallest context, best cache efficiency
   - Con: Requires build-time orchestration, fragile

**Industry trend**: Option 2 (pnpm in Docker) becoming standard for TypeScript monorepos using pnpm

## Existing Solutions in Project

### What Works Well

1. **packageManager field**: Project correctly uses modern package manager pinning
2. **Multi-stage Docker builds**: Efficient separation of Rust + Node.js builders
3. **Test infrastructure**: Solid PostgreSQL test isolation with service containers
4. **Rust build**: Works perfectly, no dependency issues

### What Needs Fixing

1. **Test workflow**: Remove explicit pnpm version (1 line change)
2. **Docker build**: Adopt pnpm in builder stage (15-20 line change)

## Research Findings

### pnpm/action-setup@v4 Behavior

**Source**: https://github.com/pnpm/action-setup/releases/tag/v4.0.0

Key changes in v4:
- **Automatic version detection** from package.json packageManager field
- **Validation**: Rejects conflicting version specifications
- **Breaking change**: Projects must choose explicit OR auto-detect, not both

Migration path:
```yaml
# Before (v3 style)
uses: pnpm/action-setup@v3
with:
  version: 8

# After (v4 style)
uses: pnpm/action-setup@v4
# Reads from package.json automatically
```

### Docker + pnpm Workspace Dependencies

**Source**: Official pnpm Docker guide (https://pnpm.io/docker)

Recommended pattern:
```dockerfile
# Install pnpm
RUN npm install -g pnpm

# Copy workspace config
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy needed packages
COPY packages/foo/package.json ./packages/foo/
COPY packages/bar/package.json ./packages/bar/

# Install filtered dependencies
RUN pnpm install --frozen-lockfile --filter foo...
```

**Key insight**: The `--filter` flag resolves workspace dependencies automatically

### daemon-client Package Analysis

**Size**: 124KB (20 compiled files)
**Dependencies**: Zero runtime dependencies
**Build output**: Pure TypeScript compilation (types + JS)

**Implication**: Extremely lightweight to include in Docker build, minimal overhead

## Decision Criteria

### Test Workflow Fix

**Requirements**:
- ✅ Must work with existing packageManager field
- ✅ Must not require manual version sync
- ✅ Must be compatible with pnpm/action-setup@v4+
- ✅ Must be simple (1-2 line change)

**Solution**: Remove explicit version, let action auto-detect

**Risk**: None (action specifically designed for this)

### Docker Build Fix

**Requirements**:
- ✅ Must resolve workspace: dependencies
- ✅ Must maintain multi-stage efficiency
- ✅ Must not significantly increase image size
- ✅ Must work for future workspace dependencies
- ✅ Should match local dev environment (pnpm)

**Evaluation**:

| Criterion | Full Context | pnpm in Docker | Pre-bundle |
|-----------|-------------|----------------|------------|
| Resolves workspace: | ✅ | ✅ | ✅ |
| Cache efficiency | ❌ | ✅ | ✅✅ |
| Image size | 🟡 | 🟡 | ✅ |
| Matches dev | ❌ (npm) | ✅ (pnpm) | ❌ (file:) |
| Complexity | ✅ Simple | 🟡 Moderate | ❌ Complex |
| Future-proof | ✅ | ✅ | ❌ |

**Recommended**: pnpm in Docker (Option A)
- Best balance of simplicity, correctness, and future-proofing
- Matches existing tooling (pnpm everywhere)
- 50MB overhead only in builder stage (removed in final image)
- Works for any number of workspace dependencies

## Constraints

### Technical Constraints

1. **Multi-platform Docker builds**: Must support linux/amd64 and linux/arm64
2. **Multi-stage build**: Must preserve Rust + Node.js separation
3. **Image size**: Final image should remain ~200-300MB (no significant bloat)
4. **Build cache**: Changes should not invalidate unrelated layers

### Operational Constraints

1. **Zero downtime**: Fixes must not break existing deployments
2. **Backward compatibility**: Local dev environment unchanged
3. **CI time**: Should not significantly increase build duration
4. **No secret changes**: Must work with existing GitHub secrets

### Policy Constraints

1. **No manual version sync**: packageManager field is source of truth
2. **Match dev environment**: Docker should use same tools as local dev
3. **Minimal changes**: Prefer simple, well-understood patterns
4. **Standard practices**: Follow official pnpm Docker guidance

## Success Criteria

### Test Workflow

✅ **Fixed when**:
- pnpm/action-setup@v4 installs pnpm successfully
- Version matches packageManager field exactly
- Tests run and pass (or fail for code reasons, not infra)
- No version conflict errors

### Docker Build

✅ **Fixed when**:
- `npm install` replaced with `pnpm install --filter`
- workspace: dependencies resolve correctly
- Multi-platform build completes (amd64 + arm64)
- Final image size unchanged (<5MB difference)
- Docker Hub push succeeds

### Both Workflows

✅ **Complete when**:
- CI passing on main branch
- Release workflow publishes Docker images
- No manual intervention required for future builds
- Documentation updated for maintainers
