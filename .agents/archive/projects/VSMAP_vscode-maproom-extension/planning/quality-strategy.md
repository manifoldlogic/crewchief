# Quality Strategy: VSCode Maproom Extension

## Philosophy

**Pragmatic Testing for MVPs:**
- Tests prevent rework, not check boxes
- Focus on integration points and critical paths
- Unit tests for complex logic, not trivial getters
- E2E tests for user workflows
- Mock sparingly, prefer real dependencies when fast

**Confidence Over Coverage:**
- 50% coverage target for MVP (reduced due to simpler architecture)
- Critical paths: 100% coverage (activation, process spawning, Docker startup)
- Trivial code: Skip tests (getters/setters, simple formatting)
- Complex logic: Comprehensive tests (stdout parsing, error handling)
- Integration boundaries: Extensive tests (Docker, binary spawning, process lifecycle)

**Coverage Targets by Component:**
- ProcessOrchestrator (spawn, monitor, restart): 70%
- StdoutParser (NDJSON parsing): 80%
- Managers (Docker, Config): 70-80%
- UI components (status bar, wizard): 50-60%
- Utilities: 60-70%
- **Overall MVP target: 50% (reduced from 60% due to simpler architecture)**

**Ship Without Meaningful Risk:**
- Block regressions in core workflows
- Catch platform-specific issues
- Validate Docker integration
- Test error handling

## Testing Pyramid

```
           ┌─────────────┐
           │  E2E Tests  │  <- 10% (User workflows)
           │    ~5-10    │
           └─────────────┘
          ┌───────────────┐
          │ Integration   │  <- 30% (Boundaries)
          │   Tests ~20   │
          └───────────────┘
        ┌─────────────────────┐
        │   Unit Tests        │  <- 60% (Logic)
        │      ~50-80         │
        └─────────────────────┘
```

## Critical Paths

### CP1: Extension Activation

**Flow:**
```
Workspace opens → Extension activates → Docker starts → Watchers start
```

**Risk:** High (blocks all functionality)

**Test Coverage:** 100%

**Tests:**
1. Activation with valid configuration
2. Activation without configuration (triggers wizard)
3. Activation with Docker unavailable
4. Activation with invalid provider
5. Activation time <500ms

### CP2: Initial Repository Scan

**Flow:**
```
Setup wizard → Docker services start → Binary spawns → Scan completes → Index ready
```

**Risk:** High (first impression)

**Test Coverage:** 100%

**Tests:**
1. Successful scan of small repo (<10 files)
2. Successful scan of medium repo (100-1000 files)
3. Progress reporting during scan
4. Cancellation mid-scan
5. Binary spawn failure
6. Database connection failure
7. Timeout after 10 minutes

### CP3: Process Spawning and Monitoring

**Flow:**
```
Extension activate → Spawn watch → Spawn branch-watch → Monitor stdout → Update status
```

**Risk:** Medium (core functionality)

**Test Coverage:** 90%

**Tests:**
1. Process spawns successfully
2. Stdout parsed correctly (NDJSON)
3. Process crash triggers restart
4. Exponential backoff on repeated crashes
5. Graceful shutdown on deactivate
6. Multiple processes managed concurrently
7. Binary not found error handling

### CP4: Stdout Parsing and Status Updates

**Flow:**
```
Process outputs NDJSON → Parser extracts events → Status bar updated → User sees progress
```

**Risk:** Medium (user visibility)

**Test Coverage:** 90%

**Tests:**
1. Valid NDJSON parsed correctly
2. Malformed NDJSON skipped with warning
3. Missing fields use defaults
4. Large output bursts buffered correctly
5. Status bar updates within 1s
6. Multiple event types handled (progress, error, complete)
7. Binary version mismatch detected
8. Incremental line parsing (partial JSON)

### CP5: Docker Service Management

**Flow:**
```
Extension start → Docker check → Services start → Health checks → Ready
```

**Risk:** High (blocks indexing)

**Test Coverage:** 90%

**Tests:**
1. Services start successfully
2. Services already running (no-op)
3. Docker daemon not running (error message)
4. Health checks timeout (error with retry)
5. Provider-specific services (Ollama vs OpenAI)
6. Service removal when switching providers
7. Graceful shutdown on deactivation

### CP6: Provider Configuration

**Flow:**
```
Setup wizard → Provider selected → Credentials entered → Validated → Saved
```

**Risk:** Medium (setup friction)

