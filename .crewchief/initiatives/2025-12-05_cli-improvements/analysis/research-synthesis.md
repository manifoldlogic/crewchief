# Research Synthesis: cli improvements

## Key Findings

### 1. Current Worktree Implementation
**Location:** `packages/cli/src/git/worktrees.ts`

- **Auto-scan is hardcoded:** Line 143 unconditionally calls `runMaproomScan()` after worktree creation
- **Binary resolution prefers local:** Lines 40-53 check packaged binaries before global installation
- **No cleanup integration:** `removeWorktree()` only calls git commands, no maproom cleanup
- **Path handling:** Uses `path.join()` for worktree paths, no `~` expansion implemented

**Evidence:**
```typescript
// Line 143 - Always scans after creation
await this.runMaproomScan(wtPath)

// Lines 40-53 - Binary resolution order
const possiblePaths = [
  path.join(__dirname, '..', 'bin', platform, execName),
  path.join(__dirname, '..', 'bin', execName),
  path.join(__dirname, '..', '..', 'maproom-mcp', 'bin', platform, execName),
]
```

### 2. Configuration Schema
**Location:** `packages/cli/src/config/schema.ts`

- **Current worktree base path:** Default is `.crewchief/worktrees` (line 5)
- **No maproom binary config:** No schema field for `maproomBinaryPath`
- **No auto-scan config:** No schema field for `autoScanOnWorktreeUse`
- **Zod validation:** All config goes through `ConfigSchema.parse()`

**Schema structure:**
```typescript
export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
})
```

### 3. Maproom Cleanup Capability
**Location:** `crates/maproom/src/db/cleanup.rs`, `crates/maproom/src/db/sqlite/mod.rs`

- **Delete method exists:** `delete_worktree_data(worktree_id)` removes all related records
- **CLI integration:** `main.rs` line 931 shows cleanup command calls `delete_worktree_data()`
- **Transaction-based:** Cleanup uses SQLite transactions for atomicity
- **Safe deletion:** Only deletes if worktree path no longer exists (stale detection)

**Evidence:**
```rust
// crates/maproom/src/db/sqlite/mod.rs:1608
pub async fn delete_worktree_data(
    &self,
    worktree_id: i64,
) -> Result<(), Box<dyn Error + Send + Sync>>
```

### 4. Daemon Client Support
**Location:** `packages/daemon-client/src/client.ts`

- **No delete_worktree method yet:** Client has `search`, `context`, `status` but not cleanup
- **Could be added:** Would follow same pattern as existing methods
- **Alternative:** Call maproom binary directly with `spawnSync`

**Current methods:**
- `ping()` - health check
- `search()` - semantic search
- `context()` - get related code
- `status()` - repo/worktree stats

### 5. Git Branch Cleanup
**Location:** `packages/cli/src/git/merge.ts`

- **Branch deletion exists:** `GitMergeService.deleteBranch()` method available
- **Used in merge flow:** Line 562 shows worktree merge deletes branch after merge
- **Force deletion:** Uses `git branch -D` to force delete unmerged branches

**Evidence:**
```typescript
// Line 562 in worktree.ts merge command
await mergeService.deleteBranch(worktreeBranch)
```

### 6. Environment Variable Usage
**Current pattern:** `CREWCHIEF_MAPROOM_BIN` is checked in multiple places:
- `packages/cli/src/git/worktrees.ts:35`
- `packages/cli/src/cli/maproom.ts:15`
- `packages/maproom-mcp/src/utils/process.ts:85`

**Implication:** Adding config file option should follow same precedence (env var takes priority over config)

### 7. Path Expansion Patterns
**Best practice from Node.js ecosystem:**
```typescript
import os from 'node:os'
import path from 'node:path'

function expandPath(inputPath: string): string {
  if (inputPath.startsWith('~')) {
    return path.join(os.homedir(), inputPath.slice(1))
  }
  return path.resolve(inputPath)
}
```

### 8. Repository Name Detection
**From git config:**
```typescript
const repoUrl = await git.getConfig('remote.origin.url')
const repoName = repoUrl.split('/').pop()?.replace('.git', '') || 'unknown'
```

## Open Questions

### Q1: Should maproomBinaryPath be in repository or worktree config section?
**Analysis:** It's a tool dependency, not worktree-specific. Should go in top-level or repository section.

