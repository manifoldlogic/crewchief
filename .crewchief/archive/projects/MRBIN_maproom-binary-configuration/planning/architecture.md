# Architecture: Maproom Binary Configuration

## Overview

This project adds configuration-based maproom binary resolution to the crewchief CLI and consolidates three separate binary resolution implementations into a single shared utility. The architecture follows the existing config system pattern and provides a clear precedence order that prioritizes explicit configuration over auto-detection.

**Key Components:**
1. Config schema extension (Zod validation in RepositorySchema)
2. Shared binary resolution utility (packages/cli/src/utils/maproom-binary.ts)
3. Updated consumers (maproom.ts, worktrees.ts)
4. MCP package keeps separate implementation (different package concerns)

**Principles:**
- Follow existing config patterns (Zod schema, optional fields)
- Reuse existing utilities (fs, path, spawnSync)
- Maintain backwards compatibility (all existing paths continue to work)
- Fail gracefully with clear error messages

## Design Decisions

### Decision 1: Add to RepositorySchema vs New Top-Level Section

**Context:** Binary path could be added to `repository` section or create a new `tools` section for tool configurations.

**Decision:** Add `maproomBinaryPath` to existing `RepositorySchema`.

**Rationale:**
- Follows existing pattern (worktreeBasePath is in repository section)
- Avoids creating a new top-level section for one field
- Repository scope is appropriate (different repos might use different builds)
- Simpler migration path (users already configure repository section)

**Schema Change:**
```typescript
export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
  maproomBinaryPath: z.string().optional(),
})
```

### Decision 2: Resolution Order Priority

**Context:** Current implementations prioritize packaged binaries before global installs, which causes confusion when developers have both.

**Decision:** New priority order:
1. `CREWCHIEF_MAPROOM_BIN` environment variable
2. `config.repository.maproomBinaryPath`
3. Global install (`command -v crewchief-maproom`)
4. Packaged binary (platform-specific in bin/)

**Rationale:**
- Environment variable: Highest priority for one-off overrides
- Config file: Persistent, version-controllable, team-shareable
- Global install: Most common production deployment
- Packaged binary: Fallback for bundled distributions

**Behavior Change:** This changes the order for users who have both global and packaged binaries. Global install now takes precedence, which better matches production expectations.

**Impact on Users:**
- Users with only global install: No change
- Users with only packaged binary: No change
- Users with both: Will now use global install instead of packaged (intentional improvement)
- Users can override with environment variable if needed

### Decision 3: Shared Utility Location

**Context:** Binary resolution is needed in both CLI (maproom.ts, worktrees.ts) and potentially other CLI utilities.

**Decision:** Create `packages/cli/src/utils/maproom-binary.ts` with shared resolution function.

**Rationale:**
- CLI package owns the maproom command integration
- Utils directory is established pattern for shared utilities
- Can be imported by both maproom.ts and worktrees.ts
- MCP package has different concerns (includes dev build detection) and stays separate

**API:**
```typescript
export interface MaproomBinaryOptions {
  configPath?: string  // from config.repository.maproomBinaryPath
}

export function findMaproomBinary(options?: MaproomBinaryOptions): string | null
```

### Decision 4: MCP Package Independence

**Context:** MCP package has similar but distinct binary resolution needs (includes development build detection).

**Decision:** Keep MCP `findMaproomBinary()` separate, do NOT consolidate with CLI utility.

**Rationale:**
- Different package concerns (MCP is published separately)
- MCP includes dev build detection (`target/release/`) that CLI doesn't need
- MCP doesn't use crewchief config system (runs standalone via stdio)
- Avoiding cross-package coupling reduces fragility
- Small code duplication is acceptable for package independence

### Decision 5: Path Validation Strategy

**Context:** Config path might be invalid, relative, or non-existent.

**Decision:**
- Accept absolute or relative paths in config
- Resolve relative paths from config file location
- Check existence with fs.existsSync()
- Return null if configured path doesn't exist (fall through to next priority)
- Emit warning if configured path is invalid
- Handle missing config file gracefully (no error, fall through)

**Rationale:**
- Relative paths are more portable across environments
- Silent fallback matches existing behavior
- Warning helps debugging without breaking existing workflows
- Config file is optional - commands must work without it

**Example:**
If config is at `/workspace/subdir/crewchief.config.js` and specifies `maproomBinaryPath: "./bin/maproom"`, it resolves to `/workspace/subdir/bin/maproom`

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Validation | Zod | Already used for config validation |
| File system | Node fs | Built-in, matches existing code |
| Path resolution | Node path | Built-in, handles platform differences |
| Binary detection | spawnSync + bash | Existing pattern for global binary detection |
| Config loading | Existing loadConfig() | Reuses established config infrastructure |

## Component Design

### 1. Config Schema Extension