**Test Coverage:** 90%

**Tests:**
1. Ollama setup (no credentials)
2. OpenAI setup with valid key
3. OpenAI setup with invalid key (validation fails)
4. Google setup with project ID
5. Switching providers
6. Credential storage in SecretStorage
7. Configuration persistence

## Test Categories

### Unit Tests

**Target:** Complex logic, algorithms, parsers

**Framework:** Vitest

**Examples:**

**File:** `src/processes/stdout-parser.test.ts`
```typescript
describe('StdoutParser', () => {
  it('should parse valid NDJSON events', async () => {
    const parser = new StdoutParser();
    const events: any[] = [];

    parser.onEvent((event) => events.push(event));

    parser.parseLine('{"type":"progress","files_processed":10,"total_files":100}\n');
    parser.parseLine('{"type":"complete","chunks_indexed":500}\n');

    expect(events).toHaveLength(2);
    expect(events[0].type).toBe('progress');
    expect(events[1].type).toBe('complete');
  });

  it('should skip malformed JSON with warning', async () => {
    const parser = new StdoutParser();
    const events: any[] = [];
    const warnings: string[] = [];

    parser.onEvent((event) => events.push(event));
    parser.onWarning((msg) => warnings.push(msg));

    parser.parseLine('not valid json\n');
    parser.parseLine('{"type":"progress","count":5}\n'); // Valid

    expect(events).toHaveLength(1);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toContain('Malformed JSON');
  });
});
```

**File:** `src/utils/binary.test.ts`
```typescript
describe('RustBinarySpawner', () => {
  it('should select correct binary for platform', () => {
    const spawner = new RustBinarySpawner();

    if (process.platform === 'darwin' && process.arch === 'arm64') {
      expect(spawner.binaryPath).toContain('darwin-arm64');
    } else if (process.platform === 'linux' && process.arch === 'x64') {
      expect(spawner.binaryPath).toContain('linux-amd64');
    }
  });

  it('should throw on unsupported platform', () => {
    const originalPlatform = process.platform;
    Object.defineProperty(process, 'platform', { value: 'freebsd' });

    expect(() => new RustBinarySpawner()).toThrow('Unsupported platform');

    Object.defineProperty(process, 'platform', { value: originalPlatform });
  });
});
```

**File:** `src/processes/process-orchestrator.test.ts`
```typescript
describe('ProcessOrchestrator', () => {
  it('should spawn process successfully', async () => {
    const orchestrator = new ProcessOrchestrator();
    const events: any[] = [];

    orchestrator.onEvent((event) => events.push(event));

    await orchestrator.spawn('watch', ['--path', '/test']);

    expect(orchestrator.isRunning('watch')).toBe(true);
    expect(events.some(e => e.type === 'started')).toBe(true);
  });

  it('should restart process on crash', async () => {
    const orchestrator = new ProcessOrchestrator();
    let restartCount = 0;

    orchestrator.onRestart(() => restartCount++);

    await orchestrator.spawn('watch', ['--path', '/test']);

    // Simulate crash
    orchestrator.killProcess('watch');
    await sleep(2000); // Wait for backoff

    expect(restartCount).toBeGreaterThan(0);
    expect(orchestrator.isRunning('watch')).toBe(true);
  });
});
```

**What NOT to Unit Test:**
- Simple getters/setters
- VSCode API calls (integration test instead)
- Configuration reading (use real config)
- Status bar updates (E2E test instead)
- File watching logic (Rust binary handles this)
- Branch detection logic (Rust binary handles this)
- Debouncing logic (Rust binary handles this)

### Integration Tests

**Target:** Boundaries between components, external dependencies

**Framework:** Vitest + VSCode Test Runner

**Examples:**

**File:** `src/test/integration/docker.test.ts`
```typescript
describe('DockerManager Integration', () => {
  let docker: DockerManager;

  beforeAll(async () => {
    // Assumes Docker is running on CI
    docker = new DockerManager(testConfig);
  });

  it('should start postgres service', async () => {
    await docker.startService('postgres');

    const status = await docker.getServiceStatus('postgres');
    expect(status).toBe('healthy');
  }, 60000); // 60s timeout

  it('should detect unhealthy services', async () => {
    // Intentionally break service
    await exec('docker compose -f test-compose.yml stop postgres');

    const status = await docker.getServiceStatus('postgres');
    expect(status).toBe('unhealthy');
  });

  afterAll(async () => {
    await docker.stopAllServices();
  });
});
```

