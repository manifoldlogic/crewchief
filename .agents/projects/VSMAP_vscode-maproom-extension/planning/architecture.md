# Architecture: VSCode Maproom Extension

## Design Philosophy

**MVP Mindset:** Ship value quickly, iterate based on feedback
- Reuse existing Maproom infrastructure (don't reinvent)
- Spawn processes instead of reimplementing logic
- Configuration over customization
- Automation over manual control
- Clear over clever

**Non-Goals:**
- Don't build enterprise-grade monitoring
- Don't create custom Docker orchestration
- Don't implement custom search algorithms
- Don't optimize prematurely

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         VSCode IDE                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Maproom Extension                        │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  │  │
│  │  │   Setup     │  │   Indexing   │  │    Docker   │  │  │
│  │  │   Wizard    │  │   Manager    │  │   Manager   │  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────┘  │  │
│  │         │                 │                  │        │  │
│  │         └─────────────────┴──────────────────┘        │  │
│  │                           │                           │  │
│  │                    ┌──────▼───────┐                   │  │
│  │                    │  Status Bar  │                   │  │
│  │                    └──────────────┘                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                              │
│                    ┌─────────▼──────────┐                   │
│                    │  VSCode APIs       │                   │
│                    │  - Secrets         │                   │
│                    │  - Settings        │                   │
│                    │  - File Watcher    │                   │
│                    │  - Tasks           │                   │
│                    └─────────┬──────────┘                   │
└──────────────────────────────┼───────────────────────────────┘
                               │
                 ┌─────────────┴──────────────┐
                 │                            │
        ┌────────▼─────────┐         ┌───────▼────────┐
        │  Docker Compose  │         │ Rust Binary    │
        │  (CLI spawn)     │         │ crewchief-     │
        │                  │         │ maproom        │
        └────────┬─────────┘         └───────┬────────┘
                 │                           │
    ┌────────────┴──────────────┐           │
    │                           │           │
┌───▼────┐  ┌────────┐  ┌──────▼───┐       │
│Postgres│  │ Ollama │  │ MCP      │       │
│(always)│  │(if cfg)│  │ Server   │       │
└────────┘  └────────┘  └──────────┘       │
                                            │
                                    ┌───────▼────────┐
                                    │  PostgreSQL    │
                                    │  + pgvector    │
                                    └────────────────┘
```

### Component Breakdown

#### Extension Core (`src/extension.ts`)

**Responsibilities:**
- Extension lifecycle (activate/deactivate)
- Component initialization and coordination
- Global state management
- Error boundary

**Key Functions:**
```typescript
export async function activate(context: ExtensionContext): Promise<void> {
  // 1. Initialize configuration
  const config = new ConfigurationManager(context);

  // 2. Check if first-time setup needed
  if (!config.isConfigured()) {
    await showSetupWizard(context);
  }

  // 3. Initialize Docker manager
  const docker = new DockerManager(config);

  // 4. Start services if auto-start enabled
  if (config.get('autoStart')) {
    await docker.ensureServicesRunning();
  }

  // 5. Initialize indexing manager
  const indexing = new IndexingManager(config, docker);

  // 6. Start watchers
  await indexing.startWatching();

  // 7. Setup status bar
  const statusBar = new StatusBarManager(indexing, docker);
  statusBar.show();

  // 8. Register commands
  registerCommands(context, config, docker, indexing);
}

export async function deactivate(): Promise<void> {
  // Graceful shutdown
  await indexingManager?.stopWatching();
  if (config.get('dockerAutoManage')) {
    await dockerManager?.stopServices();
  }
}
```

**State Management:**
- Configuration stored in VSCode settings
- Secrets stored via SecretStorage API
- Runtime state in extension context
- No external state files (except Docker volumes)

#### Indexing Manager (`src/indexing/manager.ts`)

**Responsibilities:**
- Coordinate scan, watch, and upsert operations
- Spawn and manage Rust binary processes
- Debounce file changes
- Track index status

**Architecture:**
```typescript
class IndexingManager {
  private branchWatcher: BranchWatcher;
  private fileWatcher: FileWatcher;
  private scanQueue: ScanQueue;
  private rustBinary: RustBinarySpawner;

  async startWatching(): Promise<void> {
    // Start branch watcher
    this.branchWatcher = new BranchWatcher(this.workspaceRoot);
    this.branchWatcher.onBranchChange(async (branch) => {
      await this.handleBranchSwitch(branch);
    });

    // Start file watcher
    this.fileWatcher = new FileWatcher(this.workspaceRoot);
    this.fileWatcher.onChange(async (files) => {
      await this.handleFileChanges(files);
    });
  }

  async initialScan(): Promise<void> {
    // Show progress notification
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: "Indexing repository...",
      cancellable: true
    }, async (progress, token) => {
      await this.rustBinary.scan({
        onProgress: (percent, message) => {
          progress.report({ increment: percent, message });
        },
        token
      });
    });
  }

  private async handleFileChanges(files: string[]): Promise<void> {
    // Debounce: collect changes for 3 seconds
    this.scanQueue.enqueue(files);

    // After debounce, upsert all
    if (this.scanQueue.isReady()) {
      const filesToUpdate = this.scanQueue.drain();
      await this.rustBinary.upsert(filesToUpdate);
    }
  }
}
```

**Debouncing Strategy:**

Trailing Edge Debounce with Maximum Wait:

```
Algorithm: Trailing debounce with max wait

Parameters:
  - DEBOUNCE_DELAY: 3000ms (configurable)
  - MAX_WAIT: 10000ms (hard limit)

State:
  - pendingFiles: Set<string> (accumulated changes)
  - lastChangeTime: number (timestamp of last file change)
  - firstChangeTime: number (timestamp of first file in batch)
  - timer: NodeJS.Timeout | null

On file change (file_path):
  1. Add file_path to pendingFiles set
  2. currentTime = Date.now()
  3. lastChangeTime = currentTime

  4. If firstChangeTime is null:
       firstChangeTime = currentTime

  5. Calculate timeSinceFirstChange = currentTime - firstChangeTime

  6. If timeSinceFirstChange >= MAX_WAIT:
       // Force flush (max wait exceeded)
       flush()
       return

  7. If timer is set:
       clearTimeout(timer)

  8. timer = setTimeout(() => {
       flush()
     }, DEBOUNCE_DELAY)

On flush():
  1. If pendingFiles.isEmpty():
       return // Nothing to do

  2. files = Array.from(pendingFiles)
  3. pendingFiles.clear()
  4. firstChangeTime = null
  5. lastChangeTime = null
  6. timer = null

  7. Call upsert(files)
```

**State Machine:**
```
┌──────────┐
│   IDLE   │ ← No pending files
└─────┬────┘
      │ File changed
      ▼
┌──────────────┐
│ ACCUMULATING │ ← Collecting files, timer running
└─────┬────────┘
      │
      ├─→ Another file changed within 3s
      │   └─→ Reset timer, continue ACCUMULATING
      │       (unless MAX_WAIT exceeded → flush)
      │
      ├─→ 3s elapsed, no new changes
      │   └─→ FLUSHING
      │
      └─→ MAX_WAIT (10s) elapsed
          └─→ FLUSHING (forced)

┌───────────┐
│ FLUSHING  │ ← Spawning upsert process
└─────┬─────┘
      │
      ├─→ Upsert completes successfully
      │   └─→ IDLE
      │
      ├─→ Upsert fails
      │   └─→ ERROR (retry with exponential backoff)
      │
      └─→ New file changes during upsert
          └─→ Queue for next batch (new ACCUMULATING state)
```

**Edge Cases:**

1. **Rapid continuous changes:** MAX_WAIT prevents indefinite delay
   - Example: Saving 100 files via script
   - After 10s, forces flush even if changes still coming

2. **Upsert fails:** Retry with exponential backoff
   - Attempt 1: Immediate
   - Attempt 2: Wait 2s
   - Attempt 3: Wait 4s
   - Attempt 4: Wait 8s
   - Attempt 5: Give up, show error

3. **Changes during upsert:** Queue for next batch
   - Don't interrupt running upsert
   - Accumulate new changes separately
   - Flush after current upsert completes

4. **Extension deactivation:** Force immediate flush
   - Cancel debounce timer
   - Flush pending files synchronously
   - Wait for upsert to complete (max 30s)

5. **Very large batches:** Split into chunks
   - Max 100 files per upsert call
   - If pendingFiles.size > 100:
     - Flush first 100
     - Queue remaining for next batch
   - Prevents overwhelming binary

**Example Scenarios:**

Scenario 1: Single file save
```
T+0ms:   File A changes → Start 3s timer
T+3000ms: Timer fires → Flush [A]
```

Scenario 2: Rapid saves within window
```
T+0ms:    File A changes → Start 3s timer
T+1000ms: File B changes → Reset timer (2s remaining)
T+2000ms: File C changes → Reset timer (1s remaining)
T+5000ms: Timer fires → Flush [A, B, C]
```

Scenario 3: Exceeds max wait
```
T+0ms:    File A changes → Start timer
T+2000ms: File B changes → Reset timer
T+4000ms: File C changes → Reset timer
T+6000ms: File D changes → Reset timer
T+8000ms: File E changes → Reset timer
T+10000ms: MAX_WAIT exceeded → Force flush [A, B, C, D, E]
```

Scenario 4: Changes during upsert
```
T+0ms:    Files A, B, C flush → Upsert starts
T+1000ms: File D changes → Queue for next batch
T+2000ms: File E changes → Add to queue
T+5000ms: Upsert completes → Start new debounce for [D, E]
```

#### Docker Manager (`src/docker/manager.ts`)

**Responsibilities:**
- Start/stop Docker Compose services
- Health check monitoring
- Service dependency resolution
- Error recovery

**Service States:**
```
NOT_INSTALLED → needs Docker installation
NOT_RUNNING   → daemon stopped, needs start
UNHEALTHY     → services exist but failing health checks
STARTING      → services launching
HEALTHY       → all services ready
```

**Implementation:**
```typescript
class DockerManager {
  private composePath: string;
  private services: Map<string, ServiceStatus>;

