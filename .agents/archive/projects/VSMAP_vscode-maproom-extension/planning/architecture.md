# Architecture: VSCode Maproom Extension

## Design Philosophy

**MVP Mindset:** Ship value quickly, leverage existing infrastructure
- **Delegate Heavy Lifting:** Rust binary handles all watching/indexing
- **Thin Orchestration:** Extension is just a process manager + UI
- **Reuse CLI Integration:** Worktree management already exists
- **Simple Process Management:** Spawn, monitor, parse stdout
- Clear over clever

**Non-Goals:**
- Don't reimplement file watching (Rust has it)
- Don't reimplement branch watching (Rust has it)
- Don't duplicate CLI functionality (worktree management)
- Don't implement custom search (MCP handles it)

## Critical Discovery: Existing Functionality

**The Rust Binary Already Has Everything:**

### 1. File Watching (`crewchief-maproom watch`)
```bash
crewchief-maproom watch --repo myrepo --worktree main --path /workspace --throttle 3s
```
- Watches files for changes using Rust `notify` crate
- Automatically triggers incremental upserts
- Built-in debouncing (configurable --throttle)
- Battle-tested, cross-platform

### 2. Branch Watching (`crewchief-maproom branch-watch`)
```bash
crewchief-maproom branch-watch --repo /workspace
```
- Watches `.git/HEAD` for changes
- Automatically detects branch switches
- Triggers incremental updates
- Completed in BRWATCH project (2025-11-09)

### 3. Worktree Integration (`crewchief` CLI)
```bash
crewchief worktree create feature-auth  # Auto-indexes!
crewchief worktree list
crewchief worktree use feature-auth
```
- Creates worktree AND automatically runs `maproom scan`
- Lists all worktrees
- Automatic Maproom integration

**Extension Should NOT Reimplement Any of This!**

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         VSCode IDE                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Maproom Extension (~300 lines)           │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  │  │
│  │  │   Setup     │  │   Process    │  │    Docker   │  │  │
│  │  │   Wizard    │  │ Orchestrator │  │   Manager   │  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────┘  │  │
│  │         │                 │                  │        │  │
│  │         └─────────────────┴──────────────────┘        │  │
│  │                           │                           │  │
│  │                    ┌──────▼───────┐                   │  │
│  │                    │  Status Bar  │                   │  │
│  │                    └──────────────┘                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                              │
└──────────────────────────────┼───────────────────────────────┘
                               │
                 ┌─────────────┴──────────────┐
                 │                            │
        ┌────────▼─────────┐         ┌───────▼────────┐
        │  Docker Compose  │         │ Rust Processes │
        │  (CLI spawn)     │         │ (long-running) │
        └────────┬─────────┘         └───────┬────────┘
                 │                           │
    ┌────────────┴──────────────┐    ┌──────┴────────┐
    │                           │    │               │
┌───▼────┐  ┌────────┐  ┌──────▼───┐│    crewchief- │
│Postgres│  │ Ollama │  │ MCP      ││    maproom    │
│(always)│  │(if cfg)│  │ Server   ││    watch      │
└────────┘  └────────┘  └──────────┘│    (daemon)   │
                                     │               │
                              ┌──────▼───────┐       │
                              │ crewchief-   │       │
                              │ maproom      │       │
                              │ branch-watch │       │
                              │ (daemon)     │       │
                              └──────────────┘       │
                                                     │
                                            ┌────────▼─────┐
                                            │  PostgreSQL  │
                                            │  + pgvector  │
                                            └──────────────┘
