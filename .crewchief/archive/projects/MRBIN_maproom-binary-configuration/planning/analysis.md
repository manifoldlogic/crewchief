# Analysis: Maproom Binary Configuration

## Problem Definition

Developers and users currently lack a consistent, explicit way to configure which maproom binary the CLI should use. The binary resolution logic is duplicated across multiple files (worktrees.ts, maproom.ts, maproom-mcp process.ts) with subtle differences in implementation and search order. This creates several issues:

1. **Inconsistent Resolution**: Different code paths resolve the binary differently, leading to unpredictable behavior
2. **Development Friction**: Developers building locally must rely on environment variables or hope the packaged binary location is correct
3. **Incorrect Priority**: The current resolution in CLI checks packaged binaries before global installations, which can cause issues when developers have both a stale local build and a newer global installation
4. **No Config-based Override**: There's no persistent, repository-level way to specify a custom binary path

**Specific Pain Points:**
- Developer builds a new version locally → CLI still uses old packaged binary
- Production uses global install → CLI logic prioritizes packaged binary that might not exist
- No way to set binary path in crewchief.config.js for team consistency
- Environment variables work but aren't persisted or version controlled

## Context

The crewchief CLI is a TypeScript tool that wraps a Rust binary (crewchief-maproom) for indexing and search operations. The binary resolution happens in multiple places:

1. **packages/cli/src/cli/maproom.ts** - Main CLI command forwarding
2. **packages/cli/src/git/worktrees.ts** - Worktree creation auto-indexing
3. **packages/maproom-mcp/src/utils/process.ts** - MCP server binary discovery

Each location has slightly different resolution logic, creating maintenance burden and potential bugs.

The existing configuration system (crewchief.config.js) uses Zod schemas and supports:
- Repository settings (mainBranch, worktreeBasePath)
- Terminal settings (backend, iterm)
- Evaluation settings (autoMergeThreshold, quality checks)
- Worktree settings (copyIgnoredFiles)

There is currently NO setting for tool binaries like maproom.

## Existing Solutions

### Industry Patterns

1. **npm/yarn**: Use `PATH` resolution with optional explicit paths in package.json scripts
2. **cargo**: Uses `CARGO` env var or searches PATH
3. **python/pip**: Virtual environments isolate binary paths
4. **git**: Uses PATH with optional config options like `core.editor`

**Best Practice Pattern:**
```
1. Explicit environment variable (highest priority)
2. Config file setting (persistent, version-controllable)
3. System PATH (standard installation)
4. Fallback defaults (packaged/bundled binaries)
```

### Current Codebase Implementation

**packages/cli/src/cli/maproom.ts** (lines 7-50):
```typescript
function resolvePackagedMaproomBin(): string | null {
  // 1) CREWCHIEF_MAPROOM_BIN env var
  // 2) Packaged binary (platform-specific)
  // 3) Fallback symlink location
  // 4) Sibling maproom-mcp package
  // 5) Global PATH
}
```

**packages/cli/src/git/worktrees.ts** (lines 25-66):
```typescript
private async runMaproomScan(worktreePath: string): Promise<void> {
  // 1) CREWCHIEF_MAPROOM_BIN env var
  // 2) Packaged binary (multiple locations)
  // 3) Global PATH
  // NOTE: Different search order and paths than maproom.ts!
}
```

**packages/maproom-mcp/src/utils/process.ts** (lines 83-132):
```typescript
export function findMaproomBinary(): string | null {
  // 1) CREWCHIEF_MAPROOM_BIN env var
  // 2) Platform-specific packaged binary
  // 3) Development build paths (target/release/...)
  // 4) Returns null (caller tries system PATH)
  // NOTE: Includes dev paths not checked by CLI!
}
```

**Key Differences:**
- MCP includes development build detection (`target/release/`)
- CLI worktrees.ts and maproom.ts have different packaged path lists
- Priority order varies: CLI checks packaged before global, MCP returns null for caller to try PATH

## Current State