  async ensureServicesRunning(): Promise<void> {
    // 1. Check Docker daemon
    if (!await this.isDockerRunning()) {
      throw new Error('Docker daemon not running');
    }

    // 2. Determine required services
    const provider = this.config.get('provider');
    const services = ['postgres'];
    if (provider === 'ollama') {
      services.push('ollama');
    }
    services.push('maproom-mcp');

    // 3. Remove unused services
    await this.removeUnusedServices(services);

    // 4. Start required services
    await this.startServices(services);

    // 5. Wait for healthy
    await this.waitForHealthy(services, { timeout: 120000 });
  }

  private async startServices(services: string[]): Promise<void> {
    const cmd = `docker compose -f ${this.composePath} up -d ${services.join(' ')}`;
    await execPromise(cmd);
  }

  private async waitForHealthy(
    services: string[],
    options: { timeout: number }
  ): Promise<void> {
    const deadline = Date.now() + options.timeout;

    while (Date.now() < deadline) {
      const statuses = await this.checkHealth(services);
      if (statuses.every(s => s === 'healthy')) {
        return;
      }
      await sleep(2000);
    }

    throw new Error('Services did not become healthy within timeout');
  }
}
```

**Health Check Logic:**
- Postgres: `pg_isready -U maproom -d maproom`
- Ollama: `ollama list` returns successfully
- MCP Server: TCP connection to stdio port (or skip if stdio-only)

#### Configuration Manager (`src/config/manager.ts`)

**Responsibilities:**
- Read/write VSCode settings
- Manage secrets (API keys)
- Validate configuration
- Migrate old configs

**Settings Schema:**
```typescript
interface MaproomConfiguration {
  // Core settings
  autoStart: boolean;              // Default: true
  provider: 'ollama' | 'openai' | 'google'; // Default: ollama
  databaseUrl: string;             // Default: auto-detect

  // Indexing settings
  watchDebounce: number;           // Default: 3000ms
  scanConcurrency: number;         // Default: 4

  // Docker settings
  dockerAutoManage: boolean;       // Default: true
  dockerComposePath: string;       // Default: ~/.maproom-mcp/docker-compose.yml

  // UI settings
  showProgress: boolean;           // Default: true
  statusBarPosition: 'left' | 'right'; // Default: 'right'
}
```

**Secrets Storage:**
```typescript
class SecretsManager {
  constructor(private secrets: SecretStorage) {}

  async getApiKey(provider: string): Promise<string | undefined> {
    return await this.secrets.get(`maproom.${provider}.apiKey`);
  }

  async setApiKey(provider: string, key: string): Promise<void> {
    await this.secrets.store(`maproom.${provider}.apiKey`, key);
  }
}
```

#### Setup Wizard (`src/ui/setupWizard.ts`)

**Responsibilities:**
- Guide first-time configuration
- Validate credentials
- Initialize Docker services
- Run initial scan

**Flow:**
```
1. Welcome screen
   ↓
2. Provider selection (Ollama / OpenAI / Google)
   ↓
3. [If OpenAI/Google] Enter API key
   ↓
4. Test connection & validate credentials
   ↓
5. Initialize Docker services
   ↓
6. Wait for services healthy
   ↓
7. Run initial repository scan
   ↓
8. Success screen with next steps
```

**Implementation:**
```typescript
class SetupWizard {
  async run(): Promise<void> {
    // Step 1: Provider selection
    const provider = await vscode.window.showQuickPick([
      {
        label: 'Ollama (Local)',
        description: 'Free, runs locally, no API key needed',
        value: 'ollama'
      },
      {
        label: 'OpenAI',
        description: 'Fast, high-quality, requires API key',
        value: 'openai'
      },
      {
        label: 'Google Vertex AI',
        description: 'Cloud-based, requires project ID',
        value: 'google'
      }
    ]);

    // Step 2: Credentials (if needed)
    let apiKey: string | undefined;
    if (provider !== 'ollama') {
      apiKey = await vscode.window.showInputBox({
        prompt: `Enter your ${provider} API key`,
        password: true,
        validateInput: (value) => {
          return value.length > 0 ? null : 'API key required';
        }
      });

      // Validate credentials
      const valid = await this.validateCredentials(provider, apiKey);
      if (!valid) {
        throw new Error('Invalid credentials');
      }
    }

    // Step 3: Save configuration
    await this.config.set('provider', provider);
    if (apiKey) {
      await this.secrets.setApiKey(provider, apiKey);
    }

    // Step 4: Docker setup
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: "Starting Maproom services..."
    }, async () => {
      await this.docker.ensureServicesRunning();
    });

    // Step 5: Initial scan
    const shouldScan = await vscode.window.showInformationMessage(
      'Maproom is configured! Scan this repository now?',
      'Yes', 'Later'
    );

    if (shouldScan === 'Yes') {
      await this.indexing.initialScan();
    }
  }
}
```

#### Status Bar Integration (`src/ui/statusBar.ts`)

**Responsibilities:**
- Show index status at a glance
- Provide click-through to details
- Update in real-time

**States:**
```typescript
enum IndexStatus {
  NOT_CONFIGURED,  // ⚙️  Setup Required
  INDEXING,        // ⟳  Indexing...
  HEALTHY,         // ✓  Indexed (2m ago)
  STALE,           // ⚠  Stale (2h ago)
  ERROR            // ✗  Error
}
```

**Display Logic:**
```typescript
class StatusBarManager {
  private item: vscode.StatusBarItem;

  update(status: IndexStatus, metadata?: any): void {
    switch (status) {
      case IndexStatus.NOT_CONFIGURED:
        this.item.text = '$(gear) Maproom: Setup';
        this.item.tooltip = 'Click to configure Maproom';
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        break;

      case IndexStatus.INDEXING:
        this.item.text = '$(sync~spin) Indexing...';
        this.item.tooltip = `${metadata.filesProcessed}/${metadata.totalFiles} files`;
        break;

      case IndexStatus.HEALTHY:
        this.item.text = '$(check) Indexed';
        this.item.tooltip = `Last updated: ${metadata.lastUpdate}`;
        this.item.backgroundColor = undefined;
        break;

      case IndexStatus.STALE:
        this.item.text = '$(warning) Stale';
        this.item.tooltip = 'Index may be outdated. Click to refresh.';
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        break;

      case IndexStatus.ERROR:
        this.item.text = '$(error) Error';
        this.item.tooltip = metadata.error;
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        break;
    }
  }

  onClick(): void {
    // Show detailed status panel
    vscode.commands.executeCommand('maproom.showStatus');
  }
}
```

#### Rust Binary Spawner (`src/utils/binary.ts`)

**Responsibilities:**
- Spawn `crewchief-maproom` processes
- Parse stdout/stderr for progress
- Handle process lifecycle
- Platform-specific binary selection

**Implementation:**
```typescript
class RustBinarySpawner {
  private binaryPath: string;

  constructor() {
    // Select correct binary for platform
    const platform = process.platform;
    const arch = process.arch;
    this.binaryPath = this.resolveBinaryPath(platform, arch);
  }

  private resolveBinaryPath(platform: string, arch: string): string {
    const binaries: Record<string, string> = {
      'darwin-arm64': 'crewchief-maproom-darwin-arm64',
      'darwin-x64': 'crewchief-maproom-darwin-amd64',
      'linux-x64': 'crewchief-maproom-linux-amd64',
      'linux-arm64': 'crewchief-maproom-linux-arm64',
      'win32-x64': 'crewchief-maproom-windows-amd64.exe'
    };

    const key = `${platform}-${arch}`;
    const binary = binaries[key];
    if (!binary) {
      throw new Error(`Unsupported platform: ${key}`);
    }

    // Binary bundled with extension
    return path.join(__dirname, '..', 'bin', binary);
  }

