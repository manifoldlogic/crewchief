# Architecture: Worktree Use Auto-Scan Control

## High-Level Overview

This project adds a single configuration field to control auto-scan behavior during worktree operations. The architecture is intentionally minimal - we're adding a config flag and a conditional check, not building new infrastructure.

**Core Change**: Replace unconditional `runMaproomScan()` call with config-gated execution.

**Affected Components**:
1. **Config Schema** (`packages/cli/src/config/schema.ts`) - Add field
2. **WorktreeService** (`packages/cli/src/git/worktrees.ts`) - Check config before scan
3. **Tests** (`packages/cli/src/cli/__tests__/worktree-create.test.ts`) - Verify behavior
4. **Documentation** (`packages/cli/README.md`) - Explain option and migration

## Key Design Decisions

### Decision 1: Config Field Name and Default

**Options Considered**:
1. `autoScanOnWorktreeUse: false` (opt-in)
2. `skipAutoScan: false` (opt-out via double negative)
3. `enableAutoScan: false` (verbose but clear)

**Choice**: `autoScanOnWorktreeUse: false`

**Rationale**:
- Positive naming (clearer than double negatives)
- Specific scope ("on worktree use" vs just "auto scan")
- Default `false` makes fast operations the default
- Matches existing pattern: `copyIgnoredFiles` is also optional/default-off

**Trade-off**: Breaking change, but necessary for better UX.

### Decision 2: Scope of Change

**Options Considered**:
1. Add only to `worktree create`
2. Add to both `create` and `use`
3. Add separate flags for each command

**Choice**: Add only to `worktree create`

**Rationale**:
- The `worktree use` command doesn't call `createWorktree()` - it only switches between existing worktrees
- `worktree use` never triggers scanning in current implementation (lines 210-277 in packages/cli/src/cli/worktree.ts)
- Simplest and most accurate scope: control scanning during worktree creation only
- No changes needed to `worktree use` command

### Decision 3: Implementation Strategy

**Options Considered**:
1. Add boolean parameter to `createWorktree()` method
2. Read config inside `runMaproomScan()` method
3. Read config in `createWorktree()` and conditionally call scan

**Choice**: Read config in `createWorktree()`, conditionally call scan

**Rationale**:
- Keeps `runMaproomScan()` pure (no config dependency)
- Config check at call site is explicit and testable
- Matches existing pattern (see `copyIgnoredFiles` check, lines 128-142)
- Clear separation: create logic vs scan logic

### Decision 4: Backward Compatibility Strategy

**Options Considered**:
1. Default to `true` (maintain current behavior)
2. Default to `false` (breaking change, faster ops)
3. Auto-detect based on repo size
4. Deprecation period with warnings

**Choice**: Default to `false` with clear migration documentation

**Rationale**:
- Fast worktree operations are more important than auto-indexing
- Clear breaking change is better than complex auto-detection
- Migration is trivial (one line in config file)
- Deprecation period delays the fix unnecessarily

**Breaking Change Communication**:
- Prominent changelog entry
- Migration guide in README
- Example config in documentation
- Clear explanation of trade-offs

## Technology Choices

### Zod Schema Extension

**Why Zod**: Already used throughout codebase for config validation.

**Implementation**:
```typescript
export const WorktreeSchema = z.object({
  copyIgnoredFiles: z.array(z.string()).optional(),
  copyFromPath: z.string().default('.'),
  overwriteStrategy: z.enum(['skip', 'overwrite', 'backup']).default('skip'),
  autoScanOnWorktreeUse: z.boolean().default(false), // NEW FIELD
})
```

**Type Safety**: TypeScript infers type from schema - no manual type definitions needed.

### No New Dependencies

**Why**: This change requires zero new dependencies. We use:
- Existing config loading (`loadConfig()`)
- Existing Zod schema
- Existing WorktreeService methods

**Benefit**: Minimal risk, fast review, easy rollback if needed.

## Component Design

### 1. Config Schema Update

**File**: `packages/cli/src/config/schema.ts`

**Change**:
```typescript
export const WorktreeSchema = z.object({
  copyIgnoredFiles: z.array(z.string()).optional(),
  copyFromPath: z.string().default('.'),
  overwriteStrategy: z.enum(['skip', 'overwrite', 'backup']).default('skip'),
  autoScanOnWorktreeUse: z.boolean().default(false), // Add this line
})
```

**Impact**:
- Type `CrewChiefConfig` automatically includes new field
- Default value ensures backward compatibility (no scan by default)
- Optional field means users don't have to set it

### 2. WorktreeService Integration

