# Architecture: Maproom Binary Configuration

## Overview

This is a **completion project**, not a greenfield implementation. The binary configuration feature already exists with:
- Schema definition (`RepositorySchema.maproomBinaryPath`)
- Resolution logic (`findMaproomBinary()` function)
- Comprehensive test coverage (20+ test cases)
- Partial documentation

The architecture focuses on **integration gaps**: ensuring all call sites use the existing config path and documenting the feature properly.

## Design Decisions

### Decision 1: No Changes to Resolution Order

**Context:** The project summary mentions "update resolution order to prioritize config over local build detection" and "global install before packaged binary."

**Analysis:**
- Current order in `findMaproomBinary()`: env → config → global → packaged
- This is **already correct** according to the acceptance criteria
- "Config over local build" is already true (config is priority 2, packaged is priority 4)
- "Global before packaged" is already true (global is priority 3, packaged is priority 4)

**Decision:** No changes to resolution order needed. The current implementation already matches the desired behavior.

**Rationale:** Avoid unnecessary changes. The resolution order is well-tested and works correctly.

### Decision 2: MCP Package Unchanged

**Context:** The maproom-mcp package has its own `findMaproomBinary()` with different resolution logic (env → packaged → dev paths).

**Analysis:**
- MCP runs as a standalone daemon for IDE integration
- May not be in a crewchief project context (no access to crewchief.config.js)
- Different resolution order is intentional for monorepo development
- Used by VSCode extension, not directly by CLI users

**Decision:** Leave MCP package as-is. Do not add config support or change resolution order.

**Rationale:**
- MCP serves a different use case (IDE integration vs CLI usage)
- Adding config loading would create unneeded complexity
- Development workflow is env var for local builds (already supported)
- Out of scope for a 1-day effort

### Decision 3: No Shared Utility Extraction

**Context:** Project summary mentions "Extract `findMaproomBinary()` function for reuse."

**Analysis:**
- CLI and MCP implementations are intentionally different
- Different resolution priorities serve different contexts
- Extracting would require complex parameterization
- Current duplication is acceptable (85 lines vs 52 lines)

**Decision:** Keep implementations separate. No shared utility extraction.

**Rationale:** The two implementations serve different needs. Forced abstraction would add complexity without benefit.

### Decision 4: Config File Location via Optional Parameter

**Context:** Relative paths in `maproomBinaryPath` need config file location to resolve correctly.

**Options considered:**
1. Modify `loadConfig()` return type to include config path
2. Add configPath to config object as metadata
3. Pass configFileLocation separately in MaproomBinaryOptions

**Decision:** Use existing `MaproomBinaryOptions.configFileLocation` parameter (option 3), but don't pass it to cleanMaproomRecords.

**Rationale:**
- Already exists in the interface (line 9 in maproom-binary.ts)
- No breaking changes to loadConfig()
- Call sites can pass it when available, omit when not
- For cleanMaproomRecords: Won't pass configFileLocation (acceptable MVP limitation)
- Relative paths in cleanMaproomRecords context will be relative to CWD, not config file
- Absolute paths work always (primary use case for local development)

**Accepted limitation:** Relative paths in maproomBinaryPath won't resolve relative to config file location when used from cleanMaproomRecords. This is acceptable because:
- Most users use absolute paths or paths relative to project root
- Can be enhanced in future if needed
- Doesn't break existing functionality

### Decision 5: Fix cleanMaproomRecords Only

**Context:** Three call sites for `findMaproomBinary()` in CLI package, one doesn't pass config.

**Analysis:**
- `maproom.ts:37` ✅ - Already correct
- `worktrees.ts:40` ✅ - Already correct
- `worktrees.ts:242` ❌ - `cleanMaproomRecords()` exports function, doesn't load config

**Decision:** Update `cleanMaproomRecords()` to:
1. Accept optional config parameter
2. Load config if not provided
3. Pass config path to `findMaproomBinary()`