  async scan(options: ScanOptions): Promise<void> {
    const args = [
      'scan',
      '--path', options.path,
      '--repo', options.repo,
      '--worktree', options.worktree,
      '--commit', options.commit,
      '--concurrency', options.concurrency.toString()
    ];

    return this.spawn(args, {
      onStdout: (line) => {
        // Parse progress: "Scanning: 50/100 files"
        const match = line.match(/Scanning: (\d+)\/(\d+)/);
        if (match) {
          const [, current, total] = match;
          options.onProgress?.(
            (parseInt(current) / parseInt(total)) * 100,
            line
          );
        }
      },
      onStderr: (line) => {
        console.error('[maproom]', line);
      }
    });
  }

  async upsert(files: string[], repo: string, worktree: string): Promise<void> {
    const args = [
      'upsert',
      '--paths', files.join(','),
      '--repo', repo,
      '--worktree', worktree,
      '--commit', 'HEAD',
      '--root', this.workspaceRoot
    ];

    await this.spawn(args);
  }

  private spawn(args: string[], options?: SpawnOptions): Promise<void> {
    return new Promise((resolve, reject) => {
      const child = spawn(this.binaryPath, args, {
        env: {
          ...process.env,
          MAPROOM_DATABASE_URL: this.config.get('databaseUrl')
        }
      });

      child.stdout.on('data', (data) => {
        options?.onStdout?.(data.toString());
      });

      child.stderr.on('data', (data) => {
        options?.onStderr?.(data.toString());
      });

      child.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`Process exited with code ${code}`));
        }
      });
    });
  }
}
```

#### File Watcher (`src/indexing/fileWatcher.ts`)

**Responsibilities:**
- Monitor file changes in workspace
- Debounce rapid changes
- Filter ignored files

**Implementation:**
```typescript
class FileWatcher {
  private watcher: vscode.FileSystemWatcher;
  private debounceTimer: NodeJS.Timeout | null = null;
  private pendingFiles: Set<string> = new Set();

  constructor(
    private workspaceRoot: string,
    private onChangeCallback: (files: string[]) => Promise<void>
  ) {
    // Watch all files except git internals
    this.watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(workspaceRoot, '**/*'),
      false, // ignoreCreateEvents
      false, // ignoreChangeEvents
      false  // ignoreDeleteEvents
    );

    this.watcher.onDidChange(uri => this.handleChange(uri));
    this.watcher.onDidCreate(uri => this.handleChange(uri));
    this.watcher.onDidDelete(uri => this.handleChange(uri));
  }

  private handleChange(uri: vscode.Uri): void {
    // Skip .git directory
    if (uri.path.includes('.git/')) return;

    // Add to pending set
    this.pendingFiles.add(uri.fsPath);

    // Reset debounce timer
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    // Schedule callback after 3 seconds
    this.debounceTimer = setTimeout(() => {
      const files = Array.from(this.pendingFiles);
      this.pendingFiles.clear();
      this.onChangeCallback(files);
    }, 3000);
  }

  dispose(): void {
    this.watcher.dispose();
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
  }
}
```

#### Branch Watcher (`src/indexing/branchWatcher.ts`)

**Responsibilities:**
- Monitor `.git/HEAD` for branch switches
- Trigger incremental re-index
- Detect checkout, pull, merge
- Handle edge cases (detached HEAD, corrupted HEAD, concurrent operations)

**State Machine:**
```
┌─────────────┐
│   IDLE      │ ← Initial state, watching .git/HEAD
└──────┬──────┘
       │ HEAD file changes
       ▼
┌─────────────┐
│  PARSING    │ ← Read and validate HEAD content
└──────┬──────┘
       │
       ├─→ Valid branch ref ────────┐
       │                            ▼
       │                     ┌──────────────┐
       │                     │  COMPARING   │ ← Check if branch actually changed
       │                     └──────┬───────┘
       │                            │
       ├─→ Detached HEAD ───────────┼─→ Branch changed?
       │   (40-char SHA)            │   │
       │                            │   ├─→ Yes: TRIGGERING_SCAN
       ├─→ Corrupted HEAD ──────────┤   └─→ No:  IDLE
       │   (invalid format)         │
       │                            ▼
       └─→ Missing HEAD ─────→ ┌──────────┐
           (error state)       │  ERROR   │ ← Log error, show notification
                               └──────────┘
                                     │
                                     └─→ Retry after 5s → IDLE

┌──────────────────┐
│ TRIGGERING_SCAN  │ ← Cancel existing scan, queue new scan
└────────┬─────────┘
         │
         ├─→ Scan already running → Cancel existing → Start new
         └─→ Scan idle → Start incremental scan
              │
              ▼
         ┌─────────┐
         │  IDLE   │ ← Return to watching
         └─────────┘
```

**Edge Cases Handled:**

1. **Detached HEAD:** 40-character SHA instead of `ref: refs/heads/branch`
   - Parse as commit SHA
   - Truncate to 7 characters for display
   - Trigger full scan (not incremental)

2. **Corrupted HEAD:** Malformed content (not ref or SHA)
   - Log error with file content
   - Show warning notification
   - Retry parsing after 5 seconds
   - If persistent, disable watching and notify user

3. **Concurrent Branch Switches:** User runs `git checkout` multiple times rapidly
   - Cancel in-progress scan
   - Queue most recent branch switch
   - Only one scan runs at a time

4. **Branch Switch During Active Scan:** Scan running when HEAD changes
   - Immediately cancel current scan
   - Start new scan for new branch
   - Previous scan results discarded

5. **Missing .git/HEAD:** File doesn't exist (corrupted repo)
   - Error state, disable watching
   - Show error: "Repository corrupted, please run git fsck"
   - Provide manual scan command

6. **Rebase Operations:** HEAD changes rapidly during rebase
   - Debounce HEAD changes (500ms)
   - Only trigger scan after HEAD stable for 500ms
   - Prevents scan spam during multi-commit rebase

**Implementation:**
```typescript
enum BranchWatcherState {
  IDLE,
  PARSING,
  COMPARING,
  TRIGGERING_SCAN,
  ERROR
}

interface BranchInfo {
  type: 'branch' | 'detached' | 'corrupted';
  name: string;
  sha?: string; // For detached HEAD
}

class BranchWatcher {
  private watcher: vscode.FileSystemWatcher;
  private currentBranch: BranchInfo;
  private state: BranchWatcherState = BranchWatcherState.IDLE;
  private debounceTimer: NodeJS.Timeout | null = null;
  private errorCount: number = 0;
  private readonly MAX_ERRORS = 3;
  private readonly DEBOUNCE_MS = 500; // Wait for rebase to settle

  constructor(
    private workspaceRoot: string,
    private onBranchChange: (branch: BranchInfo) => Promise<void>
  ) {
    this.currentBranch = this.getCurrentBranch();

    // Watch .git/HEAD
    const headPath = path.join(workspaceRoot, '.git', 'HEAD');

    // Verify HEAD exists before watching
    if (!fs.existsSync(headPath)) {
      this.enterErrorState('Missing .git/HEAD - repository may be corrupted');
      return;
    }

    this.watcher = vscode.workspace.createFileSystemWatcher(
      headPath,
      true,  // ignoreCreateEvents
      false, // ignoreChangeEvents
      true   // ignoreDeleteEvents
    );

    this.watcher.onDidChange(() => this.handleHeadChange());
  }

  private getCurrentBranch(): BranchInfo {
    const headPath = path.join(this.workspaceRoot, '.git', 'HEAD');

    try {
      if (!fs.existsSync(headPath)) {
        return { type: 'corrupted', name: 'MISSING' };
      }

      const content = fs.readFileSync(headPath, 'utf-8').trim();

      // Empty file
      if (content.length === 0) {
        return { type: 'corrupted', name: 'EMPTY' };
      }

      // Branch reference: "ref: refs/heads/main"
      if (content.startsWith('ref:')) {
        const parts = content.split('/');
        const branchName = parts[parts.length - 1];

        if (!branchName || branchName.length === 0) {
          return { type: 'corrupted', name: 'INVALID_REF' };
        }

        return { type: 'branch', name: branchName };
      }

      // Detached HEAD: 40-character SHA
      if (/^[0-9a-f]{40}$/i.test(content)) {
        return {
          type: 'detached',
          name: content.substring(0, 7), // Short SHA for display
          sha: content
        };
      }

      // Unknown format
      logger.warn(`Unexpected HEAD format: ${content.substring(0, 50)}`);
      return { type: 'corrupted', name: 'INVALID_FORMAT' };

    } catch (error) {
      logger.error('Failed to read .git/HEAD', error);
      return { type: 'corrupted', name: 'READ_ERROR' };
    }
  }

  private async handleHeadChange(): Promise<void> {
    // Debounce rapid changes (e.g., during rebase)
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    this.debounceTimer = setTimeout(() => {
      this.processHeadChange();
    }, this.DEBOUNCE_MS);
  }