```

## Core Components

### 1. Docker Manager

Manages PostgreSQL and Maproom MCP server lifecycle.

**Responsibilities:**
- Start/stop Docker Compose services
- Health check monitoring
- Service dependency ordering
- Error handling and recovery

**Implementation:** KEEP existing design (this is unique to VSCode extension)

### 2. Process Orchestrator

Spawns and manages long-running Rust binary processes.

**Spawned Processes:**

#### File Watcher Process
```typescript
spawn('crewchief-maproom', ['watch', '--throttle', '3s'], {
  cwd: workspaceRoot,
  stdio: ['pipe', 'pipe', 'pipe']
});
```
- Uses Rust binary's built-in file watching
- Debouncing handled by Rust (--throttle flag)
- Incremental upserts handled by Rust
- Extension just monitors stdout for progress

#### Branch Watcher Process
```typescript
spawn('crewchief-maproom', ['branch-watch', '--repo', workspaceRoot], {
  cwd: workspaceRoot,
  stdio: ['pipe', 'pipe', 'pipe']
});
```
- Uses Rust binary's built-in branch detection
- .git/HEAD watching handled by Rust
- Incremental updates handled by Rust
- Extension just monitors stdout for status

**Process Lifecycle:**
- Start on extension activation
- Monitor stdout/stderr for progress/errors
- Parse NDJSON output (binary uses structured logging)
- Kill gracefully on extension deactivation
- Restart on crash (with backoff)

**Example Implementation:**
```typescript
class ProcessOrchestrator {
  private watchProcess: ChildProcess | null = null;
  private branchWatchProcess: ChildProcess | null = null;

  async startWatching(workspaceRoot: string): Promise<void> {
    // Spawn file watcher
    this.watchProcess = spawn('crewchief-maproom', [
      'watch',
      '--repo', this.getRepoName(workspaceRoot),
      '--worktree', await this.getCurrentBranch(workspaceRoot),
      '--path', workspaceRoot,
      '--throttle', '3s'
    ]);

    this.watchProcess.stdout.on('data', (data) => {
      this.handleWatchOutput(data.toString());
    });

    // Spawn branch watcher
    this.branchWatchProcess = spawn('crewchief-maproom', [
      'branch-watch',
      '--repo', workspaceRoot
    ]);

    this.branchWatchProcess.stdout.on('data', (data) => {
      this.handleBranchWatchOutput(data.toString());
    });
  }

  async stopWatching(): Promise<void> {
    this.watchProcess?.kill('SIGTERM');
    this.branchWatchProcess?.kill('SIGTERM');

    // Wait for graceful shutdown
    await Promise.all([
      this.waitForExit(this.watchProcess, 5000),
      this.waitForExit(this.branchWatchProcess, 5000)
    ]);
  }
}
```

### 3. Status Bar Manager

Displays indexing status based on process output.

**States:**
- Idle: "$(database) Maproom Ready"
- Watching: "$(eye) Watching..."
- Indexing: "$(sync~spin) Indexing 15 files..."
- Error: "$(error) Maproom Error"

**Input:** Parsed stdout from watch processes
**Output:** Status bar text + tooltip

**Implementation:**
```typescript
class StatusBarManager {
  private item: vscode.StatusBarItem;

  handleWatchOutput(output: ProcessOutput): void {
    switch (output.type) {
      case 'watching':
        this.item.text = '$(eye) Watching...';
        this.item.tooltip = 'Maproom is watching for changes';
        break;

      case 'indexing':
        this.item.text = `$(sync~spin) Indexing ${output.filesCount} files...`;
        this.item.tooltip = `Processing ${output.currentFile}`;
        break;

      case 'complete':
        this.item.text = '$(check) Indexed';
        this.item.tooltip = `Last updated: ${output.timestamp}`;
        break;

      case 'error':
        this.item.text = '$(error) Error';
        this.item.tooltip = output.message;
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        break;
    }
  }
}
```

### 4. Setup Wizard (First Run)

Guides user through initial configuration.

**Steps:**
1. Check Docker availability
2. Start Docker services
3. Select embedding provider (Ollama/OpenAI/Google)
4. Store credentials in SecretStorage
5. Trigger initial scan: `spawn('crewchief-maproom', ['scan'])`

**Configuration stored:** VSCode settings + SecretStorage

## Component Interactions

```
VSCode Activation
    ↓