**File:** `src/test/integration/indexing.test.ts`
```typescript
describe('Indexing Integration', () => {
  let indexing: IndexingManager;
  let testRepo: string;

  beforeAll(async () => {
    // Create test repository
    testRepo = await createTestRepo([
      { path: 'src/index.ts', content: 'export function main() {}' },
      { path: 'src/utils.ts', content: 'export function helper() {}' }
    ]);

    indexing = new IndexingManager(testConfig, testDocker);
  });

  it('should scan test repository', async () => {
    const result = await indexing.scan(testRepo);

    expect(result.filesScanned).toBe(2);
    expect(result.chunks).toBeGreaterThan(0);
  }, 30000);

  it('should upsert changed files', async () => {
    // Modify file
    fs.writeFileSync(
      path.join(testRepo, 'src/index.ts'),
      'export function main() { return 42; }'
    );

    await indexing.upsert([path.join(testRepo, 'src/index.ts')]);

    // Verify updated in database
    const chunk = await queryDatabase(
      'SELECT * FROM maproom.chunks WHERE relpath = $1',
      ['src/index.ts']
    );
    expect(chunk.content).toContain('return 42');
  });

  afterAll(async () => {
    await cleanupTestRepo(testRepo);
  });
});
```

**File:** `src/test/integration/secrets.test.ts`
```typescript
describe('Secrets Integration', () => {
  let secrets: SecretsManager;

  beforeAll(() => {
    // Use VSCode test environment's SecretStorage
    secrets = new SecretsManager(testContext.secrets);
  });

  it('should store and retrieve API key', async () => {
    await secrets.setApiKey('openai', 'sk-test123');

    const retrieved = await secrets.getApiKey('openai');
    expect(retrieved).toBe('sk-test123');
  });

  it('should return undefined for missing key', async () => {
    const retrieved = await secrets.getApiKey('nonexistent');
    expect(retrieved).toBeUndefined();
  });

  it('should delete API key', async () => {
    await secrets.setApiKey('google', 'gcp-test');
    await secrets.deleteApiKey('google');

    const retrieved = await secrets.getApiKey('google');
    expect(retrieved).toBeUndefined();
  });
});
```

### End-to-End Tests

**Target:** Complete user workflows

**Framework:** @vscode/test-electron + Playwright (for UI)

**Examples:**

**File:** `src/test/e2e/setup-workflow.test.ts`
```typescript
describe('Setup Workflow E2E', () => {
  it('should complete setup wizard for Ollama', async () => {
    // 1. Open workspace without config
    await openWorkspace(testWorkspace);

    // 2. Wait for setup wizard
    await waitForElement('Setup Maproom');

    // 3. Select Ollama
    await selectQuickPickItem('Ollama (Local)');

    // 4. Wait for Docker services
    await waitForNotification('Starting Maproom services', { timeout: 120000 });

    // 5. Confirm initial scan
    await clickButton('Yes');

    // 6. Wait for scan completion
    await waitForNotification('Indexing complete', { timeout: 300000 });

    // 7. Verify status bar
    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.text).toContain('Indexed');
  }, 600000); // 10 minute timeout

  it('should complete setup wizard for OpenAI', async () => {
    await openWorkspace(testWorkspace);

    await selectQuickPickItem('OpenAI');

    // Enter API key
    await typeInInputBox(process.env.TEST_OPENAI_KEY!);

    await waitForNotification('Validating credentials');
    await waitForNotification('Starting Maproom services');

    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.text).toContain('Indexed');
  }, 600000);
});
```

**File:** `src/test/e2e/process-lifecycle.test.ts`
```typescript
describe('Process Lifecycle E2E', () => {
  beforeAll(async () => {
    await openConfiguredWorkspace(testWorkspace);
  });

  it('should spawn and monitor watch process', async () => {
    // 1. Extension activates and spawns process
    await waitForStatusBar('Watching');

    // 2. Verify process running
    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.text).toContain('Watching');

    // 3. Save a file
    const doc = await vscode.workspace.openTextDocument(
      path.join(testWorkspace, 'src', 'test.ts')
    );
    const edit = new vscode.WorkspaceEdit();
    edit.insert(doc.uri, new vscode.Position(0, 0), '// New comment\n');
    await vscode.workspace.applyEdit(edit);
    await doc.save();

    // 4. Wait for Rust binary to detect and process
    await sleep(5000);

    // 5. Verify status updated from stdout
    const updatedStatus = await getStatusBarItem('Maproom');
    expect(updatedStatus.text).toContain('Indexed');
  });

  it('should recover from process crash', async () => {
    // 1. Kill watch process
    await executeCommand('maproom.killProcesses');

    // 2. Wait for restart
    await sleep(3000);

    // 3. Verify process restarted
    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.text).toContain('Watching');
  });
});
```