  private async processHeadChange(): Promise<void> {
    this.state = BranchWatcherState.PARSING;

    const newBranch = this.getCurrentBranch();

    // Handle corrupted state
    if (newBranch.type === 'corrupted') {
      this.errorCount++;

      if (this.errorCount >= this.MAX_ERRORS) {
        this.enterErrorState(`Persistent .git/HEAD corruption: ${newBranch.name}`);
        return;
      }

      logger.warn(`Corrupted HEAD detected (${this.errorCount}/${this.MAX_ERRORS}): ${newBranch.name}`);

      // Retry after delay
      setTimeout(() => this.processHeadChange(), 5000);
      return;
    }

    // Reset error count on successful parse
    this.errorCount = 0;
    this.state = BranchWatcherState.COMPARING;

    // Compare with current branch
    const changed = this.branchChanged(this.currentBranch, newBranch);

    if (changed) {
      logger.info(`Branch changed: ${this.currentBranch.name} (${this.currentBranch.type}) -> ${newBranch.name} (${newBranch.type})`);

      this.state = BranchWatcherState.TRIGGERING_SCAN;
      this.currentBranch = newBranch;

      try {
        await this.onBranchChange(newBranch);
        this.state = BranchWatcherState.IDLE;
      } catch (error) {
        logger.error('Branch change handler failed', error);
        this.state = BranchWatcherState.ERROR;

        // Show error but don't disable watching
        vscode.window.showErrorMessage(
          `Failed to re-index after branch switch: ${error.message}`,
          'Retry'
        ).then(action => {
          if (action === 'Retry') {
            this.onBranchChange(newBranch);
          }
        });
      }
    } else {
      this.state = BranchWatcherState.IDLE;
    }
  }

  private branchChanged(old: BranchInfo, new_: BranchInfo): boolean {
    // Type changed
    if (old.type !== new_.type) return true;

    // Branch name changed
    if (old.name !== new_.name) return true;

    // Detached HEAD SHA changed
    if (old.type === 'detached' && new_.type === 'detached') {
      return old.sha !== new_.sha;
    }

    return false;
  }

  private enterErrorState(message: string): void {
    this.state = BranchWatcherState.ERROR;
    logger.error(`BranchWatcher error: ${message}`);

    vscode.window.showErrorMessage(
      `Branch watching disabled: ${message}. Use "Maproom: Scan Repository" to manually update index.`,
      'Show Logs'
    ).then(action => {
      if (action === 'Show Logs') {
        outputChannel.show();
      }
    });
  }

  dispose(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    this.watcher.dispose();
  }
}
```

**Decision Tree: Which Scan Type?**

```
HEAD changed
    │
    ├─→ Detached HEAD (commit SHA)?
    │   └─→ Use: FULL scan (not incremental)
    │       Reason: Detached state means uncommitted work possible
    │
    ├─→ Branch name changed?
    │   ├─→ Never scanned this branch before?
    │   │   └─→ Use: FULL scan with --worktree=<branch>
    │   │       Reason: First time seeing this branch
    │   │
    │   └─→ Branch exists in index?
    │       └─→ Use: INCREMENTAL scan with --worktree=<branch>
    │           Reason: Content-addressed dedup will skip unchanged files
    │
    └─→ Same branch, just committed?
        └─→ Use: INCREMENTAL scan
            Reason: Only new/modified files since last commit
```

## Platform Support

###  Complete Platform Matrix

| Platform | Architecture | Binary Name | VSCode | Cursor | Devcontainer | Status |
|----------|-------------|-------------|---------|--------|--------------|--------|
| macOS | Apple Silicon (ARM64) | crewchief-maproom-darwin-arm64 | ✅ | ✅ | ✅ | Fully Supported |
| macOS | Intel (x64) | crewchief-maproom-darwin-amd64 | ✅ | ✅ | ✅ | Fully Supported |
| Linux | x64 (amd64) | crewchief-maproom-linux-amd64 | ✅ | ✅ | ✅ | Fully Supported |
| Linux | ARM64 | crewchief-maproom-linux-arm64 | ✅ | ✅ | ✅ | Fully Supported |
| Windows | x64 (amd64) | crewchief-maproom-windows-amd64.exe | ✅ | ✅ | ⚠️  | Supported (Docker Desktop required) |
| Windows | ARM64 | - | ❌ | ❌ | ❌ | Not Supported (no Windows ARM Docker) |
| FreeBSD | Any | - | ❌ | ❌ | ❌ | Not Supported |
| Other | Any | - | ❌ | ❌ | ❌ | Not Supported |

**Platform Detection Logic:**
```typescript
function getPlatformBinaryName(): string {
  const platform = process.platform; // 'darwin', 'linux', 'win32'
  const arch = process.arch; // 'arm64', 'x64'

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
      `Platform ${platform}-${arch} is not supported. ` +
      `Supported platforms: ${Object.keys(binaryMap).join(', ')}`
    );
  }

  return binary;
}
```

**Error Messages by Platform:**

1. **Unsupported Platform (e.g., FreeBSD, Windows ARM):**
   ```
   Maproom extension is not available for freebsd-x64.

   Supported platforms:
   - macOS (Intel and Apple Silicon)
   - Linux (x64 and ARM64)
   - Windows (x64 only)

   For other platforms, you can build the Rust binary manually:
   https://github.com/your-org/crewchief#building-from-source
   ```

2. **Binary Missing (corrupted installation):**
   ```
   Maproom binary not found at: /path/to/bin/crewchief-maproom-darwin-arm64

   This may indicate a corrupted installation. Try:
   1. Uninstall the extension
   2. Reinstall from VSIX
   3. Verify file exists

   If issue persists, please file a bug report.
   ```

3. **Binary Not Executable (permissions):**
   ```
   Maproom binary is not executable.

   Run this command to fix:
   chmod +x /path/to/bin/crewchief-maproom-darwin-arm64

   Then reload VSCode.
   ```

**Fallback Strategy:** None - extension requires platform-specific binary. If unsupported, show clear error and disable extension.

### Devcontainer Integration

**Challenge:** Devcontainers run inside Docker, but extension needs to connect to Docker services.

**Solution:** Three devcontainer modes supported:

1. **Docker-in-Docker (DinD):**
   - Docker daemon runs inside devcontainer
   - Extension connects to localhost as normal
   - `.devcontainer/devcontainer.json`:
     ```json
     {
       "features": {
         "ghcr.io/devcontainers/features/docker-in-docker:2": {}
       }
     }
     ```

2. **Docker-outside-of-Docker (DooD):**
   - Host Docker socket mounted into container
   - Extension connects to host Docker daemon
   - `.devcontainer/devcontainer.json`:
     ```json
     {
       "mounts": [
         "source=/var/run/docker.sock,target=/var/run/docker.sock,type=bind"
       ]
     }
     ```

3. **Remote Docker (Linux only):**
   - Docker daemon on different machine
   - Extension connects via DOCKER_HOST environment variable
   - `.devcontainer/devcontainer.json`:
     ```json
     {
       "remoteEnv": {
         "DOCKER_HOST": "tcp://host.docker.internal:2375"
       }
     }
     ```

**Detection Logic:**
```typescript
function isDevcontainer(): boolean {
  // Check for devcontainer environment marker
  return (
    process.env.REMOTE_CONTAINERS === 'true' ||
    process.env.CODESPACES === 'true' ||
    fs.existsSync('/workspace/.devcontainer')
  );
}

