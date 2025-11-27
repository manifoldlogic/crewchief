# Architecture: VSCode Extension Daemon Migration

## Target Architecture

```
┌────────────────────────────────────────────────────────────┐
│ VSCode Extension                                            │
│                                                             │
│  ┌─────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  activate() │───▶│ OllamaClient │───▶│ Pull Model   │  │
│  └──────┬──────┘    └──────────────┘    └──────────────┘  │
│         │                                                   │
│         ▼                                                   │
│  ┌──────────────────┐    ┌──────────────────┐             │
│  │ Reconciliation   │───▶│ ProcessOrchest-  │             │
│  │ (git diff+upsert)│    │ rator (refactored)│            │
│  └──────────────────┘    └────────┬─────────┘             │
│                                    │ spawns                 │
└────────────────────────────────────┼────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────┐
│ crewchief-maproom watch                                       │
│                                                               │
│  ┌──────────────────┐    ┌──────────────────┐               │
│  │ File Watcher     │    │ Branch Watcher   │               │
│  │ (notify crate)   │    │ (.git/HEAD)      │               │
│  └────────┬─────────┘    └────────┬─────────┘               │
│           │                        │                         │
│           └────────────┬───────────┘                         │
│                        ▼                                     │
│           ┌──────────────────────┐                          │
│           │ Incremental Indexer  │                          │
│           └──────────┬───────────┘                          │
│                      │                                       │
│                      ▼                                       │
│           ┌──────────────────────┐    ┌──────────────────┐ │
│           │     SQLite DB        │◀───│ Host Ollama      │ │
│           │ ~/.maproom/maproom.db│    │ localhost:11434  │ │
│           └──────────────────────┘    └──────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

## Verified CLI Interfaces

### Watch Command

```bash
crewchief-maproom watch [OPTIONS]

Options:
  --repo <REPO>          # Repository name (defaults to git remote origin)
  --path <PATH>          # Path to watch (defaults to current directory)
  --throttle <THROTTLE>  # Debounce interval [default: 2s]
  --worktree <WORKTREE>  # DEPRECATED: auto-detected from branch
```

**Invocation from extension:**
```typescript
spawn(binaryPath, ['watch', '--path', workspaceRoot], {
  env: {
    MAPROOM_DATABASE_URL: `sqlite://${databasePath}`,
    MAPROOM_EMBEDDING_PROVIDER: provider,
  }
})
```

### Upsert Command (for reconciliation)

```bash
crewchief-maproom upsert --commit <COMMIT> --repo <REPO> --worktree <WORKTREE> --root <ROOT> [--paths <PATHS>]
```

**Invocation from extension:**
```typescript
spawn(binaryPath, [
  'upsert',
  '--commit', headCommit,
  '--repo', repoName,
  '--worktree', branchName,
  '--root', workspaceRoot,
  '--paths', changedFiles.join(','),
], { env: { MAPROOM_DATABASE_URL: ... } })
```

### Database URL Format

SQLite databases use these URL formats (all valid):
- `sqlite:///absolute/path/to/db` - Absolute path with prefix
- `file:/absolute/path/to/db` - File URI scheme
- `/absolute/path/to/db` - Bare absolute path

**Extension should use:** `sqlite://${absolutePath}` for consistency.

## Key Design Decisions

### 1. Single Watch Process via Refactored ProcessOrchestrator

**Decision**: Refactor existing `ProcessOrchestrator` to spawn a single unified `watch` process instead of creating a new class.

**Rationale**:
- `ProcessOrchestrator` already has platform-aware binary selection
- `StdoutParser` already handles NDJSON parsing
- `CrashRecovery` already implements exponential backoff
- Refactoring preserves tested infrastructure

**Changes to ProcessOrchestrator**:
1. Remove `branch-watch` process spawning
2. Update to spawn single `watch` command
3. Add `branch_switched` event handling
4. Remove dual-process coordination logic

### 2. TypeScript-Based Startup Reconciliation

**Decision**: Perform startup reconciliation in TypeScript before spawning watch, not in Rust.

**Implementation**:
```
1. Read last indexed commit from SQLite (via simple query)
2. Run `git diff --name-only <last-commit>..HEAD` via child_process
3. If files changed, spawn `crewchief-maproom upsert` with changed paths
4. Store current HEAD as last indexed commit
5. Start watch process
```