**File:** `src/test/e2e/branch-switching.test.ts`
```typescript
describe('Branch Switching E2E', () => {
  beforeAll(async () => {
    await openConfiguredWorkspace(testGitRepo);
    await waitForStatusBar('Watching');
  });

  it('should re-index on branch switch (via Rust binary)', async () => {
    // 1. Switch branch via git command
    await exec('git checkout feature-branch', { cwd: testGitRepo });

    // 2. Wait for Rust binary to detect (branch-watch process)
    await sleep(2000);

    // 3. Status bar should show indexing activity
    await waitForStatusBar('Indexing', { timeout: 5000 });

    // 4. Wait for completion
    await waitForStatusBar('Indexed', { timeout: 60000 });

    // 5. Verify status reflects new branch
    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.tooltip).toContain('feature-branch');
  });
});
```

## Test Infrastructure

### Test Utilities

**File:** `src/test/utils/workspace.ts`
```typescript
export async function createTestRepo(files: FileSpec[]): Promise<string> {
  const tempDir = tmp.dirSync().name;

  // Initialize git
  await exec('git init', { cwd: tempDir });

  // Create files
  for (const file of files) {
    const filePath = path.join(tempDir, file.path);
    await fs.promises.mkdir(path.dirname(filePath), { recursive: true });
    await fs.promises.writeFile(filePath, file.content);
  }

  // Commit
  await exec('git add .', { cwd: tempDir });
  await exec('git commit -m "Initial commit"', { cwd: tempDir });

  return tempDir;
}

export async function openConfiguredWorkspace(workspacePath: string): Promise<void> {
  // Set configuration before opening
  await vscode.workspace.getConfiguration('maproom').update('autoStart', true);
  await vscode.workspace.getConfiguration('maproom').update('provider', 'ollama');

  // Open workspace
  await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(workspacePath));

  // Wait for activation
  await waitForExtensionActivation('maproom');
}
```

**File:** `src/test/utils/assertions.ts`
```typescript
export async function waitForStatusBar(
  expectedText: string,
  options = { timeout: 10000 }
): Promise<void> {
  const deadline = Date.now() + options.timeout;

  while (Date.now() < deadline) {
    const statusBar = await getStatusBarItem('Maproom');
    if (statusBar.text.includes(expectedText)) {
      return;
    }
    await sleep(100);
  }

  throw new Error(`Status bar did not show "${expectedText}" within ${options.timeout}ms`);
}

export async function waitForNotification(
  expectedMessage: string,
  options = { timeout: 30000 }
): Promise<void> {
  // Intercept VSCode notification API
  const notifications: string[] = [];

  const original = vscode.window.showInformationMessage;
  vscode.window.showInformationMessage = (message: string) => {
    notifications.push(message);
    return original(message);
  };

  const deadline = Date.now() + options.timeout;

  while (Date.now() < deadline) {
    if (notifications.some(n => n.includes(expectedMessage))) {
      vscode.window.showInformationMessage = original;
      return;
    }
    await sleep(100);
  }

  vscode.window.showInformationMessage = original;
  throw new Error(`Notification "${expectedMessage}" not shown within ${options.timeout}ms`);
}
```

### CI Configuration

**MVP CI Strategy:** Linux-only for E2E tests (pragmatic for MVP)