**File:** `packages/cli/src/config/schema.ts`

**Change:**
```typescript
export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
  maproomBinaryPath: z.string().optional(), // NEW
})
```

**Validation:**
- Optional field (no breaking change)
- String type (file path)
- No format validation (Zod level) - handled at runtime

### 2. Binary Resolution Utility

**File:** `packages/cli/src/utils/maproom-binary.ts` (NEW)

**Interface:**
```typescript
export interface MaproomBinaryOptions {
  /** Path from config.repository.maproomBinaryPath */
  configPath?: string
}

export interface BinaryResolutionResult {
  /** Path to binary or null if not found */
  path: string | null
  /** Where the binary was found (for debugging/logging) */
  source: 'env' | 'config' | 'global' | 'packaged' | 'not-found'
}

/**
 * Find the crewchief-maproom binary using priority-based resolution.
 *
 * Priority order:
 * 1. CREWCHIEF_MAPROOM_BIN environment variable
 * 2. configPath option (from config.repository.maproomBinaryPath)
 * 3. Global install (command -v crewchief-maproom)
 * 4. Packaged binary (platform-specific in bin/)
 */
export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult
```

**Implementation Strategy:**
```typescript
export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult {
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'

  // Priority 1: Environment variable
  const envPath = process.env.CREWCHIEF_MAPROOM_BIN
  if (envPath && fs.existsSync(envPath)) {
    return { path: envPath, source: 'env' }
  }

  // Priority 2: Config file
  if (options?.configPath) {
    const resolved = path.resolve(options.configPath)
    if (fs.existsSync(resolved)) {
      return { path: resolved, source: 'config' }
    }
    // Warn but continue to fallback
    logger.warn(`Configured maproom binary not found at ${resolved}`)
  }

  // Priority 3: Global install
  const which = spawnSync('bash', ['-lc', 'command -v crewchief-maproom'])
  if (which.status === 0) {
    return { path: 'crewchief-maproom', source: 'global' }
  }

  // Priority 4: Packaged binary
  // Note: x64 maps to amd64 on some platforms, handle gracefully
  const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch
  const platform = `${process.platform}-${arch}`
  const __dirname = path.dirname(fileURLToPath(import.meta.url))

  const packagedPaths = [
    path.join(__dirname, '..', 'bin', platform, execName),
    path.join(__dirname, '..', 'bin', execName),
    path.join(__dirname, '..', '..', 'maproom-mcp', 'bin', platform, execName),
  ]

  for (const p of packagedPaths) {
    if (fs.existsSync(p)) {
      return { path: p, source: 'packaged' }
    }
  }

  return { path: null, source: 'not-found' }
}
```

### 3. Updated maproom.ts Consumer

**File:** `packages/cli/src/cli/maproom.ts`

**Change:**
```typescript
import { findMaproomBinary } from '../utils/maproom-binary.js'
import { loadConfig } from '../config/loader.js'
import { logger } from '../utils/logger.js'

async function runMaproomForward(args: string[]) {
  // ... validation logic ...

  // Load config to get binary path (handle missing config gracefully)
  let configPath: string | undefined
  try {
    const config = await loadConfig()
    configPath = config.repository.maproomBinaryPath
  } catch (error) {
    // Config file missing or invalid - continue with defaults
    logger.debug('No config file found, using default binary resolution')
  }

  const result = findMaproomBinary({ configPath })

  if (!result.path) {
    console.error(
      'crewchief-maproom not found. Options:\n' +
      '1. Install globally: npm install -g @crewchief/cli\n' +
      '2. Set CREWCHIEF_MAPROOM_BIN environment variable\n' +
      '3. Add maproomBinaryPath to crewchief.config.js\n\n' +
      'Resolution attempts:\n' +
      '- Environment: ' + (process.env.CREWCHIEF_MAPROOM_BIN || 'not set') + '\n' +
      '- Config: ' + (configPath || 'not configured') + '\n' +
      '- Global: not found\n' +
      '- Packaged: not found'
    )
    process.exitCode = 1
    return
  }

  const res = spawnSync(result.path, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

// Commander action handlers must be async
program
  .command('scan')
  .action(async (args) => await runMaproomForward(['scan', ...(args || [])]))
```

**Note:** Function is now async and Commander action handlers use async/await pattern.

### 4. Updated worktrees.ts Consumer

**File:** `packages/cli/src/git/worktrees.ts`

**Change:**
```typescript
import { findMaproomBinary } from '../utils/maproom-binary.js'

private async runMaproomScan(worktreePath: string): Promise<void> {
  try {
    const config = await loadConfig()
    const result = findMaproomBinary({
      configPath: config.repository.maproomBinaryPath
    })

    if (!result.path) {
      console.log('⚠️  Maproom binary not found, skipping indexing for new worktree')
      return
    }

    console.log('🔍 Running maproom scan for new worktree...')

    const scanResult = spawnSync(result.path, ['scan'], {
      cwd: worktreePath,
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    // ... rest of output handling ...
  } catch (error) {
    console.warn('⚠️  Failed to run maproom scan:', error instanceof Error ? error.message : error)
  }
}
```