**File**: `packages/cli/src/git/worktrees.ts`

**Current Code** (lines 128-145):
```typescript
// Copy ignored files if configured and not skipped
if (!skipCopyIgnored) {
  try {
    const config = await loadConfig()
    if (config.worktree?.copyIgnoredFiles?.length) {
      console.log('\n📁 Copying ignored files to worktree...')
      await copyIgnoredFiles({
        sourceRoot: this.cwd,
        worktreeRoot: wtPath,
        config,
      })
    }
  } catch (error) {
    console.warn('⚠️  Failed to copy ignored files:', error instanceof Error ? error.message : error)
  }
}

// Run maproom scan to index the new worktree
await this.runMaproomScan(wtPath)
```

**New Code**:
```typescript
// Load config once for both operations
let config: CrewChiefConfig | null = null
try {
  config = await loadConfig()
} catch (error) {
  console.warn('⚠️  Failed to load config:', error instanceof Error ? error.message : error)
}

// Copy ignored files if configured and not skipped
if (!skipCopyIgnored && config?.worktree?.copyIgnoredFiles?.length) {
  try {
    console.log('\n📁 Copying ignored files to worktree...')
    await copyIgnoredFiles({
      sourceRoot: this.cwd,
      worktreeRoot: wtPath,
      config,
    })
  } catch (error) {
    console.warn('⚠️  Failed to copy ignored files:', error instanceof Error ? error.message : error)
  }
}

// Run maproom scan if configured
if (config?.worktree?.autoScanOnWorktreeUse) {
  await this.runMaproomScan(wtPath)
}
```

**Design Notes**:
- Load config once, reuse for both operations (clean and efficient)
- Config loading errors don't block worktree creation (null config used as fallback)
- Keep `runMaproomScan()` unchanged (no modification needed)
- Consistent error handling with single try-catch for config load

### 3. Test Updates

**File**: `packages/cli/src/cli/__tests__/worktree-create.test.ts`

**New Tests to Add**:

```typescript
describe('auto-scan behavior', () => {
  it('skips maproom scan by default', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      worktree: undefined, // No worktree config
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.runMaproomScan).not.toHaveBeenCalled()
  })

  it('runs maproom scan when autoScanOnWorktreeUse is true', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      worktree: { autoScanOnWorktreeUse: true },
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.runMaproomScan).toHaveBeenCalledWith('/path/to/worktrees/feature-x')
  })

  it('skips maproom scan when autoScanOnWorktreeUse is false', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      worktree: { autoScanOnWorktreeUse: false },
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.runMaproomScan).not.toHaveBeenCalled()
  })

  it('handles config loading errors gracefully', async () => {
    vi.mocked(loadConfig).mockRejectedValue(new Error('Config read failed'))

    // Should still create worktree successfully
    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalled()
    expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining('Failed to check auto-scan config'))
  })
})
```

**Test Strategy**:
- Mock `runMaproomScan` to verify it's called/not called
- Test all config combinations: undefined, true, false
- Verify error handling doesn't break worktree creation
- Maintain existing test coverage

### 4. Documentation Updates

**File**: `packages/cli/README.md`

**Section to Add** (after "Semantic Code Search" section, around line 70):

```markdown
#### Auto-Scan Configuration

By default, worktree operations do NOT automatically trigger maproom indexing. This keeps worktree creation fast.

To enable auto-scan when creating or switching worktrees, add to your `crewchief.config.js`:

```javascript
export default {
  worktree: {
    autoScanOnWorktreeUse: true, // Enable auto-indexing
  },
}
```

**Trade-offs**:
- **Auto-scan enabled**: New worktrees are immediately searchable, but creation is slower (5-30s depending on repo size)
- **Auto-scan disabled** (default): Fast worktree operations, but you must manually run `crewchief maproom scan` when needed

**Manual scanning**:
```bash
# After creating a worktree, index it manually:
crewchief maproom scan
```

**Migration from older versions**: If you relied on automatic scanning, simply add `autoScanOnWorktreeUse: true` to your config.
```

## Data Flow

### Before (Current Behavior)

```
User runs: crewchief worktree create feature-x
    ↓
Load config
    ↓
WorktreeService.createWorktree()
    ├─> Create git worktree
    ├─> Save metadata
    ├─> Copy ignored files (if configured)
    └─> runMaproomScan() [ALWAYS RUNS] ← Problem
        └─> 5-30 second delay
    ↓
Return path to user
```

### After (New Behavior)