**File:** `.github/workflows/test-extension.yml`
```yaml
name: Test VSCode Extension

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        # MVP: Test on Linux only (devcontainer environment)
        # Post-MVP: Add macOS and Windows
        os: [ubuntu-latest]
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3

      - uses: actions/setup-node@v3
        with:
          node-version: 18

      - name: Install dependencies
        run: |
          cd packages/vscode-maproom
          pnpm install

      - name: Start Docker services
        run: |
          docker compose -f packages/maproom-mcp/config/docker-compose.yml up -d postgres
          # Wait for healthy
          timeout 60 bash -c 'until docker exec maproom-postgres pg_isready -U maproom; do sleep 2; done'

      - name: Run unit tests
        run: |
          cd packages/vscode-maproom
          pnpm test:unit

      - name: Run integration tests
        run: |
          cd packages/vscode-maproom
          pnpm test:integration

      - name: Run E2E tests
        run: |
          cd packages/vscode-maproom
          xvfb-run -a pnpm test:e2e
        env:
          TEST_OPENAI_KEY: ${{ secrets.TEST_OPENAI_KEY }}

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./packages/vscode-maproom/coverage/coverage-final.json
          flags: unittests
          fail_ci_if_error: false  # Don't fail on coverage upload errors (MVP)

      - name: Check coverage threshold
        run: |
          cd packages/vscode-maproom
          # Fail if coverage below 50%
          pnpm run coverage:check --lines 50 --functions 50 --branches 50
```

**Devcontainer Testing:**

Test in devcontainer environment (matches user environment):

```json
// .devcontainer/devcontainer.json (test configuration)
{
  "name": "Maproom Extension Test",
  "image": "mcr.microsoft.com/vscode/devcontainers/typescript-node:18",
  "features": {
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },
  "postCreateCommand": "pnpm install && pnpm build",
  "customizations": {
    "vscode": {
      "extensions": [
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode"
      ]
    }
  }
}
```

**Manual Cross-Platform Testing:**

Before release, manual testing checklist on:
- [ ] macOS Intel (local machine)
- [ ] macOS Apple Silicon (local machine or CI runner)
- [ ] Linux x64 (devcontainer or CI)
- [ ] Windows x64 (local machine or skip for MVP)

## Manual Testing Checklist

**Pre-Release Testing:**

### Installation
- [ ] VSIX installs successfully in VSCode
- [ ] VSIX installs successfully in Cursor
- [ ] Symlink installation works for development
- [ ] Extension activates on workspace open

### Setup Wizard
- [ ] Ollama setup completes without errors
- [ ] OpenAI setup validates API key
- [ ] Google setup validates project ID
- [ ] Invalid credentials show error message
- [ ] Setup can be cancelled
- [ ] Setup can be retried after failure

### Docker Management
- [ ] Services start on first activation
- [ ] Services don't restart if already running
- [ ] Error shown if Docker not installed
- [ ] Error shown if Docker daemon stopped
- [ ] Services stop on extension deactivation (if autoManage=true)
- [ ] Manual stop command works

### Indexing
- [ ] Initial scan shows progress notification
- [ ] Initial scan completes for small repo (<10 files)
- [ ] Initial scan completes for medium repo (100-1000 files)
- [ ] Initial scan can be cancelled
- [ ] File changes trigger upsert after 3s
- [ ] Multiple rapid file changes batched correctly
- [ ] Branch switch detected within 1s
- [ ] Branch switch triggers incremental scan

### Status Bar
- [ ] Shows "Setup Required" when not configured
- [ ] Shows "Indexing" during scan
- [ ] Shows "Indexed (Xm ago)" after completion
- [ ] Shows "Error" on failure
- [ ] Click opens status details

### Error Handling
- [ ] Docker not running shows helpful error
- [ ] Binary spawn failure shows error + logs
- [ ] Database connection failure shows error
- [ ] Invalid configuration shows error
- [ ] Retry buttons work

### Platform-Specific
- [ ] Works on macOS (Apple Silicon) - primary development platform
- [ ] Works on Linux (x64) in devcontainer - CI environment
- [ ] Works on macOS (Intel) - manual test before release
- [ ] Correct binary selected for each platform (test platform detection code)
- [ ] Devcontainer DinD mode works (Docker-in-Docker)
- [ ] Devcontainer DooD mode works (Docker-outside-of-Docker with socket mount)
- [ ] Windows x64 - defer to post-MVP (document as experimental)

**Edge Cases to Test:**

1. **Process Management:**
   - Test: Binary not found → clear error message
   - Test: Binary wrong platform → platform mismatch error
   - Test: Process crashes repeatedly → circuit breaker activates
   - Test: Stdout buffer overflow → incremental parsing works

2. **NDJSON Parsing:**
   - Test: Malformed JSON in stdout → skip line, continue
   - Test: Missing required fields → defaults used
   - Test: Binary version mismatch → warning shown
   - Test: Multiple events in one line → all parsed

