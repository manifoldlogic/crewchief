# Analysis: Maproom Binary Configuration

## Problem Definition

The CrewChief CLI has **partial** support for configuring the maproom binary path through `repository.maproomBinaryPath`, but the implementation is **incomplete and inconsistent**:

1. **Inconsistent usage across call sites**: `cleanMaproomRecords()` in `worktrees.ts:242` calls `findMaproomBinary()` without passing config options, bypassing the config-based resolution entirely.

2. **Missing config file location context**: The `configFileLocation` parameter for relative path resolution is not consistently passed to all call sites.

3. **Documentation gaps**: While README.md mentions the config option (lines 220-242), the development workflow documentation in `docs/development/local-development.md` only covers environment variable methods (lines 102-125), not the config approach.

4. **Redundant implementation in MCP package**: The `packages/maproom-mcp/src/utils/process.ts` has a separate `findMaproomBinary()` that doesn't support config paths and has different resolution logic (checking dev paths instead of global install).

**Concrete impact**: Developers using `maproomBinaryPath` in their config will have it ignored by `crewchief worktree:clean` and other maproom database operations.

## Context

### Why This Work Is Needed

**Primary use case**: Local Rust development
- Developers working on maproom (Rust) code need the CLI to use their locally-built binary
- Current solution requires setting `CREWCHIEF_MAPROOM_BIN` environment variable each time
- Config-based approach allows persistent setting in `crewchief.config.local.js` (gitignored)

**Secondary benefits**:
- CI/CD environments can specify exact binary versions
- Custom binary paths for testing/validation workflows
- Simpler development workflow (no env var management)

### Current State

**CLI Package** (`packages/cli/src/utils/maproom-binary.ts`):
- ✅ Schema field exists: `maproomBinaryPath: z.string().optional()` (line 19 in schema.ts)
- ✅ Resolution logic implemented: env → config → global → packaged (lines 27-89)
- ✅ Relative path support with `configFileLocation` parameter (lines 40-52)
- ⚠️ Used correctly in 2 of 3 call sites:
  - ✅ `src/cli/maproom.ts:37` - passes config path
  - ✅ `src/git/worktrees.ts:40` - passes config path in `runMaproomScan()`
  - ❌ `src/git/worktrees.ts:242` - `cleanMaproomRecords()` doesn't pass config

**MCP Package** (`packages/maproom-mcp/src/utils/process.ts`):
- Different resolution order: env → packaged → dev paths
- No config support at all
- No global install check
- Used by VSCode extension and standalone MCP server

**README.md** (lines 220-242):
- Documents the config option with example
- Shows resolution priority order
- Lists use cases

**docs/development/local-development.md** (lines 102-148):
- Documents env var methods (Methods 2 & 3)
- Missing: config-based method (would be Method 1)

## Existing Solutions

### Industry Patterns

**Binary resolution precedence** is standard across Node.js tooling:

1. **esbuild**: `ESBUILD_BINARY_PATH` env → npm bin → download on-demand
2. **node-gyp**: `npm_config_node_gyp` → bundled binary
3. **sharp**: Platform detection → bundled binary → optional install

**Config-based binary paths** are common in:
- TypeScript: `tsconfig.json` with `compilerOptions.paths`
- Prettier: `.prettierrc` with `resolveConfig`
- ESLint: `.eslintrc` with `resolvePlugins`

### Codebase Patterns

**CrewChief already follows best practices**:
- Zod for config validation (type-safe, runtime validation)
- Path expansion utilities: `expandWorktreePath()` handles tilde, placeholders
- Comprehensive error messages showing what was tried
- Test coverage: 26 existing test cases in `clean-maproom-records.test.ts`, plus comprehensive tests in `maproom-binary.test.ts`

**Similar config pattern** in `worktreeBasePath`:
```typescript
worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>')
```
- Supports tilde expansion, placeholders, relative/absolute paths
- Documented in schema with examples

## Research Findings

### Key Insights

1. **The feature is mostly done**: Schema, resolution logic, tests all exist
2. **The gap is integration**: One call site doesn't use it, docs incomplete
3. **MCP independence**: MCP package shouldn't depend on CLI config (runs as standalone daemon)
4. **Test coverage is excellent**: No new test infrastructure needed

### Code Analysis

**`findMaproomBinary()` function** (maproom-binary.ts:27-89):
```typescript
export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult {
  // 1. Check environment variable first
  // 2. Check config path (with relative path resolution)
  // 3. Check global installation
  // 4. Check packaged binary locations
}
```

**Return type provides debugging info**:
```typescript
interface BinaryResolutionResult {
  path: string | null
  source: 'env' | 'config' | 'global' | 'packaged' | 'not-found'
}
```

**Error message in `maproom.ts:40-54`** shows all resolution attempts:
```
Resolution attempts:
- Environment: [value]
- Config: [value]
- Global: [result]
- Packaged: [result]
```