```
User runs: crewchief worktree create feature-x
    ↓
Load config
    ↓
WorktreeService.createWorktree()
    ├─> Create git worktree
    ├─> Save metadata
    ├─> Copy ignored files (if configured)
    └─> Check config.worktree.autoScanOnWorktreeUse
        ├─> If true: runMaproomScan() [5-30s delay]
        └─> If false (default): skip [instant]
    ↓
Return path to user [FAST]
```

## Integration Points

### 1. Config Loading

**Integration**: `loadConfig()` from `packages/cli/src/config/loader.ts`

**Behavior**:
- Returns parsed and validated config
- Throws on validation errors
- Must be caught to prevent worktree creation failure

**Safety**: Wrap config check in try-catch to ensure worktree creation always succeeds.

### 2. WorktreeService Methods

**Integration**: `createWorktree()` method already exists

**Modification**: Add conditional scan logic before return statement

**No Changes Needed**:
- `runMaproomScan()` - works as-is
- `initRepository()` - unchanged
- Other methods - unchanged

### 3. CLI Commands

**No Changes Required**: The `worktree create` and `worktree use` commands in `packages/cli/src/cli/worktree.ts` call `WorktreeService.createWorktree()`, which will now handle the conditional scan internally.

**Benefit**: Command layer stays clean, logic lives in service layer.

## Error Handling Strategy

### Config Loading Errors

**Scenario**: Config file is malformed or unreadable

**Handling**:
```typescript
try {
  const config = await loadConfig()
  if (config.worktree?.autoScanOnWorktreeUse) {
    await this.runMaproomScan(wtPath)
  }
} catch (error) {
  console.warn('⚠️  Failed to check auto-scan config:', error.message)
  // Continue - worktree is still created successfully
}
```

**Principle**: Config errors should never break worktree creation.

### Scan Errors

**Scenario**: Maproom binary not found or scan fails

**Handling**: Already handled in `runMaproomScan()` method - prints warnings but doesn't throw.

**No Change Needed**: Existing error handling is sufficient.

### Validation Errors

**Scenario**: User sets `autoScanOnWorktreeUse: "yes"` (wrong type)

**Handling**: Zod schema validation catches this at config load time, provides clear error message.

**No Change Needed**: Zod handles validation automatically.

## Performance Considerations

### Impact Analysis

**Before**:
- Worktree creation: 5-30 seconds (always includes scan)
- User experience: Feels slow/broken on large repos

**After** (default behavior):
- Worktree creation: <1 second (no scan)
- User experience: Instant, as expected

**After** (with auto-scan enabled):
- Worktree creation: 5-30 seconds (same as before)
- User experience: Same as current behavior, but user opted in

### Config Loading Overhead

**Cost**: ~1-5ms to load and parse config file

**Mitigation**: Accept this minimal cost for the benefit of user control

**Future Optimization**: Cache config in memory if multiple worktree operations happen in sequence (not in scope for MVP).

## Security Considerations

See [security-review.md](security-review.md) for detailed analysis.

**Summary**:
- Config validation via Zod prevents injection attacks
- File path handling already secured in existing code
- No new security surfaces introduced
- LOW risk overall

## Deployment Strategy

### Phase 1: Implementation

1. Update config schema
2. Add conditional scan logic
3. Update tests
4. Update documentation

### Phase 2: Testing

1. Run existing test suite (ensure no regression)
2. Run new auto-scan tests
3. Manual testing with config variations

### Phase 3: Documentation

1. Update README with clear examples
2. Add migration guide to changelog
3. Document trade-offs clearly

### Phase 4: Release

1. Version bump with breaking change notation (likely minor version, with clear notes)
2. Release notes highlighting:
   - Breaking change
   - Migration path (one-line config change)
   - Performance improvement for default case
   - Rationale for change

## Rollback Plan

**If Issues Arise**:

1. **Quick Fix**: Change default from `false` to `true` in schema
   - One-line change
   - Restores old behavior immediately
   - Buys time for proper fix

2. **Full Rollback**: Revert entire change
   - Remove config field
   - Remove conditional logic
   - Restore unconditional scan
   - Document in changelog

**Risk**: Low - change is minimal and isolated to one service method.

## Future Enhancements

**Not in Scope for MVP**, but natural extensions:

1. **CLI Flag**: `--scan` / `--no-scan` flag to override config per-command
2. **Purpose-Based Scanning**: Auto-scan agent worktrees but not manual ones
3. **Lazy Scanning**: Scan on first search attempt rather than creation
4. **Background Scanning**: Queue scan to run after worktree creation completes
5. **Smart Defaults**: Disable auto-scan for small changes, enable for full clones

**Principle**: Ship simple MVP first, iterate based on user feedback.
