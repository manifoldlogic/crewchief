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
- 3-second window for collecting file changes
- Reset timer on each new change
- Batch upsert to reduce process spawns
- Skip if files already in queue

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

**Implementation:**
```typescript
class BranchWatcher {
  private watcher: vscode.FileSystemWatcher;
  private currentBranch: string;

  constructor(
    private workspaceRoot: string,
    private onBranchChange: (branch: string) => Promise<void>
  ) {
    this.currentBranch = this.getCurrentBranch();

    // Watch .git/HEAD
    const headPath = path.join(workspaceRoot, '.git', 'HEAD');
    this.watcher = vscode.workspace.createFileSystemWatcher(
      headPath,
      true,  // ignoreCreateEvents
      false, // ignoreChangeEvents
      true   // ignoreDeleteEvents
    );

    this.watcher.onDidChange(() => this.handleHeadChange());
  }

  private getCurrentBranch(): string {
    const headPath = path.join(this.workspaceRoot, '.git', 'HEAD');
    const content = fs.readFileSync(headPath, 'utf-8').trim();

    // Parse "ref: refs/heads/main" -> "main"
    if (content.startsWith('ref:')) {
      return content.split('/').pop() || 'unknown';
    }

    // Detached HEAD (commit hash)
    return content.substring(0, 7);
  }

  private async handleHeadChange(): Promise<void> {
    const newBranch = this.getCurrentBranch();

    if (newBranch !== this.currentBranch) {
      console.log(`Branch changed: ${this.currentBranch} -> ${newBranch}`);
      this.currentBranch = newBranch;
      await this.onBranchChange(newBranch);
    }
  }

  dispose(): void {
    this.watcher.dispose();
  }
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
Spawn: crewchief-maproom scan \
  --path /workspace \
  --repo crewchief \
  --worktree feature-branch \
  --commit HEAD \
  --incremental  # Only changed files
    ↓
Content-addressed deduplication (BLOBSHA)
    ↓
Scan completes
    ↓
Update status bar: "✓ Indexed"
```

### Docker Service Management Flow

```
Extension activates
    ↓
Check config: autoStart = true
    ↓
DockerManager.ensureServicesRunning()
    ↓
Check: docker ps (is daemon running?)
    ↓ (yes)
Determine required services:
  - postgres (always)
  - ollama (if provider=ollama)
  - maproom-mcp (always)
    ↓
Check existing containers:
  docker compose ps
    ↓
Remove unused services:
  docker compose rm openai-service
    ↓
Start required services:
  docker compose up -d postgres ollama maproom-mcp
    ↓
Poll health checks every 2s:
  - postgres: pg_isready
  - ollama: ollama list
  - maproom-mcp: (skip, stdio-only)
    ↓
All healthy within 120s?
    ↓ (yes)
Services ready!
    ↓ (no)
Show error: "Services failed to start"
Offer: "View Logs" / "Retry"
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

### Environment Variables

Extension reads environment variables for container connection:

```bash
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5433/maproom
MAPROOM_EMBEDDING_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
```

**Fallback logic:**
1. Check environment variable
2. Check VSCode setting
3. Use default value

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

## Future Extensibility

### Planned Features (Post-MVP)

1. **Multi-Workspace Support**
   - Track multiple workspaces simultaneously
   - Shared index or separate databases?
   - Workspace-specific settings

2. **Index Statistics Panel**
   - WebView showing detailed stats
   - Charts for index growth
   - Query performance metrics

3. **Custom Embedding Models**
   - Allow user-provided Ollama models
   - Model selection UI
   - Dimension validation

4. **Search UI (Maybe)**
   - Sidebar search panel
   - Result preview
   - Jump to definition
   - **Note:** Still prefer MCP for search

### Extension Points

**Command API:**
```typescript
// Allow other extensions to trigger indexing
vscode.commands.executeCommand('maproom.scan', { path: '/custom/path' });
```

**Event API:**
```typescript
// Expose events for integration
const onIndexUpdated = new vscode.EventEmitter<IndexUpdateEvent>();
export const events = {
  onIndexUpdated: onIndexUpdated.event
};
```

## Conclusion

This architecture prioritizes:
1. **Simplicity:** Reuse existing infrastructure
2. **Reliability:** Automatic recovery, health checks
3. **Performance:** Background processing, debouncing
4. **Security:** Encrypted credentials, input validation
5. **Maintainability:** TypeScript, minimal dependencies

**MVP Scope:** Everything needed for automatic indexing, nothing more.

**Next:** Quality strategy to ensure MVP ships with confidence.