function getDockerHost(): string {
  // Priority: env var > auto-detect > default

  // 1. Explicit env var
  if (process.env.DOCKER_HOST) {
    return process.env.DOCKER_HOST;
  }

  // 2. Auto-detect devcontainer mode
  if (isDevcontainer()) {
    if (process.platform === 'linux') {
      // DooD: socket mounted
      if (fs.existsSync('/var/run/docker.sock')) {
        return 'unix:///var/run/docker.sock';
      }

      // Remote: try host.docker.internal
      return 'tcp://host.docker.internal:2375';
    } else {
      // macOS/Windows: host.docker.internal works
      return 'tcp://host.docker.internal:2375';
    }
  }

  // 3. Default (local Docker)
  return process.platform === 'win32'
    ? 'npipe:////./pipe/docker_engine'
    : 'unix:///var/run/docker.sock';
}
```

**Devcontainer-Specific Configuration:**

```json
// .devcontainer/settings.json (workspace-level override)
{
  "maproom.databaseUrl": "postgresql://maproom:maproom@host.docker.internal:5433/maproom",
  "maproom.dockerComposePath": "${localWorkspaceFolder}/.devcontainer/docker-compose.yml"
}
```

## Error Handling & Recovery

### Comprehensive Error Taxonomy

All errors categorized by:
- **Retriable:** Can retry with backoff
- **Fatal:** Requires manual intervention
- **User Action:** What user can do to fix

| Error Category | Example | Retriable | Retry Budget | User Action |
|---------------|---------|-----------|-------------|-------------|
| **Docker Errors** |
| Docker Not Installed | `docker: command not found` | ❌ Fatal | - | Install Docker Desktop |
| Docker Daemon Stopped | `Cannot connect to Docker daemon` | ✅ Retriable | 3 attempts, 5s delay | Start Docker Desktop, then click "Retry" |
| Service Unhealthy | Health check timeout | ✅ Retriable | 3 attempts, 10s delay | Check logs via "View Logs", then "Retry" |
| Port Conflict | `port 5433 already in use` | ❌ Fatal | - | Stop conflicting service, or change port in settings |
| **Binary Errors** |
| Binary Not Found | Platform unsupported | ❌ Fatal | - | Use supported platform or build from source |
| Binary Permission Error | `EACCES: permission denied` | ❌ Fatal | - | Run `chmod +x <binary>`, reload extension |
| Binary Spawn Failure | `spawn ENOENT` | ✅ Retriable | 2 attempts, 1s delay | Check logs, file bug report if persists |
| Binary Crash | Exit code 139 (segfault) | ✅ Retriable | 1 attempt only | File bug report with logs |
| Binary Timeout | Scan exceeds 10min | ❌ Fatal | - | Reduce concurrency, exclude large files |
| **Database Errors** |
| Connection Refused | `ECONNREFUSED localhost:5433` | ✅ Retriable | 5 attempts, 2s delay | Ensure postgres service healthy |
| Authentication Failed | `password authentication failed` | ❌ Fatal | - | Reset database: `docker compose down -v`, restart |
| Schema Mismatch | `relation "chunks" does not exist` | ❌ Fatal | - | Delete volumes: `docker compose down -v`, restart |
| Disk Full | `no space left on device` | ❌ Fatal | - | Free disk space, restart services |
| **Configuration Errors** |
| Invalid API Key | `401 Unauthorized` | ❌ Fatal | - | Re-enter API key in settings |
| Provider Auth Failed | OpenAI quota exceeded | ❌ Fatal | - | Check API key quota, upgrade plan |
| Missing Credentials | SecretStorage empty | ❌ Fatal | - | Run setup wizard again |
| Invalid Concurrency | `concurrency must be 1-16` | ❌ Fatal | - | Fix in settings (auto-clamp to valid range) |
| **File System Errors** |
| Workspace Not Git Repo | `.git` directory missing | ❌ Fatal | - | Open a git repository, or run `git init` |
| Permission Denied | `EACCES: permission denied` reading file | ⚠️  Skip | - | Skip file, log warning, continue scan |
| File Too Large | File >10MB | ⚠️  Skip | - | Skip file, log info (can configure limit) |
| Network Filesystem Lag | Watcher misses changes on NFS | ⚠️  Known Limitation | - | Use manual rescan, or work locally |

**Retry Budget by Operation:**

```typescript
const RETRY_BUDGETS = {
  dockerStart: {
    maxAttempts: 3,
    baseDelay: 5000,
    maxDelay: 30000,
    backoff: 'exponential' // 5s, 10s, 20s
  },
  binarySpawn: {
    maxAttempts: 2,
    baseDelay: 1000,
    maxDelay: 5000,
    backoff: 'linear' // 1s, 2s
  },
  databaseConnect: {
    maxAttempts: 5,
    baseDelay: 2000,
    maxDelay: 10000,
    backoff: 'exponential' // 2s, 4s, 8s, 10s (capped), 10s
  },
  upsert: {
    maxAttempts: 3,
    baseDelay: 2000,
    maxDelay: 8000,
    backoff: 'exponential' // 2s, 4s, 8s
  }
};
```

**Retry Implementation with Circuit Breaker:**

```typescript
class RetryableOperation<T> {
  private failureCount: number = 0;
  private lastFailureTime: number = 0;
  private circuitOpen: boolean = false;

  async execute(
    fn: () => Promise<T>,
    budget: RetryBudget
  ): Promise<T> {
    // Circuit breaker: if too many recent failures, fail fast
    if (this.circuitOpen) {
      const timeSinceFailure = Date.now() - this.lastFailureTime;
      if (timeSinceFailure < 60000) { // 1 minute cooldown
        throw new Error('Circuit breaker open - too many recent failures');
      }
      // Reset after cooldown
      this.circuitOpen = false;
      this.failureCount = 0;
    }

    for (let attempt = 1; attempt <= budget.maxAttempts; attempt++) {
      try {
        const result = await fn();
        // Success - reset failure count
        this.failureCount = 0;
        return result;
      } catch (error) {
        this.failureCount++;
        this.lastFailureTime = Date.now();

        // Open circuit if too many failures
        if (this.failureCount >= 5) {
          this.circuitOpen = true;
        }

        // Last attempt - rethrow
        if (attempt === budget.maxAttempts) {
          throw error;
        }

        // Calculate delay
        const delay = this.calculateDelay(attempt, budget);
        logger.warn(`Attempt ${attempt} failed, retrying in ${delay}ms`, error);

        await sleep(delay);
      }
    }

    throw new Error('Unreachable');
  }

  private calculateDelay(attempt: number, budget: RetryBudget): number {
    if (budget.backoff === 'exponential') {
      const delay = budget.baseDelay * Math.pow(2, attempt - 1);
      return Math.min(delay, budget.maxDelay);
    } else {
      const delay = budget.baseDelay * attempt;
      return Math.min(delay, budget.maxDelay);
    }
  }
}
```

**User-Facing Error Actions:**

Every error shows actionable buttons:

```typescript
function showError(error: CategorizedError): void {
  const actions = error.userActions; // e.g., ['Retry', 'View Logs', 'Open Settings']

  vscode.window.showErrorMessage(
    error.userMessage,
    ...actions
  ).then(action => {
    switch (action) {
      case 'Retry':
        retry(error.operation);
        break;
      case 'View Logs':
        outputChannel.show();
        break;
      case 'Open Settings':
        vscode.commands.executeCommand('workbench.action.openSettings', 'maproom');
        break;
      case 'Run Setup':
        showSetupWizard();
        break;
      case 'Report Bug':
        vscode.env.openExternal(vscode.Uri.parse('https://github.com/your-org/issues/new'));
        break;
    }
  });
}
```

## Technology Stack

### Core Technologies

**Language:** TypeScript 5.x
- Strong typing for maintainability
- ESM modules
- Strict mode enabled

**Runtime:** Node.js 18+ (VSCode requirement)
- Native child_process for spawning binaries
- fs/promises for file operations
- Built-in crypto for hashing

**Platform:** VSCode Extension API 1.85+
- SecretStorage for credentials
- FileSystemWatcher for file monitoring
- Progress API for notifications
- StatusBarItem for UI

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
  "esbuild": "^0.19.0",      // Fast bundler
  "@vscode/test-electron": "^2.3.0",  // Integration testing
  "vitest": "^1.0.0"         // Unit testing
}
```

**Why Minimal Dependencies:**
- Smaller bundle size
- Faster activation
- Fewer security vulnerabilities
- Less maintenance burden

**What We're NOT Using:**
- ❌ React/Vue (no UI framework needed)
- ❌ Express (no web server)
- ❌ Dockerode (CLI spawning is simpler)
- ❌ Chokidar (VSCode API sufficient)

### Binary Distribution

**Bundled Binaries:**
```
packages/vscode-maproom/bin/
├── crewchief-maproom-darwin-arm64
├── crewchief-maproom-darwin-amd64
├── crewchief-maproom-linux-amd64
├── crewchief-maproom-linux-arm64
└── crewchief-maproom-windows-amd64.exe
```

**Build Process:**
1. Build Rust binary for all targets (GitHub Actions)
2. Copy binaries to `packages/vscode-maproom/bin/`
3. Bundle extension with binaries included
4. Platform detection at runtime

**Alternatives Considered:**
- Download on demand: Adds complexity, network dependency
- Separate extension per platform: Confusing for users
- User-provided binary: Poor UX

## Data Flow

### Initial Scan Flow

```
User opens workspace
    ↓
Extension activates
    ↓
Check if configured
    ↓ (no)
Show setup wizard
    ↓
User selects provider & enters credentials
    ↓
Save to settings + secrets
    ↓
Start Docker services
    ↓
Wait for healthy (max 2 min)
    ↓
Prompt for initial scan
    ↓ (yes)
Spawn: crewchief-maproom scan \
  --path /workspace \
  --repo crewchief \
  --worktree main \
  --commit HEAD \
  --concurrency 4
    ↓
Parse stdout for progress
    ↓
Update progress notification
    ↓
Scan completes
    ↓
Update status bar: "✓ Indexed"
    ↓
Start file & branch watchers
```

### File Change Flow

```
User saves file.ts
    ↓
FileSystemWatcher fires onChange
    ↓
Add file.ts to pendingFiles set
    ↓
Reset 3-second debounce timer
    ↓
[User saves file2.ts within 3 seconds]
    ↓
Add file2.ts to pendingFiles set
    ↓
Reset timer again
    ↓
[3 seconds elapse]
    ↓
Drain pendingFiles: [file.ts, file2.ts]
    ↓
Spawn: crewchief-maproom upsert \
  --paths file.ts,file2.ts \
  --repo crewchief \
  --worktree main \
  --commit HEAD \
  --root /workspace
    ↓
Upsert completes
    ↓
Update status bar: "✓ Indexed (just now)"
```

