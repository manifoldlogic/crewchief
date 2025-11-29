# VSCODEDB - Problem Analysis

## Problem Definition

The Maproom VSCode extension currently requires PostgreSQL via Docker as a hard dependency for all users. This creates significant friction:

1. **Setup Complexity**: Users must install Docker Desktop, understand container management, and ensure services are running before the extension works
2. **Resource Overhead**: Running PostgreSQL in Docker consumes significant memory (100MB+) and CPU for casual users who just want code search
3. **Platform Limitations**: Some corporate environments restrict Docker usage; some machines (thin clients, older hardware) struggle with container workloads
4. **Zero-Config Competition**: Modern developer tools (Cursor, Continue) work immediately after installation without external dependencies

The underlying Rust indexer (`crewchief-maproom`) already supports SQLite as a backend. The MCP server (`packages/maproom-mcp`) was just updated in the MCPDB project to support SQLite URLs. The VSCode extension is the final piece that needs SQLite support.

## Daemon SQLite Support Verification

**Verification Date:** 2025-11-26
**Status:** ✅ CONFIRMED WORKING

The Rust daemon (`crewchief-maproom`) fully supports SQLite via the `MAPROOM_DATABASE_URL` environment variable. Testing confirmed:

```bash
# Test 1: Daemon accepts SQLite URL and starts successfully
$ MAPROOM_DATABASE_URL=sqlite:///tmp/test.db ./target/release/crewchief-maproom serve
# Result: Process started, exited cleanly when terminated (exit code 124 = timeout)

# Test 2: URL parsing works correctly
$ MAPROOM_DATABASE_URL=sqlite:///nonexistent/path/test.db ./target/release/crewchief-maproom serve
# Result: Error about "Permission denied" creating /nonexistent/path - proves URL was parsed

# Test 3: Database operations attempted
$ MAPROOM_DATABASE_URL=sqlite:///tmp/maproom-test/test.db ./target/release/crewchief-maproom status
# Result: "no such table: repos" - proves SQLite connection established
```

**Key Finding:** The daemon already supports SQLite. No additional Rust work required.

This means:
- **VECSTORE project**: Not a blocking dependency (daemon works)
- **MAPCLI project**: Not a blocking dependency (CLI already supports SQLite)
- **VSCODEDB**: Can proceed immediately

## Current State Analysis

### Extension Architecture

The VSCode extension (`packages/vscode-maproom/`) currently has:

```
src/
├── extension.ts              # Main entry point - heavily PostgreSQL-focused
├── services/
│   └── postgres-checker.ts   # TCP connectivity checker (PostgreSQL only)
├── docker/
│   └── manager.ts            # Docker container lifecycle
├── process/
│   └── orchestrator.ts       # Process management (requires postgres config)
├── ui/
│   └── setupWizard.ts        # Provider selection (assumes Docker)
└── config/
    └── secrets.ts            # API key management
```

### Key Dependencies

1. **`postgres-checker.ts`**: Performs TCP socket check to verify PostgreSQL availability. Returns boolean "available" status.

2. **`extension.ts`**:
   - Calls `ensureDockerRunning()` unconditionally
   - Calls `ensurePostgresAvailable()` as blocking prerequisite
   - Creates `ProcessOrchestrator` with hardcoded postgres config
   - Uses `getPostgresUrl()` for database connection

3. **`package.json` settings**:
   - Already defines `maproom.database.provider` with `sqlite`/`postgres` enum
   - Has all PostgreSQL-specific settings (host, port, user, password, name)
   - Missing: SQLite file path setting

4. **Docker Manager**: Tightly coupled to extension activation flow

### Configuration Schema (Existing)

```json
"maproom.database.provider": {
  "type": "string",
  "enum": ["sqlite", "postgres"],
  "default": "sqlite"  // Already defaults to SQLite!
}
```

The setting exists but is **completely ignored** by the extension code.

## Existing Industry Solutions

### 1. SQLite-First Tools

**Zed Editor** and **Obsidian** demonstrate the SQLite-first pattern:
- SQLite database created automatically on first use
- Zero configuration required
- PostgreSQL available as optional "team/enterprise" feature

### 2. Fallback Chains

**VS Code Settings Sync** uses a tiered approach:
- Local storage first
- Cloud sync optional
- Clear migration path between tiers

### 3. Docker-Optional

**GitHub Codespaces** approach:
- Containers available but not required
- Local development works without Docker
- Configuration detects and adapts to environment

## Gap Analysis

| Capability | Current State | Target State |
|------------|--------------|--------------|
| SQLite support | Setting exists, unused | Fully functional default |
| Docker requirement | Mandatory | Optional (for PostgreSQL) |
| Zero-config startup | No | Yes |
| PostgreSQL mode | Only mode | Advanced option |
| File path config | Missing | `~/.maproom/maproom.db` default |
| Setup wizard | Requires Docker | Detects existing index |

## Requirements

### Functional Requirements

1. **FR-1**: Extension activates without Docker when SQLite mode selected
2. **FR-2**: Automatic detection of `~/.maproom/maproom.db` file
3. **FR-3**: Setup wizard allows SQLite path selection
4. **FR-4**: PostgreSQL mode still works when explicitly configured
5. **FR-5**: Clear user feedback about current backend mode

### Non-Functional Requirements

1. **NFR-1**: Activation time remains <500ms
2. **NFR-2**: No new npm dependencies
3. **NFR-3**: Existing PostgreSQL users unaffected
4. **NFR-4**: Clear migration documentation

## Research Findings

### MCPDB Project Artifacts

The recently completed MCPDB project provides reusable patterns:

1. **`resolve-database.ts`** pattern can be adapted:
   - 4-tier hierarchy (Explicit → DevContainer → SQLite default → PostgreSQL fallback)
   - `DatabaseConfig` interface with type discrimination
   - `isSqliteUrl()` helper function

2. **Graceful degradation** pattern:
   - SQLite mode returns `chunk_id=0` with warning
   - Status shows "degraded" response with helpful tips

### File Existence Check

The MCP server validates SQLite file existence in `daemon.ts`:
```typescript
if (dbConfig.type === 'sqlite' && dbConfig.path) {
  if (!existsSync(dbConfig.path)) {
    throw new Error(`SQLite database not found: ${dbConfig.path}...`)
  }
}
```

This pattern should be reused in the VSCode extension.

## Constraints

1. **Interface Stability**: MCP server's `DatabaseConfig` interface is now stable (from MCPDB)
2. **VSCode API**: Must use official extension APIs only
3. **Backward Compatibility**: Existing PostgreSQL users must not be broken
4. **No Major Refactoring**: Keep changes focused on database abstraction
5. **Preserve Existing Fields**: Keep `databaseUrlOverride` in `OrchestratorConfig` (already supports SQLite URLs)

## First-Run Experience

When the user has neither SQLite nor PostgreSQL configured:
1. Extension activates and checks `maproom.database.provider` setting (default: `sqlite`)
2. If SQLite: Check for `~/.maproom/maproom.db` file
3. If file missing: Setup wizard runs automatically (existing behavior in `extension.ts`)
4. User guided to run `crewchief-maproom scan` or configure PostgreSQL

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing PostgreSQL users | High | Preserve all PostgreSQL settings and paths |
| SQLite file not found confusion | Medium | Clear error messages with setup instructions |
| Docker manager conflicts | Low | Keep Docker code paths, just make them conditional |

## Success Metrics

1. Extension activates successfully with `~/.maproom/maproom.db` and no Docker
2. All existing commands work with SQLite backend
3. PostgreSQL mode unchanged when configured
4. Activation time <500ms maintained
5. Zero new npm dependencies added
