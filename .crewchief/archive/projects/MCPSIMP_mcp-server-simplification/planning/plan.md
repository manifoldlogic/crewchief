# Execution Plan: MCP Server Simplification

## Overview

Transform `@crewchief/maproom-mcp` from a 2,000-line Docker orchestration tool into a ~50-line single-purpose MCP server.

## Phase 1: Core Simplification

### 1.1 Replace CLI Entry Point
**Agent**: general-purpose
**Files**: `packages/maproom-mcp/bin/cli.cjs`

> **CRITICAL DEPENDENCY**: This task MUST complete before Phase 1.2 (Delete Unused Files).
> The current cli.cjs imports `config-manager.js` and `docker-detection.js`.
> Deleting those files before replacing cli.cjs will break the package.

Replace the entire 1,971-line CLI with minimal entry point:

```javascript
#!/usr/bin/env node

/**
 * Maproom MCP Server
 *
 * Single-purpose: Run MCP server via stdio.
 * Expects database to exist (use VSCode extension or docker compose for setup).
 */

function resolveDatabase() {
  // 1. Explicit override
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL
  }

  // 2. DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  }

  // 3. Default localhost
  return 'postgresql://maproom:maproom@localhost:5433/maproom'
}

async function main() {
  process.env.MAPROOM_DATABASE_URL = resolveDatabase()
  await import('../dist/index.js')
}

main().catch(error => {
  console.error('MCP server error:', error.message)
  process.exit(1)
})
```

### 1.2 Delete Unused Files
**Agent**: general-purpose

> **PREREQUISITE**: Phase 1.1 (Replace CLI Entry Point) must be completed first.

**Source files to delete** (no longer imported after cli.cjs replacement):
- `packages/maproom-mcp/src/config-manager.ts`
- `packages/maproom-mcp/src/utils/docker-detection.ts`

**Config files to delete** (orchestration removed):
- `packages/maproom-mcp/config/docker-compose.yml`
- `packages/maproom-mcp/config/Dockerfile.mcp-server`
- `packages/maproom-mcp/config/Dockerfile.combined`
- `packages/maproom-mcp/config/Dockerfile.maproom`
- `packages/maproom-mcp/config/init.sql`
- `packages/maproom-mcp/config/docker-compose.override.yml`
- `packages/maproom-mcp/config/docker-compose.env.example`
- `packages/maproom-mcp/config/docker-compose.test.yml`
- `packages/maproom-mcp/config/postgresql.conf`
- `packages/maproom-mcp/config/devcontainer-network-fix.sh`
- `packages/maproom-mcp/config/DEVCONTAINER_NETWORKING.md`

**Test files to delete** (reference deleted modules):
- `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`

### 1.3 Update Package.json
**Agent**: general-purpose
**File**: `packages/maproom-mcp/package.json`

Changes:
- Version: `2.2.3` → `3.0.0`
- Remove dependency: `chokidar`
- Simplify `files` array (remove deleted config files)
- Remove setup/scan/watch scripts
- Update description

## Phase 2: VSCode Extension Updates

### 2.1 Update MCP Config Writer
**Agent**: vscode-extension-specialist
**File**: `packages/vscode-maproom/src/config/mcp-writer.ts`

**Current state**: `buildEnvironment()` only returns provider-specific API keys.

**Required changes to `buildEnvironment()` method**:
1. Add `MAPROOM_DATABASE_URL` - always include connection string
2. Add `MAPROOM_EMBEDDING_PROVIDER` - pass the selected provider

Updated implementation:
```typescript
private buildEnvironment(provider: EmbeddingProvider): Record<string, string> {
  const env: Record<string, string> = {
    // Always include database URL (required for MCP server)
    MAPROOM_DATABASE_URL: 'postgresql://maproom:maproom@localhost:5433/maproom',
    // Always include provider selection
    MAPROOM_EMBEDDING_PROVIDER: provider,
  }

  // Add provider-specific credentials
  switch (provider) {
    case 'openai':
      env.OPENAI_API_KEY = '${env:OPENAI_API_KEY}'
      break
    case 'google':
      env.GOOGLE_APPLICATION_CREDENTIALS = '${env:GOOGLE_APPLICATION_CREDENTIALS}'
      break
    case 'ollama':
      // Ollama doesn't need environment variables
      break
  }

  return env
}
```

**Verification**: Generated mcp.json should include `MAPROOM_DATABASE_URL` and `MAPROOM_EMBEDDING_PROVIDER`

### 2.2 Update Version Constant
**Agent**: general-purpose
**File**: `packages/vscode-maproom/src/constants.ts`

Change: `MAPROOM_MCP_VERSION = '2.2.3'` → `MAPROOM_MCP_VERSION = '3.0.0'`

### 2.3 Update Extension docker-compose.yml
**Agent**: vscode-extension-specialist
**File**: `packages/vscode-maproom/config/docker-compose.yml`

**Current state**: Contains three services (postgres, ollama, maproom-mcp) and three volumes.