### Branch Switch Flow

```
User runs: git checkout feature-branch
    ↓
.git/HEAD file changes
    ↓
BranchWatcher detects change
    ↓
Read new branch name from .git/HEAD
    ↓
Compare: main ≠ feature-branch
    ↓
Show notification: "Branch changed, re-indexing..."
    ↓
Extension determines scan type (see decision tree above)
    ↓
Spawn: crewchief-maproom scan \
  --path /workspace \
  --repo crewchief \
  --worktree feature-branch \
  --commit HEAD \
  --concurrency 4
    ↓
[RUST BINARY HANDLES DEDUPLICATION]
Binary computes git blob SHA for each file
Binary queries database: "Is this blob SHA already indexed?"
  └─→ If YES: Skip (content unchanged)
  └─→ If NO: Parse, embed, insert
    ↓
Scan completes
    ↓
Update status bar: "✓ Indexed"
```

**CRITICAL CLARIFICATION: Content-Addressed Deduplication**

The extension does NOT handle deduplication. The Rust binary (`crewchief-maproom`) handles all deduplication automatically:

1. **Binary's Responsibility:**
   - Compute git blob SHA for each file (`git hash-object <file>`)
   - Query database: `SELECT 1 FROM maproom.chunks WHERE blobsha = $1`
   - If blob SHA exists: Skip file (content unchanged)
   - If blob SHA new: Parse, generate embeddings, insert

2. **Extension's Responsibility:**
   - Decide which scan type to trigger (full vs incremental)
   - Pass correct `--worktree` parameter
   - Monitor progress, show UI feedback
   - **NOTHING ELSE** - no SHA tracking, no database queries

3. **Why This Works:**
   - Git blob SHA is deterministic (same content = same SHA)
   - Multiple worktrees can share chunks (same file content)
   - Database indexes blobsha column for fast lookups
   - Binary handles all logic, extension just spawns

4. **Extension Metadata Tracking:**
   - Extension tracks: `{ repo, worktree, lastScanTime }`
   - Stored in VSCode workspace state
   - Used only for UI (show "last scanned 2m ago")
   - **NOT used for deduplication logic**

### Docker Service Management Flow

**Service Dependency Graph:**
```
┌──────────────┐
│  Extension   │
│   Activates  │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│  Determine Required Services         │
│  Based on provider configuration:    │
│  - postgres (ALWAYS)                  │
│  - ollama (if provider === 'ollama') │
│  - maproom-mcp (ALWAYS)              │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│  PHASE 1: Start postgres             │
│  Command: docker compose up -d        │
│           postgres                    │
│  Wait: Until healthy (max 30s)       │
└──────┬───────────────────────────────┘
       │
       ├─→ FAILURE → Rollback: None needed (postgres independent)
       │            Show error, offer retry
       │
       ▼ SUCCESS
┌──────────────────────────────────────┐
│  PHASE 2: Start provider (if needed) │
│  Command: docker compose up -d        │
│           ollama (conditional)        │
│  Wait: Until healthy (max 60s)       │
└──────┬───────────────────────────────┘
       │
       ├─→ FAILURE → Rollback: Stop postgres
       │            Show error, offer retry
       │
       ▼ SUCCESS
┌──────────────────────────────────────┐
│  PHASE 3: Start maproom-mcp          │
│  Requirements:                        │
│  - postgres MUST be healthy          │
│  - provider service healthy (if any)  │
│  Command: docker compose up -d        │
│           maproom-mcp                 │
│  Wait: TCP connection (max 30s)      │
└──────┬───────────────────────────────┘
       │
       ├─→ FAILURE → Rollback: Stop all services
       │            Show error, offer retry
       │
       ▼ SUCCESS
┌──────────────────────────────────────┐
│  ALL SERVICES HEALTHY                │
│  Total time budget: 120s max         │
└──────────────────────────────────────┘
```

**Startup Timing Constraints:**
- postgres: 30s max (usually ~5s)
- ollama: 60s max (model download can be slow)
- maproom-mcp: 30s max (depends on postgres)
- **Total: 120s maximum**

**Health Check Specifications:**

1. **postgres:**
   ```bash
   docker exec maproom-postgres pg_isready -U maproom -d maproom
   # Exit code 0 = healthy
   # Exit code != 0 = unhealthy
   ```
   Poll interval: 2s

2. **ollama:**
   ```bash
   docker exec maproom-ollama ollama list
   # Exit code 0 = healthy (daemon running)
   # Exit code != 0 = unhealthy
   ```
   Poll interval: 3s (slower startup)

3. **maproom-mcp:**
   ```typescript
   // TCP connection test
   const client = new net.Socket();
   client.setTimeout(5000);
   client.connect(MAPROOM_MCP_PORT, 'localhost', () => {
     client.destroy();
     return true; // healthy
   });
   client.on('error', () => false); // unhealthy
   ```
   Poll interval: 2s

**Rollback Strategy:**

Partial startup failures trigger complete cleanup:

```typescript
async function rollbackServices(failedPhase: number): Promise<void> {
  logger.warn(`Rollback: Phase ${failedPhase} failed, stopping all services`);

  try {
    // Stop all services (don't leave orphans)
    await exec('docker compose -f <path> down');

    // Clean up volumes if corrupted
    if (failedPhase === 1) {
      // Postgres failed - volumes might be corrupted
      const cleanVolumes = await vscode.window.showWarningMessage(
        'Database initialization failed. Clean volumes and retry?',
        'Yes', 'No'
      );

      if (cleanVolumes === 'Yes') {
        await exec('docker compose -f <path> down -v');
      }
    }
  } catch (error) {
    logger.error('Rollback failed', error);
    // Manual intervention required
    vscode.window.showErrorMessage(
      'Failed to clean up services. Run "docker compose down" manually.',
      'Show Command'
    );
  }
}
```

**Circular Dependency Prevention:**

The dependency graph is a DAG (Directed Acyclic Graph):
```
postgres ──┐
           ├──> maproom-mcp
ollama ────┘
```

No circular dependencies possible. maproom-mcp depends on postgres + provider, but nothing depends on maproom-mcp.

**Full Startup Sequence with Retries:**

```
Extension activates
    ↓
Check config: autoStart = true
    ↓
DockerManager.ensureServicesRunning()
    ↓
Check: docker info (is daemon running?)
    ↓ (no) → Error: "Docker daemon not running. Start Docker Desktop."
    ↓ (yes)
Determine required services:
  provider = ollama
  required = [postgres, ollama, maproom-mcp]
    ↓
Check existing containers:
  docker compose ps
    ↓
Stop and remove unused services:
  docker compose stop openai-service (if exists)
  docker compose rm -f openai-service
    ↓
PHASE 1: Start postgres
  docker compose up -d postgres
  Poll pg_isready every 2s (max 30s)
    ↓ (timeout) → Rollback all, show error
    ↓ (healthy)
PHASE 2: Start ollama
  docker compose up -d ollama
  Poll ollama list every 3s (max 60s)
    ↓ (timeout) → Rollback all, show error
    ↓ (healthy)
PHASE 3: Start maproom-mcp
  docker compose up -d maproom-mcp
  Poll TCP connection every 2s (max 30s)
    ↓ (timeout) → Rollback all, show error
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

**User Settings (global defaults):**
```json
{
  "maproom.autoStart": true,
  "maproom.provider": "ollama",
  "maproom.dockerAutoManage": true,
  "maproom.showProgress": true
}
```

**Workspace Settings (project-specific):**
```json
{
  "maproom.provider": "openai",  // Override for this project
  "maproom.scanConcurrency": 8   // Faster for this large repo
}
```

**Secrets (encrypted, per-user):**
```
maproom.openai.apiKey = "sk-..."
maproom.google.projectId = "my-project"
```

### Database Connection Model

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

**Rust Binary's Database Responsibilities:**
1. Connect to PostgreSQL using connection string from env var
2. Run migrations if needed (managed by binary)
3. Execute all SQL queries (INSERT, SELECT, UPDATE, DELETE)
4. Manage connection pool
5. Handle database errors

**Why This Separation:**
- Extension is lightweight (no database driver dependency)
- Binary handles all database logic (single source of truth)
- Extension just orchestrates, binary executes
- Database schema changes don't require extension updates

**Connection String Priority:**

```typescript
function getDatabaseUrl(): string {
  // 1. Environment variable (highest priority)
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL;
  }

  // 2. VSCode setting
  const configured = vscode.workspace.getConfiguration('maproom').get<string>('databaseUrl');
  if (configured) {
    return configured;
  }

  // 3. Auto-detect based on environment
  if (isDevcontainer()) {
    return 'postgresql://maproom:maproom@host.docker.internal:5433/maproom';
  }

  // 4. Default (localhost)
  return 'postgresql://maproom:maproom@localhost:5433/maproom';
}
```

**Database Health Check:**

Extension checks database health before spawning binary:

```typescript
async function isDatabaseHealthy(): Promise<boolean> {
  try {
    const result = await exec('docker exec maproom-postgres pg_isready -U maproom -d maproom');
    return result.exitCode === 0;
  } catch (error) {
    return false;
  }
}

