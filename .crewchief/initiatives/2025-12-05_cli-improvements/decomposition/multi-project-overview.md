# Multi-Project Overview: cli improvements

## Context

Initiative created: 2025-12-05
Reference: .crewchief/initiatives/2025-12-05_cli-improvements/

## Projects (in execution order)

### 1. WTPATH - Configurable Worktree Paths
**Priority:** High | **Effort:** S (1-2 days) | **Dependencies:** None

Change default worktree location from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>/` with support for path expansion and custom configuration.

### 2. MRBIN - Maproom Binary Configuration
**Priority:** High | **Effort:** S (1 day) | **Dependencies:** None

Add config option for maproom binary path and update resolution order to check config before falling back to local build.

### 3. WTSCAN - Worktree Use Auto-Scan Control
**Priority:** Medium | **Effort:** S (1-2 days) | **Dependencies:** WTPATH (config schema)

Add `autoScanOnWorktreeUse` config option (default: false) to control whether `worktree use` triggers automatic maproom scanning.

### 4. WTCLEAN - Enhanced Worktree Clean
**Priority:** High | **Effort:** M (2-3 days) | **Dependencies:** MRBIN (binary resolution)

Extend `worktree clean` to delete git branch and maproom database records in addition to removing the directory and git worktree metadata.

## Execution Strategy

### Recommended Order (Sequential)
1. WTPATH - establishes config foundation
2. MRBIN - adds binary config (can run parallel with #1)
3. WTSCAN - uses config schema from #1
4. WTCLEAN - uses binary resolution from #2

**Timeline:** 6-8 days + 2 days integration/testing = 8-10 days total

### Parallel Option (2 developers)
**Track A:** WTPATH + MRBIN (2 days)
**Track B:** Wait for Track A, then WTSCAN + WTCLEAN (3-4 days)

**Timeline:** 5-6 days total

## Dependencies

```
WTPATH (Config Foundation)
  ↓ schema changes
WTSCAN (uses autoScanOnWorktreeUse config)

MRBIN (Binary Config)
  ↓ resolution logic
WTCLEAN (uses maproomBinaryPath config)
```

**Integration Points:**
- All projects modify config schema (must merge carefully)
- WTCLEAN uses path utilities from WTPATH (if extracted)
- WTCLEAN uses binary resolution from MRBIN

## Risk Assessment

| Project | Breaking Change | Platform Risk | Complexity |
|---------|----------------|---------------|------------|
| WTPATH  | Yes (default path) | Medium (path expansion) | Low |
| MRBIN   | No | Low | Low |
| WTSCAN  | Yes (removes auto-scan) | Low | Low |
| WTCLEAN | No (enhancement) | Low | Medium |

**Mitigation:**
- Provide migration guides for breaking changes
- Test path expansion on Windows/macOS/Linux
- Make cleanup best-effort (log errors, don't fail)

## Integration Requirements

### Shared Utilities
Consider extracting:
- Path expansion function (`expandPath()`) - WTPATH creates, WTCLEAN may use
- Binary resolution function (`findMaproomBinary()`) - MRBIN creates, WTCLEAN uses

### Testing Strategy
- Unit tests per project
- Integration test suite after all complete
- Manual verification on all platforms
- Test config validation with all new fields

### Documentation Updates
- Config schema documentation
- CLI README (new defaults, breaking changes)
- Migration guide for existing users
- CHANGELOG entries per project