## Data Flow

1. **User configures binary path** (optional):
   ```javascript
   // crewchief.config.local.js
   export default {
     repository: {
       maproomBinaryPath: './target/release/crewchief-maproom'
     }
   }
   ```

2. **CLI command invoked**:
   ```bash
   crewchief maproom scan
   # or
   crewchief worktree create feature-branch
   ```

3. **Config loaded** via existing loadConfig():
   - Traverses directory tree
   - Finds crewchief.config.js or crewchief.config.local.js
   - Validates with Zod
   - Returns parsed config

4. **Binary resolution** via findMaproomBinary():
   - Check CREWCHIEF_MAPROOM_BIN env var
   - Check config.repository.maproomBinaryPath
   - Check global install (command -v)
   - Check packaged binary locations
   - Return path + source

5. **Binary execution**:
   - spawnSync with resolved path
   - Forward args and stdio
   - Return exit code

## Integration Points

### 1. Existing Config System

**Integration:** Extends RepositorySchema with new optional field

**Touch Points:**
- `packages/cli/src/config/schema.ts` - Schema definition
- `packages/cli/src/config/loader.ts` - No changes needed (already returns parsed config)
- User's `crewchief.config.js` - Optional new field

**Compatibility:** Fully backwards compatible (optional field)

### 2. CLI Commands

**Integration:** Import and use findMaproomBinary() utility

**Touch Points:**
- `packages/cli/src/cli/maproom.ts` - Replace resolvePackagedMaproomBin()
- `packages/cli/src/git/worktrees.ts` - Replace inline resolution logic

**Migration:** Remove ~86 lines of duplicated code, replace with utility calls

### 3. MCP Package (NO CHANGES)

**Integration:** None - MCP package remains independent

**Rationale:** Different package concerns, different deployment model

## Performance Considerations

**Binary Resolution Performance:**
- File existence checks: O(1) for each path checked (4-6 checks max)
- Global binary detection: One bash subprocess (20-50ms overhead)
- Total overhead: <100ms per CLI invocation (acceptable for CLI tool)

**Caching Strategy:**
- NO caching needed (resolution is fast enough)
- Each command resolves fresh (handles binary updates mid-session)
- Config loading overhead: <50ms per command (acceptable for CLI)
- Missing config is caught and handled, no performance impact

**Optimization:**
- Early returns (env var checked first, exits on match)
- Packaged paths checked last (least likely in production)

## Maintainability

### Code Organization

**Before:**
- 3 separate implementations (~140 lines total)
- Subtle differences in logic
- No single source of truth

**After:**
- 1 shared utility (~60 lines)
- 2 simple call sites (~10 lines each)
- Clear contract via TypeScript interface

### Testing Strategy

**Unit Tests:** `packages/cli/tests/utils/maproom-binary.test.ts`
- Test precedence order (env > config > global > packaged)
- Test platform handling (Windows .exe vs Unix)
- Test relative path resolution
- Test missing binary handling
- Mock fs.existsSync and spawnSync

**Integration Tests:** Existing tests should pass
- `packages/cli/tests/integration/maproom-commands.int.test.ts`
- Environment variable tests in MCP package

### Error Messages

**Clear guidance for users:**
```
crewchief-maproom not found. Options:
1. Install globally: npm install -g @crewchief/cli
2. Set CREWCHIEF_MAPROOM_BIN environment variable
3. Add maproomBinaryPath to crewchief.config.js:

   export default {
     repository: {
       maproomBinaryPath: './path/to/crewchief-maproom'
     }
   }
```

### Documentation Updates

**Files to update:**
1. `README.md` - Add config option to main documentation
2. `docs/development/local-development.md` - Update development workflow
3. `packages/cli/README.md` - Document repository config options
4. Example config files (if any)

## Risks and Mitigations

### Risk 1: Breaking Existing Workflows

**Mitigation:**
- All existing paths continue to work (backwards compatible)
- Change in priority order is intentional improvement
- Comprehensive testing of existing scenarios

### Risk 2: Path Resolution Edge Cases

**Mitigation:**
- Handle relative paths explicitly
- Warn on invalid config paths
- Fall through to next priority on failure

### Risk 3: Platform Differences

**Mitigation:**
- Reuse existing platform detection logic
- Test on Windows, macOS, Linux
- Handle .exe suffix consistently

### Risk 4: Async Config Loading in Sync Context

**Mitigation:**
- Config loading is already async in current code
- Make runMaproomForward async (already in async context via Commander)
- Document that binary resolution requires config loading