Docker Manager starts services
    ↓
Process Orchestrator spawns:
  - crewchief-maproom watch
  - crewchief-maproom branch-watch
    ↓
Both processes run continuously
    ↓
Extension parses stdout → updates status bar
    ↓
User sees: "$(eye) Watching..."
```

## What We DON'T Implement

**Delegated to Rust Binary:**
- ❌ File system watching (use `notify` crate in Rust)
- ❌ Debouncing logic (use --throttle flag)
- ❌ .git/HEAD monitoring (use `branch-watch` command)
- ❌ Incremental update logic (use `watch` command)
- ❌ File change detection (Rust handles it)
- ❌ Content-addressed deduplication (Rust handles it)

**Delegated to CLI:**
- ❌ Worktree creation (use `crewchief worktree create`)
- ❌ Worktree listing (use `crewchief worktree list`)
- ❌ Worktree merging (use `crewchief worktree merge`)

**Out of Scope:**
- ❌ Search UI (use MCP via Claude Code)
- ❌ Custom configuration UI (use VSCode settings)
- ❌ Marketplace publishing (Phase 5, post-MVP)

## Stdout Parsing Protocol

### NDJSON Format from Rust Binary

**File Watcher Output:**
```jsonl
{"type":"start","operation":"watch","path":"/workspace"}
{"type":"watching","repo":"crewchief","worktree":"main"}
{"type":"change_detected","files":["src/index.ts","src/utils.ts"]}
{"type":"indexing","files_count":2,"current_file":"src/index.ts"}
{"type":"complete","files_processed":2,"chunks_inserted":15,"duration_ms":1234}
{"type":"error","message":"Database connection failed","recoverable":true}
```

**Branch Watcher Output:**
```jsonl
{"type":"start","operation":"branch_watch","repo":"/workspace"}
{"type":"watching","current_branch":"main"}
{"type":"branch_changed","from":"main","to":"feature-auth"}
{"type":"scanning","worktree":"feature-auth","files":150}
{"type":"complete","worktree":"feature-auth","chunks_inserted":543}
```

### Parsing Implementation

```typescript
class StdoutParser {
  parse(line: string): ProcessEvent | null {
    try {
      const event = JSON.parse(line);
      return this.mapToProcessEvent(event);
    } catch (error) {
      logger.warn('Failed to parse output:', line);
      return null;
    }
  }

  private mapToProcessEvent(event: any): ProcessEvent {
    // Map JSON to typed event
    switch (event.type) {
      case 'watching':
        return { type: 'idle', repo: event.repo, worktree: event.worktree };
      case 'indexing':
        return { type: 'indexing', count: event.files_count, file: event.current_file };
      case 'complete':
        return { type: 'complete', processed: event.files_processed };
      case 'error':
        return { type: 'error', message: event.message, recoverable: event.recoverable };
      default:
        return { type: 'unknown', raw: event };
    }
  }
}
```

## Process Crash Recovery

### Exponential Backoff Strategy

```typescript
class ProcessManager {
  private restartCount = 0;
  private lastCrashTime = 0;
  private readonly MAX_RESTARTS = 5;