**Rationale**:
- No Rust changes required
- Uses existing `upsert` CLI command
- Keeps reconciliation logic in orchestration layer
- Easier to maintain and debug

**Fallback for first run**:
- If no last indexed commit exists, skip reconciliation
- Watch process will index files as they're detected
- User can run initial scan manually if needed

### 3. Ollama Model Management

**Decision**: Extension checks for and pulls the embedding model before starting watch.

**Implementation** (extends existing `detectOllama()` from setupWizard.ts):

```typescript
// src/ollama/client.ts - extends existing detection
export class OllamaClient {
  // SECURITY: Hardcoded to localhost, not configurable
  private readonly baseUrl = 'http://127.0.0.1:11434'

  // Reuses pattern from detectOllama() in setupWizard.ts
  async isRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/tags`, {
        signal: AbortSignal.timeout(2000)
      })
      return response.ok
    } catch {
      return false
    }
  }

  async hasModel(name: string): Promise<boolean> {
    const response = await fetch(`${this.baseUrl}/api/tags`)
    const data = await response.json()
    return data.models?.some(m =>
      m.name === name || m.name === `${name}:latest`
    )
  }

  async pullModel(name: string, onProgress?: (status: string) => void): Promise<void> {
    // Validate model name format (SECURITY)
    if (!/^[a-z0-9][a-z0-9._-]*(?::[a-z0-9._-]+)?$/i.test(name)) {
      throw new Error('Invalid model name format')
    }

    const response = await fetch(`${this.baseUrl}/api/pull`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    })

    // Stream NDJSON progress
    const reader = response.body!.getReader()
    const decoder = new TextDecoder()

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const text = decoder.decode(value)
      const lines = text.split('\n').filter(Boolean)
      for (const line of lines) {
        try {
          const event = JSON.parse(line)
          onProgress?.(event.status || 'Downloading...')
        } catch { /* ignore malformed lines */ }
      }
    }
  }
}
```

**Error Handling**:
- Ollama not running → Show error with "Install Ollama" button linking to https://ollama.ai
- Model pull fails → Show error with retry option
- Non-Ollama providers → Skip model check entirely

### 4. Remove Docker Dependency

**Decision**: Remove all Docker-related code from the extension.

**Components to Remove**:
- `src/docker/manager.ts` - DockerManager class
- `src/docker/index.ts` - Docker exports
- `src/docker/example-usage.ts` - Example code
- `src/docker/manager.test.ts` - Tests
- PostgreSQL-related settings in `package.json`

**Components to Update**:
- `extension.ts` - Remove `ensureDockerRunning()` calls
- `setupWizard.ts` - Remove Docker-dependent flows
- Settings - Remove PostgreSQL configuration options

### 5. Simplified Extension Flow

**New Activation Flow**:
```typescript
async function activate(context: ExtensionContext) {
  // 1. Fast sync setup (< 100ms)
  const outputChannel = createOutputChannel('Maproom')
  const statusBar = new StatusBarManager(context)
  statusBar.setState('starting')
  registerCommands(context)

  // 2. Background initialization
  void initializeAsync(context)
}

async function initializeAsync(context: ExtensionContext) {
  try {
    // 1. Check/configure provider
    const provider = getConfiguredProvider(context) || await runSetupWizard(context)
    if (!provider) return // User cancelled

    // 2. Ensure Ollama model (ONLY for ollama provider)
    if (provider === 'ollama') {
      await ensureOllamaModel('nomic-embed-text')
    }

    // 3. Run startup reconciliation
    await reconcileChanges(context)

    // 4. Start unified watch process (refactored orchestrator)
    orchestrator = new ProcessOrchestrator(outputChannel, {
      extensionRoot: context.extensionPath,
      workspaceRoot: getWorkspaceRoot(),
      databaseUrlOverride: getDatabaseUrl(),
      provider,
    })

    await orchestrator.startWatching() // Now spawns single watch
    statusBar.setState('watching')
    statusBar.connectOrchestrator(orchestrator)

  } catch (error) {
    statusBar.setState('error', error.message)
    showErrorNotification(error)
  }
}
```

## NDJSON Event Types

The unified watch command emits these events (to be added to `events.ts`):

### BranchSwitchedEvent (NEW - must add)

```typescript
/**
 * Branch switched event - emitted when git branch changes
 *
 * Example:
 * {"type":"branch_switched","timestamp":"2025-01-16T10:30:00Z","repo":"crewchief",
 *  "old_branch":"main","new_branch":"feature-auth","old_worktree_id":1,
 *  "new_worktree_id":42,"worktree_created":false}
 */