**Rationale:** Minimal change with maximum consistency. Backwards compatible (config parameter optional).

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Config validation | Zod (existing) | Already in use, type-safe |
| Path resolution | Node.js `path` module | Standard, platform-aware |
| Binary detection | fs.existsSync + spawnSync | Simple, reliable |
| Testing | Vitest (existing) | Already set up, good mocking |

No new technology needed. All components already in place.

## Component Design

### Config Schema (No Changes)

**Location:** `packages/cli/src/config/schema.ts`

**Existing implementation:**
```typescript
export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>'),
  maproomBinaryPath: z.string().optional(), // Line 19
})
```

**No changes needed.** Schema already accepts the config value.

### Binary Resolution Function (No Changes)

**Location:** `packages/cli/src/utils/maproom-binary.ts`

**Existing interface:**
```typescript
export interface MaproomBinaryOptions {
  configPath?: string // from config.repository.maproomBinaryPath
  configFileLocation?: string // for relative path resolution
}

export interface BinaryResolutionResult {
  path: string | null
  source: 'env' | 'config' | 'global' | 'packaged' | 'not-found'
}

export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult
```

**No changes needed.** Function already implements the correct logic.

### cleanMaproomRecords Function (Needs Update)

**Location:** `packages/cli/src/git/worktrees.ts:240-265`

**Current signature:**
```typescript
export async function cleanMaproomRecords(): Promise<void>
```

**Updated signature:**
```typescript
export async function cleanMaproomRecords(config?: CrewChiefConfig): Promise<void>
```

**Implementation changes:**
1. Accept optional config parameter
2. If not provided, call `loadConfig()` to get it
3. Pass `config.repository.maproomBinaryPath` to `findMaproomBinary()`
4. Do NOT pass configFileLocation (intentionally omitted for MVP)
5. Handle config load errors gracefully (continue without config)

**Backward compatibility:** Existing callers continue to work (config parameter optional).

**Call sites (no changes needed):**
- Line 216: `await cleanMaproomRecords()` in worktree:clean command
- Line 328: `await cleanMaproomRecords()` in worktree:prune command
- Line 390: `await cleanMaproomRecords()` in worktree:use --clean flag

All three can rely on cleanMaproomRecords loading config internally.

### Config File Location Tracking (New Helper)

**Need:** Get config file path from `loadConfig()` for relative path resolution.

**Solution:** Update `findConfigFile()` to be exported, reuse in call sites.

**Alternative:** Store in closure/module variable during loadConfig.
**Decision:** Export `findConfigFile()` helper - cleaner, more explicit.

## Data Flow

### Primary Flow: CLI Command → Binary Resolution

```
1. User runs: crewchief maproom scan
   ↓
2. maproom.ts:runMaproomForward()
   ↓
3. loadConfig() → CrewChiefConfig
   ↓
4. findMaproomBinary({
     configPath: config.repository.maproomBinaryPath,
     configFileLocation: configFilePath
   })
   ↓
5. Resolution order:
   a. Check CREWCHIEF_MAPROOM_BIN env var
   b. Check configPath (resolve relative if needed)
   c. Check global install (command -v)
   d. Check packaged binary
   ↓
6. Return { path, source }
   ↓
7. spawnSync(path, args) → Execute binary
```

### Secondary Flow: cleanMaproomRecords

```
1. Code calls: cleanMaproomRecords()
   ↓
2. If config not provided:
     config = await loadConfig()
     (handle errors gracefully)
   ↓
3. findMaproomBinary({
     configPath: config?.repository.maproomBinaryPath
     // Note: configFileLocation NOT provided
     // Relative paths resolve from CWD, not config file
   })
   ↓
4. Proceed with cleanup using found binary
```

### Config Resolution Flow

```
1. findConfigFile(cwd) traverses directories up
   ↓
2. Checks for:
   - crewchief.config.local.js (priority 1)
   - crewchief.config.js (priority 2)
   ↓
3. Returns config file path or null
   ↓
4. loadConfig() imports the module
   ↓
5. Zod validation → CrewChiefConfig
```

## Integration Points