  async handleProcessExit(code: number, signal: string): Promise<void> {
    const now = Date.now();

    // Reset counter if last crash was >5 minutes ago
    if (now - this.lastCrashTime > 300000) {
      this.restartCount = 0;
    }

    this.lastCrashTime = now;
    this.restartCount++;

    if (this.restartCount > this.MAX_RESTARTS) {
      vscode.window.showErrorMessage(
        'Maproom watch process crashed too many times. Please check logs.',
        'Show Logs', 'Restart Extension'
      );
      return;
    }

    // Exponential backoff: 1s, 2s, 4s, 8s, 16s
    const delay = Math.pow(2, this.restartCount - 1) * 1000;
    logger.warn(`Process crashed, restarting in ${delay}ms (attempt ${this.restartCount})`);

    await sleep(delay);
    await this.startWatching();
  }
}
```

## Platform Support

### Complete Platform Matrix

| Platform | Architecture | Binary Name | VSCode | Cursor | Devcontainer | Status |
|----------|-------------|-------------|---------|--------|--------------|--------|
| macOS | Apple Silicon (ARM64) | crewchief-maproom-darwin-arm64 | ✅ | ✅ | ✅ | Fully Supported |
| macOS | Intel (x64) | crewchief-maproom-darwin-amd64 | ✅ | ✅ | ✅ | Fully Supported |
| Linux | x64 (amd64) | crewchief-maproom-linux-amd64 | ✅ | ✅ | ✅ | Fully Supported |
| Linux | ARM64 | crewchief-maproom-linux-arm64 | ✅ | ✅ | ✅ | Fully Supported |
| Windows | x64 (amd64) | crewchief-maproom-windows-amd64.exe | ✅ | ✅ | ⚠️  | Supported (Docker Desktop required) |

**Platform Detection Logic:**
```typescript
function getPlatformBinaryName(): string {
  const platform = process.platform;
  const arch = process.arch;

  const binaryMap: Record<string, string> = {
    'darwin-arm64': 'crewchief-maproom-darwin-arm64',
    'darwin-x64': 'crewchief-maproom-darwin-amd64',
    'linux-arm64': 'crewchief-maproom-linux-arm64',
    'linux-x64': 'crewchief-maproom-linux-amd64',
    'win32-x64': 'crewchief-maproom-windows-amd64.exe'
  };

  const key = `${platform}-${arch}`;
  const binary = binaryMap[key];

  if (!binary) {
    throw new UnsupportedPlatformError(
      `Platform ${platform}-${arch} is not supported.`
    );
  }

  return binary;
}
```

## Docker Service Management

### Service Startup Sequence

```
Extension activates
    ↓
Check config: autoStart = true
    ↓
DockerManager.ensureServicesRunning()
    ↓
Check: docker info (is daemon running?)
    ↓ (yes)
Determine required services:
  provider = ollama
  required = [postgres, ollama, maproom-mcp]
    ↓
PHASE 1: Start postgres
  docker compose up -d postgres
  Poll pg_isready every 2s (max 30s)
    ↓ (healthy)
PHASE 2: Start ollama
  docker compose up -d ollama
  Poll ollama list every 3s (max 60s)
    ↓ (healthy)
PHASE 3: Start maproom-mcp
  docker compose up -d maproom-mcp
  Poll TCP connection every 2s (max 30s)
    ↓ (healthy)
Services ready! (total time: typically 15-30s)
```

## Configuration Strategy

### Settings Hierarchy

```
User Settings (global)
  ~/.config/Code/User/settings.json
    ↓ (override)
Workspace Settings (per-project)
  /workspace/.vscode/settings.json
    ↓ (runtime)
Extension Defaults
  package.json contribution.configuration.properties
```

### Example Configuration

**User Settings:**
```json
{
  "maproom.autoStart": true,
  "maproom.provider": "ollama",
  "maproom.dockerAutoManage": true,
  "maproom.showProgress": true
}
```

**Workspace Settings:**
```json
{
  "maproom.provider": "openai",  // Override for this project
  "maproom.scanConcurrency": 8   // Faster for this large repo
}
```

## Database Connection Model

**CRITICAL CLARIFICATION:** The extension does NOT connect to the database directly.

**Architecture:**
```
Extension ──(spawns)──> Rust Binary ──(connects)──> PostgreSQL
     │                       │
     │                       └──> Reads env vars:
     │                            MAPROOM_DATABASE_URL
     │                            MAPROOM_EMBEDDING_PROVIDER
     │
     └──> NO database connection
          NO SQL queries
          NO schema knowledge