**Required changes**:
1. **Remove services**:
   - Delete `ollama` service definition (lines 49-84)
   - Delete `maproom-mcp` service definition (lines 86-131)

2. **Remove volumes**:
   - Delete `ollama-models` volume
   - Delete `maproom-logs` volume
   - Keep only `maproom-data` volume

3. **Keep**:
   - `postgres` service (unchanged)
   - `maproom-data` volume
   - `maproom-network` network

**Resulting docker-compose.yml** should have ~50 lines (down from ~144).

### 2.4 Update DockerManager Service Startup
**Agent**: vscode-extension-specialist
**File**: `packages/vscode-maproom/src/docker/manager.ts`

**Current state**: `ensureServicesRunning()` conditionally starts Ollama based on provider.

**Required changes to `ensureServicesRunning()` method**:
```typescript
// BEFORE (lines 193-199):
const services = provider === 'ollama'
  ? ['postgres', 'ollama', 'maproom-mcp']
  : ['postgres', 'maproom-mcp']

// AFTER:
const services = ['postgres']  // Only PostgreSQL, always
```

**Additional changes**:
- Remove `provider` parameter from method signature (no longer needed)
- Update callers to not pass provider
- Simplify documentation comments

### 2.5 Update MCP Writer Tests
**Agent**: general-purpose
**File**: `packages/vscode-maproom/src/config/mcp-writer.test.ts`

Update tests to verify:
1. `MAPROOM_DATABASE_URL` is included in generated config
2. `MAPROOM_EMBEDDING_PROVIDER` is included in generated config
3. Provider-specific keys still work correctly

## Phase 3: Documentation & Testing

### 3.1 Update CLAUDE.md
**Agent**: general-purpose
**File**: `packages/maproom-mcp/CLAUDE.md`

Reflect simplified architecture and removed commands.

### 3.2 Write Unit Tests
**Agent**: general-purpose
**File**: `packages/maproom-mcp/tests/unit/resolve-database.test.ts`

Test the three database resolution paths.

### 3.3 Manual Verification
**Agent**: verify-ticket
**Checklist**:
- [ ] `npx @crewchief/maproom-mcp` with database → Server starts
- [ ] `npx @crewchief/maproom-mcp` without database → Error message
- [ ] DevContainer with `IN_DEVCONTAINER=true` → Uses container hostname
- [ ] VSCode extension Docker management works
- [ ] Existing MCP tests pass

## Phase 4: Release

### 4.1 Update README
**Agent**: general-purpose
**File**: `packages/maproom-mcp/README.md`

Document:
- Breaking changes
- New usage pattern
- Migration guide

### 4.2 Version Bump
**Agent**: commit-ticket
**Files**: All package.json files

Ensure version 3.0.0 across workspace.

## Agent Assignments

| Phase | Task | Agent |
|-------|------|-------|
| 1.1 | Replace CLI | general-purpose |
| 1.2 | Delete files | general-purpose |
| 1.3 | Update package.json | general-purpose |
| 2.1 | MCP config writer | vscode-extension-specialist |
| 2.2 | Version constant | general-purpose |
| 2.3 | Update docker-compose.yml | vscode-extension-specialist |
| 2.4 | Update DockerManager | vscode-extension-specialist |
| 2.5 | Update MCP writer tests | general-purpose |
| 3.1 | Update CLAUDE.md | general-purpose |
| 3.2 | Write unit tests | general-purpose |
| 3.3 | Manual verification | verify-ticket |
| 4.1 | Update README | general-purpose |
| 4.2 | Version bump | commit-ticket |

## Coordination Notes

### Execution Order
Phase 1 (MCP package) and Phase 2 (VSCode extension) can be developed in parallel, but:
- Phase 1 must complete and publish to npm before Phase 4.2
- Extension references `@crewchief/maproom-mcp@VERSION`, so version must exist

### Publishing Sequence
1. Complete Phase 1-3
2. Publish `@crewchief/maproom-mcp@3.0.0` to npm
3. Update extension version constant (Phase 2.2)
4. Publish extension update (Phase 4.2)

### Rollback Plan
If issues are discovered after publishing:

**MCP Package Rollback**:
```bash
npm deprecate @crewchief/maproom-mcp@3.0.0 "Breaking issues discovered"
# Users will continue using 2.x versions
```

**Extension Rollback**:
- Revert `MAPROOM_MCP_VERSION` to `2.2.3`
- Publish extension patch with previous version reference

**Database Compatibility**: No database migrations in this change, so rollback is safe

## Security Checkpoints

- [ ] Phase 1: No credentials hardcoded in new CLI
- [ ] Phase 2: MCP config doesn't expose secrets in logs
- [ ] Phase 3: Tests verify credential redaction preserved

## Success Criteria

1. **CLI reduced to ~50 lines** (from 1,971)
2. **`npx @crewchief/maproom-mcp` runs MCP server directly** (no subcommands)
3. **Database auto-detection works** for all three scenarios
4. **VSCode extension unchanged** for users
5. **All existing tests pass**
6. **Version 3.0.0 published**