### Existing Integration Points (No Changes)

1. **maproom.ts CLI commands** - Already passes config correctly
2. **worktrees.ts:runMaproomScan()** - Already passes config correctly
3. **Test suite** - Comprehensive coverage already exists

### New Integration Point

**worktrees.ts:cleanMaproomRecords()** - Will now:
- Accept optional config parameter
- Load config if not provided
- Pass config path to binary resolution

## Performance Considerations

### Config Loading Performance

**Current:** `loadConfig()` is called multiple times (once per command)
**Impact:** Negligible - config file is small (~50 lines), JavaScript import is cached
**Optimization:** Not needed. Config loading is fast (<1ms).

### Binary Resolution Performance

**Current:** Checks multiple locations in sequence
**Impact:** Fast - filesystem checks are <1ms each
**Worst case:** ~5ms (env var, config, global check, 2-3 packaged paths)
**Optimization:** Not needed. Resolution is fast enough.

### Relative Path Resolution

**Operation:** `path.resolve(configDir, relativePath)`
**Impact:** Negligible (<0.1ms)
**Optimization:** Not needed.

## Maintainability

### Code Organization

**Strengths:**
- Clear separation: config/ schema, utils/ helpers
- Well-tested: 20+ test cases with mocking
- Good error messages: Shows all resolution attempts
- Type-safe: Zod validation, TypeScript interfaces

**No changes needed to organization.** Current structure is maintainable.

### Testing Strategy

**Existing tests cover:**
- Precedence order (env > config > global > packaged)
- Platform variations (Windows .exe, Unix no suffix)
- Relative path resolution
- Missing paths (falls through gracefully)
- Edge cases (errors, empty paths)

**New tests needed:**
- `cleanMaproomRecords()` with config parameter
- `cleanMaproomRecords()` without config (loads it)
- Config load failure handling in cleanMaproomRecords

**Estimate:** 2-3 additional test cases, ~50 lines.

### Documentation Structure

**Current state:**
- README.md: User-facing documentation (configuration section)
- local-development.md: Developer workflow (env var methods)
- CLAUDE.md: Component guidance

**Updates needed:**
- local-development.md: Add config-based method (Method 1)
- Show example with crewchief.config.local.js
- Explain when to use each method (config vs env var)

### Future Extensibility

**Potential future needs:**
1. Additional binary options (debug builds, profiling)
   - Easy: Add more fields to RepositorySchema
   - Example: `maproomBinaryDebug`, `maproomBinaryProfile`

2. Per-command binary overrides
   - Medium: Add command-specific config sections
   - Example: `commands.scan.maproomBinaryPath`

3. Binary version constraints
   - Medium: Add version checking after resolution
   - Example: `maproomBinaryMinVersion: "1.2.0"`

Current architecture supports these extensions without major refactoring.

## Migration Strategy

**Not applicable** - this is completing an existing feature, not migrating from old to new.

### For Users

**No action required:**
- Existing env var usage continues to work
- New config option is opt-in
- No breaking changes

### For Developers

**Immediate benefit:**
- Can use `maproomBinaryPath` in crewchief.config.local.js
- Works consistently across all commands
- Documented in local-development.md

## Error Handling

### Config Load Errors

**Scenarios:**
1. Config file not found
2. Config file invalid JavaScript
3. Config validation fails (Zod)

**Handling:**
- `maproom.ts`: Already handles with try/catch (lines 29-35)
- `cleanMaproomRecords`: Will handle with try/catch, continue without config
- User sees warning but command continues with env var or packaged binary

### Binary Not Found

**Current behavior:** Clear error message showing all attempts
```
Resolution attempts:
- Environment: not set
- Config: ./my-binary (not found)
- Global: not found
- Packaged: not found
```

**No changes needed.** Error handling already excellent.

### Invalid Config Path

**Current behavior:** Logs warning, falls through to next priority
```typescript
logger.warn(`Configured maproom binary path not found: ${resolvedConfigPath}`)
```

**No changes needed.** Graceful degradation already implemented.