// NEVER do this in extension:
// await pg.query('SELECT 1'); // ❌ NO direct database access
```

### Scan Types & CLI Syntax

**Three Scan Types:**

1. **Full Scan (Initial):**
   - When: First time indexing a repository
   - CLI: `crewchief-maproom scan --path <workspace> --repo <name> --worktree <branch> --commit HEAD`
   - Behavior: Index ALL files (respects .gitignore)

2. **Full Scan (Branch Switch to New Branch):**
   - When: Switching to a branch never scanned before
   - CLI: `crewchief-maproom scan --path <workspace> --repo <name> --worktree <new-branch> --commit HEAD`
   - Behavior: Index all files, but dedupe against existing chunks

3. **Incremental Update (Upsert):**
   - When: File changes on current branch, or switching to previously-scanned branch
   - CLI: `crewchief-maproom upsert --paths file1.ts,file2.ts --repo <name> --worktree <branch> --commit HEAD --root <workspace>`
   - Behavior: Only process specified files

**Decision Tree:**

```
Operation needed?
    │
    ├─→ Initial workspace open
    │   └─→ Check: Has this repo been scanned before?
    │       ├─→ No → FULL SCAN
    │       └─→ Yes → Skip (index already exists)
    │
    ├─→ Branch switch (git checkout)
    │   └─→ Check: Have we scanned this branch before?
    │       │   Query database: SELECT 1 FROM maproom.worktrees WHERE repo=$1 AND worktree=$2
    │       │   (Note: Binary does this query, not extension)
    │       │
    │       ├─→ No (new branch) → FULL SCAN for branch
    │       └─→ Yes (existing branch) → INCREMENTAL UPDATE
    │           └─→ git diff --name-only <old-commit> <new-commit>
    │               └─→ Upsert only changed files
    │
    └─→ File changes (watching)
        └─→ INCREMENTAL UPDATE
            └─→ Upsert modified files
```

**Exact CLI Syntax:**

```bash
# Full Scan
crewchief-maproom scan \
  --path /workspace/my-project \
  --repo my-project \
  --worktree main \
  --commit abc123def456 \
  --concurrency 4

# Incremental Upsert
crewchief-maproom upsert \
  --paths src/index.ts,src/utils.ts,README.md \
  --repo my-project \
  --worktree main \
  --commit abc123def456 \
  --root /workspace/my-project

# Note: NO --incremental flag exists
# The binary auto-detects via blobsha deduplication
```

**Parameters Explained:**

| Parameter | Required | Description | Example |
|-----------|----------|-------------|---------|
| `--path` | Yes (scan) | Absolute path to scan | `/workspace/my-project` |
| `--paths` | Yes (upsert) | Comma-separated file paths (relative to root) | `src/a.ts,src/b.ts` |
| `--repo` | Yes | Repository name (from git remote or dirname) | `my-project` or `github.com/org/repo` |
| `--worktree` | Yes | Branch/worktree name | `main`, `feature-branch` |
| `--commit` | Yes | Git commit SHA (for versioning) | `abc123def456` or `HEAD` |
| `--root` | Yes (upsert) | Repository root (for resolving relative paths) | `/workspace/my-project` |
| `--concurrency` | No | Parallel workers (default: 4, max: 16) | `8` |

**Repository Name Extraction:**

```typescript
function getRepoName(workspacePath: string): string {
  // 1. Try git remote
  try {
    const remote = execSync('git remote get-url origin', { cwd: workspacePath }).toString().trim();

    // Parse: git@github.com:org/repo.git → org/repo
    // Parse: https://github.com/org/repo.git → org/repo
    const match = remote.match(/github\.com[:/](.+?)(?:\.git)?$/);
    if (match) {
      return match[1]; // e.g., "crewchief-ai/crewchief"
    }
  } catch {
    // No git remote
  }

  // 2. Fallback to directory name
  return path.basename(workspacePath); // e.g., "my-project"
}
```

### Rust Binary Protocol

**Output Format (stdout):**

Binary outputs structured JSON lines (ndjson) for progress:

```jsonl
{"type":"start","total_files":150,"operation":"scan"}
{"type":"progress","files_processed":50,"current_file":"src/index.ts","percent":33}
{"type":"progress","files_processed":100,"current_file":"src/utils/helper.ts","percent":66}
{"type":"progress","files_processed":150,"current_file":"README.md","percent":100}
{"type":"complete","files_processed":150,"chunks_inserted":543,"chunks_skipped":102,"duration_ms":45231}
```

**Extension Parsing:**

```typescript
class BinaryOutputParser {
  parse(line: string): ProgressEvent | null {
    try {
      const event = JSON.parse(line);

      switch (event.type) {
        case 'start':
          return {
            type: 'start',
            totalFiles: event.total_files,
            operation: event.operation
          };

        case 'progress':
          return {
            type: 'progress',
            filesProcessed: event.files_processed,
            currentFile: event.current_file,
            percent: event.percent
          };

        case 'complete':
          return {
            type: 'complete',
            filesProcessed: event.files_processed,
            chunksInserted: event.chunks_inserted,
            chunksSkipped: event.chunks_skipped,
            durationMs: event.duration_ms
          };

        case 'error':
          return {
            type: 'error',
            message: event.message,
            file: event.file,
            recoverable: event.recoverable
          };

        default:
          logger.warn(`Unknown event type: ${event.type}`);
          return null;
      }
    } catch (error) {
      logger.warn(`Failed to parse binary output: ${line}`);
      return null;
    }
  }
}
```

**Error Output (stderr):**

Binary logs errors and warnings to stderr:

```
WARN: Skipping large file: src/data/huge-file.json (15MB > 10MB limit)
ERROR: Failed to parse src/malformed.ts: Unexpected token at line 42
INFO: Using cached embeddings for 50 unchanged files
```

**Exit Codes:**

| Code | Meaning | Extension Action |
|------|---------|------------------|
| 0 | Success | Update status bar, show success notification |
| 1 | Partial failure (some files skipped) | Show warning with count of skipped files |
| 2 | Database connection failed | Retry with backoff, check postgres health |
| 3 | Invalid arguments | Show error, fix arguments, don't retry |
| 10 | Embedding provider failed | Show error, check API key, offer retry |
| 139 | Segmentation fault (crash) | Log crash, file bug report, retry once |
| 124 | Timeout (killed by extension) | Show timeout error, suggest reducing concurrency |

### Environment Variables Reference

**Complete Environment Variable Table:**

| Variable | Required | Default | Description | Example |
|----------|----------|---------|-------------|---------|
| **Database** |
| `MAPROOM_DATABASE_URL` | Yes | `postgresql://maproom:maproom@localhost:5433/maproom` | PostgreSQL connection string | `postgresql://user:pass@host:5433/db` |
| **Embedding Provider** |
| `MAPROOM_EMBEDDING_PROVIDER` | Yes | `ollama` | Which provider to use | `ollama`, `openai`, `google` |
| `OLLAMA_HOST` | No (Ollama only) | `http://localhost:11434` | Ollama API endpoint | `http://localhost:11434` |
| `OPENAI_API_KEY` | Yes (OpenAI only) | - | OpenAI API key | `sk-...` |
| `GOOGLE_PROJECT_ID` | Yes (Google only) | - | Google Cloud project ID | `my-project-123` |
| `GOOGLE_APPLICATION_CREDENTIALS` | Yes (Google only) | - | Path to GCP service account JSON | `/path/to/creds.json` |
| **Docker** |
| `DOCKER_HOST` | No | Platform-specific | Docker daemon socket/URL | `unix:///var/run/docker.sock` |
| **Indexing Behavior** |
| `MAPROOM_MAX_FILE_SIZE` | No | `10485760` (10MB) | Skip files larger than this (bytes) | `5242880` (5MB) |
| `MAPROOM_LOG_LEVEL` | No | `info` | Binary log verbosity | `debug`, `info`, `warn`, `error` |

**Precedence Order:**

For each configuration value:

1. **Environment variable** (highest priority)
2. **VSCode setting** (`maproom.*`)
3. **Extension default** (lowest priority)

**Example:**

```typescript
function getEmbeddingProvider(): string {
  // 1. Check env var
  if (process.env.MAPROOM_EMBEDDING_PROVIDER) {
    return process.env.MAPROOM_EMBEDDING_PROVIDER;
  }

  // 2. Check VSCode setting
  const setting = vscode.workspace.getConfiguration('maproom').get<string>('provider');
  if (setting) {
    return setting;
  }

  // 3. Default
  return 'ollama';
}
```

**Environment Setup for Binary:**