3. **Process Lifecycle:**
   - Test: Extension deactivates during indexing → graceful shutdown
   - Test: Docker stops mid-operation → process fails, restart attempted
   - Test: Multiple restarts quickly → exponential backoff works

4. **Resource Constraints:**
   - Test: Large output burst → buffering works, no memory leak
   - Test: Binary crashes mid-scan → restart, resume watching
   - Test: Concurrent process crashes → both restart independently

5. **Edge Cases Delegated to Rust:**
   - Detached HEAD handling → Rust binary handles
   - Corrupted .git/HEAD → Rust binary handles
   - File watching debouncing → Rust binary handles
   - Rapid branch switches → Rust binary handles

## Performance Benchmarks

**Targets:**

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Extension activation | <500ms | `console.time` in `activate()` |
| Initial scan (10 files) | <30s | Progress notification duration |
| Initial scan (100 files) | <5min | Progress notification duration |
| File change detection | <1s | Time from save to upsert start |
| Branch switch detection | <1s | Time from checkout to scan start |
| Memory (idle) | <50MB | VSCode Process Explorer |
| Memory (indexing) | <200MB | VSCode Process Explorer |
| CPU (idle) | <5% | Task Manager / Activity Monitor |

**Benchmark Suite:**

**File:** `src/test/benchmarks/activation.bench.ts`
```typescript
import { bench, describe } from 'vitest';

describe('Extension Activation', () => {
  bench('activate extension', async () => {
    const start = performance.now();
    await activate(mockContext);
    const duration = performance.now() - start;

    expect(duration).toBeLessThan(500); // <500ms target
  });
});
```

## Quality Gates

**Pre-Commit:**
- [ ] All unit tests pass
- [ ] TypeScript compiles without errors
- [ ] No linting errors
- [ ] No security vulnerabilities (npm audit)

**Pre-PR:**
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Test coverage >50%
- [ ] Manual smoke test passed

**Pre-Release:**
- [ ] All tests pass (unit + integration + E2E)
- [ ] Manual testing checklist complete
- [ ] Performance benchmarks within targets
- [ ] No critical bugs
- [ ] Installation documentation tested
- [ ] Works on all target platforms

## Risk Mitigation

### High-Risk Areas

**1. Docker Service Orchestration**
- **Risk:** Services fail to start, block all functionality
- **Mitigation:**
  - Extensive integration tests with real Docker
  - Graceful error messages with actions
  - Manual override commands
  - Comprehensive logging

**2. Binary Spawning**
- **Risk:** Platform-specific failures, missing binaries
- **Mitigation:**
  - Test on all platforms in CI
  - Bundle all binaries in extension
  - Validate binary exists before spawn
  - Clear error if unsupported platform

**3. Process Crash Recovery**
- **Risk:** Processes crash and don't restart
- **Mitigation:**
  - Unit tests for crash recovery
  - Exponential backoff tested
  - Circuit breaker prevents infinite restarts
  - Manual testing of crash scenarios

**4. Credentials Storage**
- **Risk:** API keys leaked or lost
- **Mitigation:**
  - Integration tests for SecretStorage
  - Never log credentials
  - Validate storage on each platform
  - Clear migration path if storage fails

### Known Limitations (Accept for MVP)

**Not Testing:**
- Multi-workspace support (not implemented yet)
- Custom embedding models (not implemented yet)
- Advanced search features (not extension's responsibility)
- Offline mode edge cases (acceptable risk)

**Test Gaps (Acceptable):**
- Race conditions in watchers (rare, low impact)
- Extremely large repos (>10k files) - manual testing only
- Network filesystem edge cases - document as known issue
- Docker Desktop licensing changes - can't test

## Conclusion

**Quality Strategy Summary:**
1. **Unit tests** for complex logic (stdout parsing, process management)
2. **Integration tests** for Docker, binary spawning, process lifecycle
3. **E2E tests** for complete workflows (setup, process monitoring, status updates)
4. **Manual testing** for platform-specific issues and UX
5. **CI automation** to catch regressions early

**Success Criteria:**
- All critical paths have 100% test coverage
- Overall coverage >50% (reduced due to simpler architecture)
- All tests pass on CI
- Manual checklist complete
- Performance targets met

**Risk Acceptance:**
- File watching edge cases delegated to Rust binary
- Branch detection edge cases delegated to Rust binary
- E2E tests only on Linux (CI limitation)
- Large repo testing manual only

**Next:** Security review to identify and mitigate security risks.