```

**Extension's Database Responsibilities:**
1. **ONLY**: Pass database URL to binary via environment variable
2. **ONLY**: Ensure postgres service is healthy before spawning binary
3. **NOTHING ELSE**

## Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| **Database** |
| `MAPROOM_DATABASE_URL` | Yes | `postgresql://maproom:maproom@localhost:5433/maproom` | PostgreSQL connection string |
| **Embedding Provider** |
| `MAPROOM_EMBEDDING_PROVIDER` | Yes | `ollama` | Which provider to use |
| `OLLAMA_HOST` | No | `http://localhost:11434` | Ollama API endpoint |
| `OPENAI_API_KEY` | Yes (OpenAI only) | - | OpenAI API key |
| **Docker** |
| `DOCKER_HOST` | No | Platform-specific | Docker daemon socket/URL |

**Environment Setup for Binary:**
```typescript
function buildBinaryEnv(): Record<string, string> {
  return {
    ...process.env,
    MAPROOM_DATABASE_URL: getDatabaseUrl(),
    MAPROOM_EMBEDDING_PROVIDER: getEmbeddingProvider(),
    MAPROOM_LOG_LEVEL: getLogLevel(),

    // Provider-specific
    ...(getEmbeddingProvider() === 'ollama' && {
      OLLAMA_HOST: getOllamaHost()
    }),
    ...(getEmbeddingProvider() === 'openai' && {
      OPENAI_API_KEY: await getApiKey('openai')
    })
  };
}
```

## Performance Considerations

### Activation Time Budget

**Target:** <500ms from workspace open to extension ready

**Breakdown:**
- Import modules: ~50ms
- Read configuration: ~20ms
- Initialize managers: ~30ms
- Docker health check: ~100ms (async, non-blocking)
- Spawn watch processes: ~100ms
- Register commands: ~10ms
- **Total:** ~310ms + async Docker

### Memory Budget

**Target:** <50MB idle, <200MB during indexing

**Breakdown:**
- Extension base: ~15MB
- VSCode APIs: ~10MB
- Watch process monitors: ~5MB
- Rust binaries (subprocesses): ~50MB each during scan

## Deployment Strategy

### Development Installation

**Method 1: VSIX Package**
```bash
cd packages/vscode-maproom
pnpm install
pnpm run package  # Creates maproom-0.1.0.vsix

code --install-extension maproom-0.1.0.vsix
```

**Method 2: Debug Mode**
```bash
cd packages/vscode-maproom
pnpm install
pnpm run watch  # Continuous compilation

# Open in VSCode, press F5 to launch Extension Development Host
```

## Technology Stack

### Core Technologies

**Language:** TypeScript 5.x
**Runtime:** Node.js 18+
**Platform:** VSCode Extension API 1.85+

### Dependencies

**Production:**
```json
{
  "vscode": "^1.85.0"  // VSCode Extension API (peer dependency)
}
```

**Development:**
```json
{
  "@types/node": "^18.x",
  "@types/vscode": "^1.85.0",
  "typescript": "^5.3.0",
  "esbuild": "^0.19.0",
  "@vscode/test-electron": "^2.3.0",
  "vitest": "^1.0.0"
}
```

**Why Minimal Dependencies:**
- Smaller bundle size
- Faster activation
- Less maintenance burden
- Fewer security vulnerabilities

## Conclusion

This architecture prioritizes:
1. **Simplicity:** Delegate to existing Rust binary and CLI
2. **Thin Orchestration:** ~300 lines of process management
3. **Reliability:** Rust handles complex logic, extension just monitors
4. **Maintainability:** Minimal code, clear responsibilities
5. **Performance:** Fast activation, low memory

**MVP Scope:** Everything needed for automatic indexing via process orchestration, nothing more.

**Key Insight:** Extension is NOT implementing indexing - it's orchestrating existing tools.