## Constraints

### Technical Constraints

1. **Two separate packages with different contexts**:
   - CLI: Runs in project context, has access to crewchief.config.js
   - MCP: Runs as standalone daemon, may not be in a crewchief project

2. **Relative path resolution needs config file location**:
   - Must know where config file is to resolve relative paths
   - `loadConfig()` returns config object but not file path
   - Need to track config file path through call chain

3. **Platform compatibility**:
   - Windows: `.exe` suffix, backslash paths
   - Unix: No suffix, forward slash paths
   - Already handled by existing code

4. **Backwards compatibility**:
   - Env var must continue to work (highest priority)
   - Existing packaged binary fallback must work
   - No breaking changes to function signatures

### Business Constraints

1. **Effort: S (1 day)** - Must stay focused, no scope creep
2. **Non-breaking change** - Existing usage must continue working
3. **Development workflow priority** - Optimize for local Rust development

### Time Constraints

1. **Single day effort** - approximately 6-8 hours
2. **Comprehensive tests already exist** - focus on gaps only
3. **Documentation updates** - should be quick (1-2 examples)

## Success Criteria

### From Acceptance Criteria

1. ✅ **Config accepts `maproomBinaryPath` setting**
   - Already implemented in schema.ts:19
   - Zod validation already working

2. ⚠️ **Config path takes precedence over packaged binary**
   - Already correct in findMaproomBinary() logic
   - Need to verify all call sites use it

3. ✅ **Env var still takes highest precedence**
   - Already implemented and tested
   - Lines 30-35 in maproom-binary.ts

4. ⚠️ **Global install checked before local packaged binary**
   - Already correct in CLI (lines 58-62)
   - Wrong in MCP (no global check at all)
   - MCP shouldn't be changed (different context)

5. ❌ **Binary resolution is consistent across all commands**
   - Currently inconsistent: `cleanMaproomRecords()` doesn't use config
   - Need to fix this one call site

6. ❌ **Development workflow documented**
   - Partially documented in README.md
   - Missing from local-development.md
   - Need to add config-based method example

### Measurable Outcomes

1. **All 3 CLI call sites use config**:
   - `maproom.ts:37` ✅
   - `worktrees.ts:40` ✅
   - `worktrees.ts:242` ❌ → needs fix

2. **Documentation complete**:
   - README.md ✅ (already done)
   - local-development.md ✅ (Method 1 already documented, lines 76-100, may need minor verification)

3. **Tests pass**:
   - Existing 26 tests in clean-maproom-records.test.ts continue to pass ✅
   - Need 2-3 new tests specifically for config parameter passing in cleanMaproomRecords

4. **Config file location tracked**:
   - Decision: Won't pass configFileLocation to cleanMaproomRecords (out of scope for MVP)
   - Relative paths will be relative to CWD, not config file location
   - Absolute paths work fine (primary use case)
   - This limitation is acceptable for MVP and can be enhanced later if needed

## Known Gaps

### Questions to Answer

1. **How to get config file location?**
   - `loadConfig()` internally finds it via `findConfigFile()`
   - Could return `{ config, configPath }` instead of just config
   - Breaking change? No - return type would change from `CrewChiefConfig` to `{ config, path }`
   - **Decision**: Too invasive for scope, won't pass configFileLocation to cleanMaproomRecords
   - **Accepted limitation**: Relative paths in maproomBinaryPath will be relative to CWD, not config file
   - **Mitigation**: Use absolute paths or paths relative to CWD (primary use case anyway)

2. **Should MCP package be updated?**
   - MCP runs independently, may not have config access
   - Different resolution order is intentional (dev paths vs global)
   - **Decision**: Leave MCP as-is, it serves different use case

3. **Should we extract shared utility?**
   - Project summary mentions "shared utility extraction"
   - CLI and MCP have different contexts/needs
   - **Decision**: Not necessary - they're intentionally different

### Assumptions

1. **Primary users are CLI users** - MCP is IDE integration, different workflow
2. **Config file is in project root** - standard pattern, matches tsconfig.json discovery
3. **Relative paths are relative to config file** - matches industry standards
4. **Local development is the main use case** - production uses global install
5. **Test coverage is comprehensive** - no major test infrastructure changes needed

## Risk Assessment

### Low Risk Items
- Schema changes: None needed (already exists)
- Resolution logic: Already correct, well-tested
- Backwards compatibility: Only adding config passing, not changing behavior

### Medium Risk Items
- Missing config file location in some contexts
  - **Mitigation**: Make configFileLocation optional, only use for relative paths
- Documentation clarity
  - **Mitigation**: Add clear examples with comments

### High Risk Items
- None identified

### Overall Risk: **LOW**
- Changes are minimal and localized
- Comprehensive test suite catches regressions
- Additive change (no breaking modifications)