export interface BranchSwitchedEvent {
  type: 'branch_switched'
  timestamp: string           // ISO 8601
  repo: string
  old_branch: string
  new_branch: string
  old_worktree_id: number
  new_worktree_id: number
  worktree_created: boolean   // true if new worktree was created
}
```

Update `isWatchEvent()` type guard:
```typescript
case 'branch_switched':
  return (
    typeof event.timestamp === 'string' &&
    typeof event.repo === 'string' &&
    typeof event.old_branch === 'string' &&
    typeof event.new_branch === 'string' &&
    typeof event.old_worktree_id === 'number' &&
    typeof event.new_worktree_id === 'number' &&
    typeof event.worktree_created === 'boolean'
  )
```

## Reusable Components

### Existing Infrastructure to Leverage

| Component | Location | Use Case |
|-----------|----------|----------|
| `StdoutParser` | `process/parser.ts` | NDJSON parsing (keep as-is) |
| `CrashRecovery` | `process/recovery.ts` | Auto-restart with backoff (keep as-is) |
| `detectOllama()` | `ui/setupWizard.ts` | Base for OllamaClient.isRunning() |
| `SecretsManager` | `config/secrets.ts` | API key storage (keep as-is) |
| `getRepoName()` | `utils/git.ts` | Repository name detection |
| `getBranchName()` | `utils/git.ts` | Branch name detection |

### Components to Refactor

| Component | Current | After |
|-----------|---------|-------|
| `ProcessOrchestrator` | Spawns watch + branch-watch | Spawns single watch |

### Components to Add

| Component | Location | Purpose |
|-----------|----------|---------|
| `OllamaClient` | `src/ollama/client.ts` | Model detection and pull |
| `BranchSwitchedEvent` | `src/process/events.ts` | New event type |
| `reconcileChanges()` | `src/process/reconcile.ts` | Startup reconciliation |

## Settings Changes

### Remove (PostgreSQL-related)
```json
{
  "maproom.database.provider": "REMOVE",
  "maproom.database.host": "REMOVE",
  "maproom.database.port": "REMOVE",
  "maproom.database.user": "REMOVE",
  "maproom.database.password": "REMOVE",
  "maproom.database.name": "REMOVE"
}
```

### Keep/Update
```json
{
  "maproom.database.sqlitePath": "~/.maproom/maproom.db",
  "maproom.embedding.provider": "ollama",
  "maproom.embedding.model": "nomic-embed-text"
}
```

**Note**: Removed `maproom.ollama.host` - Ollama should only ever be localhost for security.

## Error States and Recovery

| State | Trigger | User Action |
|-------|---------|-------------|
| Ollama not installed | HTTP connection refused | "Install Ollama" button → https://ollama.ai |
| Ollama not running | HTTP connection refused | "Start Ollama" button |
| Model missing | Model not in `/api/tags` | Auto-pull with progress notification |
| Model pull failed | Network error during pull | Retry button |
| Watch process crash | Non-zero exit | Auto-restart with backoff (CrashRecovery) |
| SQLite not found | Database file missing | Preserve existing `showNoSqliteGuidance()` |

## Migration Path

For users with existing PostgreSQL setup:

1. **Data Migration**: Not needed - SQLite is a fresh local index
2. **Settings Migration**: Detect old settings, show migration prompt
3. **Rollback**: Users can pin previous extension version if needed

## File Changes Summary

| Action | Files |
|--------|-------|
| **Delete** | `src/docker/*` |
| **Refactor** | `src/process/orchestrator.ts` (single watch) |
| **Update** | `src/extension.ts`, `src/ui/setupWizard.ts`, `src/process/events.ts` |
| **New** | `src/ollama/client.ts`, `src/process/reconcile.ts` |
| **Remove** | `src/services/postgres-checker.ts` |

## Multi-Workspace Behavior

Each VSCode workspace spawns its own watch process:
- Workspace A → watch process for workspace A root
- Workspace B → watch process for workspace B root

All processes share the same SQLite database but track different worktrees.