**Recommendation:** Add `repository.maproomBinaryPath` for consistency with other binary paths.

### Q2: How to handle worktree path template variables?
**Options:**
- Simple string interpolation: `~/.crewchief/worktrees/{repo}/`
- Template function in config: `(repo, branch) => path`
- Fixed default with manual override

**Recommendation:** Use fixed default `~/.crewchief/worktrees/<repo-name>/` with manual override via config.

### Q3: Should clean command delete branch by default or require flag?
**Considerations:**
- Pro default deletion: Matches user intent (remove everything)
- Con default deletion: Might delete work if not merged

**Recommendation:** Delete branch by default, but check if merged and warn. Add `--keep-branch` flag for override.

### Q4: How to call maproom cleanup - daemon or direct binary?
**Options:**
1. Direct binary spawn (simpler, no daemon dependency)
2. Daemon client (faster if daemon running)
3. Hybrid (try daemon, fallback to binary)

**Recommendation:** Start with direct binary spawn (simpler, fewer failure modes). Future enhancement can add daemon support.

### Q5: Should auto-scan config be per-worktree or global?
**Analysis:** Global setting is simpler and matches user workflow preference. Per-worktree is more flexible but adds complexity.

**Recommendation:** Start with global `worktree.autoScanOnWorktreeUse` config. Can add per-worktree override later if needed.

## Assumptions

### A1: Users want centralized worktree storage
**Assumption:** Most developers prefer worktrees outside the repo to avoid clutter.

**Validation needed:** Survey existing users or add telemetry to see current usage patterns.

**Risk:** Low - backward compatibility maintained via config.

### A2: Auto-scan should be opt-in, not opt-out
**Assumption:** Automatic scanning is unexpected behavior and should require explicit enablement.

**Validation needed:** Check if any users rely on auto-scan behavior.

**Risk:** Medium - could disrupt existing workflows if users expect auto-scan.

### A3: Maproom binary will be globally installed for production use
**Assumption:** Developers using crewchief-cli in production will have maproom binary in PATH.

**Validation needed:** Check installation docs and npm package setup.

**Risk:** Low - fallback to packaged binary still works.

### A4: Complete cleanup is always desired
**Assumption:** When users run `clean`, they want all traces removed (directory, branch, database).

**Validation needed:** Monitor for issues after implementation.

**Risk:** Medium - users might want to keep branch for reference.

### A5: Path expansion works consistently across platforms
**Assumption:** Node.js `os.homedir()` returns correct home directory on all platforms.

**Validation needed:** Test on Windows, macOS, Linux.

**Risk:** Low - widely used pattern in Node.js ecosystem.

## Technical Constraints

### C1: Zod Schema Compatibility
All config changes must maintain Zod schema validation:
- New fields must have defaults for backward compatibility
- Validation errors should be clear and actionable
- Type safety must be preserved in TypeScript

### C2: Git Worktree Limitations
Git worktree operations have constraints:
- Cannot remove worktree if it's the current directory
- Branch must exist before creating worktree
- Worktree paths must be unique

### C3: Maproom Database Consistency
Database operations must handle edge cases:
- Worktree record might not exist (user didn't scan)
- Multiple worktrees might reference same branch
- Database might be locked during cleanup

### C4: Cross-platform Path Handling
Path operations must work on:
- Windows: `C:\Users\...`, backslash separators
- macOS/Linux: `/home/...`, forward slash separators
- Windows WSL: Hybrid path handling

## Implementation Guidance

### Phase 1: Configuration Foundation (Project 1 + 4)
1. Add config schema fields with defaults
2. Implement path expansion utility
3. Update binary resolution logic
4. Test config validation

### Phase 2: Behavior Changes (Project 2)
1. Add auto-scan config check
2. Modify `worktree use` to respect config
3. Update tests for new behavior
4. Document breaking change

### Phase 3: Enhanced Cleanup (Project 3)
1. Extract maproom binary finder to shared utility
2. Implement worktree-to-repo-name detection
3. Add maproom cleanup call to clean command
4. Add branch deletion after cleanup
5. Test failure scenarios

### Phase 4: Documentation & Migration
1. Update CLI README with new defaults
2. Add migration guide for existing users
3. Update config example file
4. Add changelog entry