```typescript
function buildBinaryEnv(): Record<string, string> {
  return {
    // Inherit system environment
    ...process.env,

    // Override with extension-specific values
    MAPROOM_DATABASE_URL: getDatabaseUrl(),
    MAPROOM_EMBEDDING_PROVIDER: getEmbeddingProvider(),
    MAPROOM_LOG_LEVEL: getLogLevel(),

    // Provider-specific
    ...(getEmbeddingProvider() === 'ollama' && {
      OLLAMA_HOST: getOllamaHost()
    }),
    ...(getEmbeddingProvider() === 'openai' && {
      OPENAI_API_KEY: await getApiKey('openai')
    }),
    ...(getEmbeddingProvider() === 'google' && {
      GOOGLE_PROJECT_ID: getGoogleProjectId(),
      GOOGLE_APPLICATION_CREDENTIALS: getGoogleCredentialsPath()
    })
  };
}
```

## Deployment Strategy

### Development Installation

**Method 1: VSIX Package**
```bash
cd packages/vscode-maproom
pnpm install
pnpm run package  # Creates maproom-0.1.0.vsix

# Install in VSCode
code --install-extension maproom-0.1.0.vsix

# Install in Cursor
cursor --install-extension maproom-0.1.0.vsix
```

**Method 2: Symlink (live development)**
```bash
cd packages/vscode-maproom
pnpm install
pnpm run compile

# Symlink into VSCode extensions
ln -s $(pwd) ~/.vscode/extensions/maproom-dev

# Reload VSCode
code --reload
```

**Method 3: Debug Mode**
```bash
cd packages/vscode-maproom
pnpm install
pnpm run watch  # Continuous compilation

# Open in VSCode
code .

# Press F5 to launch Extension Development Host
```

### Marketplace Publishing (Future)

```bash
# Install vsce
pnpm add -D @vscode/vsce

# Package
pnpm run package

# Publish
vsce publish --pat <azure-devops-pat>
```

## Performance Considerations

### Activation Time Budget

**Target:** <500ms from workspace open to extension ready

**Breakdown:**
- Import modules: ~50ms
- Read configuration: ~20ms
- Initialize managers: ~30ms
- Docker health check: ~100ms (async, non-blocking)
- Start watchers: ~50ms
- Register commands: ~10ms
- **Total:** ~260ms + async Docker

**Optimization:**
- Lazy-load heavy modules
- Defer Docker checks to background
- Use esbuild for bundling (tree-shaking)
- Minimize synchronous file I/O

### Memory Budget

**Target:** <50MB idle, <200MB during indexing

**Breakdown:**
- Extension base: ~15MB
- VSCode APIs: ~10MB
- File watchers: ~5MB
- Rust binary (subprocess): ~50MB during scan
- Docker API calls: ~5MB

**Optimization:**
- Don't cache large data structures
- Stream binary output (don't buffer all)
- Dispose watchers when not needed

### Indexing Throughput

**Target:** >100 files/min

**Factors:**
- CPU cores (concurrency)
- Embedding provider latency
  - Ollama (local): Fast, ~50ms/file
  - OpenAI (API): Medium, ~100ms/file
  - Google (API): Medium, ~100ms/file
- File size (larger files take longer)
- Database connection speed

**Not Extension's Responsibility:**
- Rust binary handles concurrency
- Extension just spawns and monitors

## Error Handling Strategy

### Error Categories

1. **Docker Errors**
   - Docker not installed
   - Daemon not running
   - Services unhealthy
   - Port conflicts

2. **Indexing Errors**
   - Binary spawn failure
   - Scan timeout
   - Database connection failure
   - Out of disk space

3. **Configuration Errors**
   - Invalid API key
   - Provider authentication failure
   - Missing credentials
   - Corrupt settings

4. **File System Errors**
   - Workspace not a git repo
   - Permission denied
   - Network file system lag
   - .git directory missing

### Error Handling Patterns

**User-Facing Errors:**
```typescript
try {
  await docker.ensureServicesRunning();
} catch (error) {
  if (error.message.includes('Docker daemon not running')) {
    const action = await vscode.window.showErrorMessage(
      'Docker daemon is not running. Start Docker Desktop and try again.',
      'Retry',
      'Disable Auto-Start'
    );

    if (action === 'Retry') {
      await docker.ensureServicesRunning();
    } else if (action === 'Disable Auto-Start') {
      await config.set('autoStart', false);
    }
  } else {
    // Unknown error - log and show generic message
    console.error('Docker error:', error);
    vscode.window.showErrorMessage(
      `Failed to start services: ${error.message}`
    );
  }
}
```

**Silent Recovery:**
```typescript
// Retry logic for transient failures
async function withRetry<T>(
  fn: () => Promise<T>,
  options = { attempts: 3, delay: 1000 }
): Promise<T> {
  for (let i = 0; i < options.attempts; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === options.attempts - 1) throw error;
      await sleep(options.delay * Math.pow(2, i)); // Exponential backoff
    }
  }
  throw new Error('Unreachable');
}
```

**Logging:**
```typescript
// Output channel for debugging
const outputChannel = vscode.window.createOutputChannel('Maproom');

outputChannel.appendLine(`[${new Date().toISOString()}] Scanning repository...`);
outputChannel.appendLine(`[ERROR] Failed to spawn binary: ${error.message}`);

// Show output channel on errors
vscode.window.showErrorMessage('Indexing failed. Check logs.', 'Show Logs')
  .then(action => {
    if (action === 'Show Logs') {
      outputChannel.show();
    }
  });
```

## Security Considerations

### Credential Storage

**VSCode Secrets API:**
- OS-level encryption (Keychain/Credential Manager/libsecret)
- Per-user, not per-workspace (credentials don't leak into repos)
- Never logged or displayed in plaintext

**Implementation:**
```typescript
// Store
await context.secrets.store('maproom.openai.apiKey', apiKey);

// Retrieve
const apiKey = await context.secrets.get('maproom.openai.apiKey');

// Delete
await context.secrets.delete('maproom.openai.apiKey');
```

### Process Isolation

**Binary Spawning:**
- Environment variables for config (not command-line args)
- No shell injection (use execFile, not exec)
- Validate all inputs before passing to binary

**Bad:**
```typescript
exec(`crewchief-maproom scan --path ${userPath}`); // Shell injection!
```

**Good:**
```typescript
execFile(binaryPath, ['scan', '--path', userPath]); // Safe
```

### Network Security

**Database Connections:**
- Only connect to localhost or Docker internal network
- No external database connections (for now)
- Validate connection strings

**API Keys:**
- Transmitted via HTTPS only (OpenAI/Google)
- Never logged
- Never sent to analytics

### Input Validation

**File Paths:**
```typescript
function validatePath(inputPath: string): string {
  const resolved = path.resolve(inputPath);
  const workspace = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;

  if (!workspace) {
    throw new Error('No workspace open');
  }

  // Ensure path is within workspace
  if (!resolved.startsWith(workspace)) {
    throw new Error('Path outside workspace');
  }

  return resolved;
}
```

**Configuration Values:**
```typescript
function validateConcurrency(value: number): number {
  if (!Number.isInteger(value) || value < 1 || value > 16) {
    throw new Error('Concurrency must be 1-16');
  }
  return value;
}
```

## Monitoring & Observability

### Metrics to Track

**Extension Health:**
- Activation time
- Memory usage
- CPU usage
- Error rates

**Indexing Performance:**
- Scan duration
- Files processed per minute
- Upsert latency
- Queue depth

**Docker Health:**
- Service uptime
- Restart count
- Health check failures

### Logging Strategy

**Log Levels:**
- **ERROR:** Failures requiring user attention
- **WARN:** Degraded performance, retries
- **INFO:** Major operations (scan, upsert)
- **DEBUG:** Detailed flow (disabled by default)

**Output Channels:**
```typescript
const channel = vscode.window.createOutputChannel('Maproom');

// Structured logging
function log(level: string, message: string, metadata?: any) {
  const timestamp = new Date().toISOString();
  const meta = metadata ? ` ${JSON.stringify(metadata)}` : '';
  channel.appendLine(`[${timestamp}] ${level}: ${message}${meta}`);
}

log('INFO', 'Starting repository scan', { repo: 'crewchief', worktree: 'main' });
log('ERROR', 'Binary spawn failed', { error: error.message, code: error.code });
```

## Post-MVP Features

All future features have been moved to `post-mvp-roadmap.md` to maintain strict MVP focus.

**MVP includes ONLY:**
- Automatic repository scanning
- File/branch change watching
- Docker service orchestration
- Provider configuration wizard
- Status bar integration
- VSIX distribution

**Explicitly out of scope:**
- Multi-workspace support
- Search UI in extension
- Custom embedding models
- Index statistics dashboard
- Marketplace publishing
- Advanced configuration UI

See `post-mvp-roadmap.md` for detailed future roadmap.

## Conclusion

This architecture prioritizes:
1. **Simplicity:** Reuse existing infrastructure
2. **Reliability:** Automatic recovery, health checks
3. **Performance:** Background processing, debouncing
4. **Security:** Encrypted credentials, input validation
5. **Maintainability:** TypeScript, minimal dependencies

**MVP Scope:** Everything needed for automatic indexing, nothing more.

**Next:** Quality strategy to ensure MVP ships with confidence.
