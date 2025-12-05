# Domain Model: cli improvements

## Core Entities

### Worktree
**Definition:** A git working tree associated with a specific branch, stored in a filesystem directory.

**Attributes:**
- `path`: Filesystem location (string, can use `~` expansion)
- `branch`: Git branch name (string)
- `baseBranch`: Branch it was created from (string)
- `sourceBranch`: Original branch when created (stored in metadata)
- `createdAt`: Creation timestamp
- `purpose`: 'agent' | 'manual' (how it was created)

**States:**
- Active (in git worktree list)
- Stale (git metadata exists but directory missing)
- Orphaned (directory exists but git metadata removed)

**Lifecycle:** create → use → clean

### Configuration
**Definition:** User-specified settings controlling CLI behavior, validated by Zod schema.

**Hierarchy:**
1. System defaults (hardcoded in schema)
2. Project config (`crewchief.config.js`)
3. Local config (`crewchief.config.local.js`)

**Relevant Fields:**
- `repository.worktreeBasePath`: Where to store worktrees (string with path expansion)
- `repository.maproomBinaryPath`: Path to maproom binary (optional string)
- `worktree.autoScanOnWorktreeUse`: Whether to auto-scan (boolean, default: false)
- `worktree.copyIgnoredFiles`: Files to copy to new worktrees (string[])

### Maproom Worktree Record
**Definition:** Database record tracking indexed files and chunks for a specific worktree.

**Attributes:**
- `id`: Database primary key (integer)
- `repo_id`: Parent repository ID (foreign key)
- `name`: Worktree branch name (string)
- `path`: Filesystem path (string)
- `file_count`: Number of indexed files (integer)
- `chunk_count`: Number of indexed chunks (integer)

**Location:** SQLite database at `~/.maproom/maproom.db` (or `MAPROOM_DATABASE_URL`)

**Cleanup Method:** `delete_worktree_data(worktree_id)` - removes all chunks, files, and worktree record

### Maproom Binary
**Definition:** Rust executable (`crewchief-maproom`) that performs indexing and search operations.

**Resolution Order:**
1. `CREWCHIEF_MAPROOM_BIN` environment variable (if exists)
2. Config file `maproomBinaryPath` (NEW - if set)
3. Packaged binary in `packages/cli/bin/<platform>/`
4. Global installation (`command -v crewchief-maproom`)

**Capabilities:**
- `scan`: Index files in current directory
- `serve`: Start daemon for JSON-RPC communication
- `clean-ignored`: Remove ignored file chunks

## Boundaries

### What This Domain Controls
- Worktree storage location and path resolution
- Git worktree lifecycle (create, list, remove)
- Worktree metadata (creation context, source branch)
- Configuration schema for worktree-related settings
- Integration points with maproom (triggering scans, cleanup)

### What Other Domains Control
- **Git:** Low-level worktree operations (`git worktree add/remove/list`)
- **Maproom:** Indexing logic, database schema, embedding generation
- **Config Loader:** File discovery, parsing, validation, merging
- **Terminal:** How commands are invoked (iTerm2, headless)

### Integration Points
- **CLI → Git:** Via `simple-git` library and `git` CLI commands
- **CLI → Maproom:** Via binary spawning (`spawnSync`) or daemon client (future)
- **CLI → Config:** Via `loadConfig()` and Zod validation
- **Worktree → Metadata Service:** Via `WorktreeMetadataService` for `.crewchief-meta.json`

## Interactions

### Worktree Creation Flow
```
User command: crewchief worktree create <name>
  ↓
Load config (get worktreeBasePath, autoScanOnWorktreeUse)
  ↓
Expand path (~/.crewchief/worktrees/<repo-name>/ → /home/user/.crewchief/worktrees/myrepo/)
  ↓
Create directory structure
  ↓
Call git worktree add -B <name> <path> <baseBranch>
  ↓
Save worktree metadata (.crewchief-meta.json)
  ↓
[If autoScanOnWorktreeUse: true] Run maproom scan
  ↓
Return absolute path
```

### Worktree Use Flow
```
User command: crewchief worktree use <name>
  ↓
List existing worktrees (git worktree list)
  ↓
Match by branch name, basename, or path
  ↓
[If autoScanOnWorktreeUse: true] Run maproom scan
  ↓
Print path (for cd) or spawn subshell (if --shell)
```

### Worktree Clean Flow (Current)
```
User command: crewchief worktree clean <name>
  ↓
Match worktree by selector
  ↓
Call git worktree remove --force <path>
  ↓
Delete directory (removeDirSync)
  ↓
[Missing: Delete maproom record]
  ↓
[Missing: Delete git branch]
```

### Worktree Clean Flow (Enhanced - Proposed)
```
User command: crewchief worktree clean <name>
  ↓
Match worktree by selector
  ↓
Get worktree branch name
  ↓
Call git worktree remove --force <path>
  ↓
Delete directory (removeDirSync)
  ↓
[NEW] Find maproom binary (use maproomBinaryPath config)
  ↓
[NEW] Get repo name and worktree name from path
  ↓
[NEW] Call maproom delete_worktree_data (via CLI or daemon)
  ↓
[NEW] Delete git branch (git branch -D <branch>)
  ↓
Success message with all cleanup actions
```

### Configuration Resolution Flow
```
Load config request
  ↓
Search for crewchief.config.local.js (project root)
  ↓
If not found, search for crewchief.config.js
  ↓
If not found, use defaults from schema
  ↓
Validate with Zod schema
  ↓
Apply transformations:
  - Expand ~ in worktreeBasePath
  - Resolve maproomBinaryPath to absolute
  - Validate autoScanOnWorktreeUse is boolean
  ↓
Return typed config object
```

### Maproom Binary Resolution Flow (Current)
```
Need maproom binary
  ↓
Check CREWCHIEF_MAPROOM_BIN env var → use if exists
  ↓
Check packages/cli/bin/<platform>/crewchief-maproom → use if exists
  ↓
Check packages/cli/bin/crewchief-maproom → use if exists
  ↓
Check ../maproom-mcp/bin/<platform>/... → use if exists
  ↓
Try global installation (command -v crewchief-maproom)
  ↓
If none found, warn and skip operation
```

### Maproom Binary Resolution Flow (Proposed)
```
Need maproom binary
  ↓
Check CREWCHIEF_MAPROOM_BIN env var → use if exists
  ↓
[NEW] Check config.maproomBinaryPath → use if exists
  ↓
Check global installation (command -v crewchief-maproom)
  ↓
Check packages/cli/bin/<platform>/crewchief-maproom → use if exists
  ↓
If none found, warn and skip operation
```

## Key Constraints

- Worktree paths must be absolute or use `~` prefix
- Branch names must be valid git references
- Maproom cleanup is best-effort (don't fail clean command if maproom unavailable)
- Config validation happens at load time (fail fast)
- Path expansion must handle Windows (`C:\Users\...`) and Unix (`/home/...`)