**Binary Resolution Logic Locations:**
1. `packages/cli/src/cli/maproom.ts` - 44 lines of resolution logic
2. `packages/cli/src/git/worktrees.ts` - 42 lines of resolution logic
3. `packages/maproom-mcp/src/utils/process.ts` - 50 lines in `findMaproomBinary()`

**Config Schema:**
- `RepositorySchema` at `packages/cli/src/config/schema.ts` (lines 3-6)
- Currently has: `mainBranch`, `worktreeBasePath`
- Does NOT have: any binary path configuration

**Environment Variable Usage:**
- `CREWCHIEF_MAPROOM_BIN` checked in 3+ locations
- Documented in `.devcontainer/README.md` and `docs/development/local-development.md`
- Works but not persistent or team-friendly

## Research Findings

### Code Analysis

1. **Duplication is Extensive**: Three separate implementations of nearly identical logic
2. **Priority Order Inconsistency**:
   - CLI prioritizes packaged > global
   - Production users expect global > packaged
   - Development needs explicit path > everything
3. **No Shared Utility**: Each location reimplements binary discovery
4. **Platform Detection**: All three handle Windows .exe suffix and platform-specific paths
5. **Error Handling**: Each location has different fallback behavior

### Development Workflow Analysis

From `docs/development/local-development.md`:
```bash
# Current recommended approach
export CREWCHIEF_MAPROOM_BIN="./packages/cli/bin/darwin-arm64/crewchief-maproom"
crewchief maproom scan
```

**Problems:**
- Must set env var in every shell session
- Not documented in crewchief.config.js
- Team members can't share this configuration

### Configuration System Analysis

The existing config loader (`packages/cli/src/config/loader.ts`):
- Traverses directory tree to find config (like tsconfig.json)
- Supports `crewchief.config.local.js` for gitignored overrides
- Uses Zod for validation with helpful error messages
- Loaded in multiple places, available where binary resolution happens

**Perfect fit** for adding binary path configuration!

## Constraints

### Technical Constraints

1. **Backwards Compatibility**: Existing resolution must continue to work for users who don't set the config
2. **Cross-package Consistency**: CLI, worktrees, and MCP should resolve the same way
3. **Platform Differences**: Must handle Windows .exe vs Unix executable names
4. **ESM Modules**: All packages use ESM, shared utility must be ESM-compatible

### Business Constraints

1. **No Breaking Changes**: This is a minor version bump, must be additive
2. **Development Workflow**: Must support local development builds
3. **Production Deployment**: Must work with global npm installs

### Resource Constraints

1. **Small Project**: Scoped to 1 day (S effort)
2. **No External Dependencies**: Can only use existing config/validation infrastructure
3. **Test Coverage**: Must maintain existing test coverage levels

## Success Criteria

### Primary Success Criteria

1. **Config Schema Updated**: `RepositorySchema` includes `maproomBinaryPath?: string`
2. **Consistent Resolution**: All three locations use same resolution order
3. **Shared Implementation**: Single utility function for binary resolution
4. **Correct Priority Order**:
   - CREWCHIEF_MAPROOM_BIN env var (highest)
   - config.repository.maproomBinaryPath
   - Global install (`command -v crewchief-maproom`)
   - Packaged binary (fallback)

### Measurable Outcomes

1. **Code Reduction**: ~100 lines of duplicated code → single shared utility
2. **Test Coverage**: Resolution logic has explicit tests for precedence order
3. **Documentation**: README.md and development docs reference config option
4. **Zero Regressions**: All existing tests pass without modification

### User Experience Success

1. **Developer Setup**: Can add `maproomBinaryPath` to crewchief.config.local.js
2. **Production Clarity**: Global install is preferred (no confusion with stale local builds)
3. **Team Consistency**: Can commit binary path to crewchief.config.js if desired
4. **Error Messages**: Clear feedback when binary not found at configured path

### Non-Goals

- NOT adding version checking or binary validation
- NOT auto-downloading binaries
- NOT changing database connection logic
- NOT modifying the Rust binary itself
