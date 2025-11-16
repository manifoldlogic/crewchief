# Quality Strategy: VSCode Maproom Extension

## Philosophy

**Pragmatic Testing for MVPs:**
- Tests prevent rework, not check boxes
- Focus on integration points and critical paths
- Unit tests for complex logic, not trivial getters
- E2E tests for user workflows
- Mock sparingly, prefer real dependencies when fast

**Confidence Over Coverage:**
- 60% coverage target for MVP (not 100%)
- Critical paths: 100% coverage (activation, scanning, Docker startup)
- Trivial code: Skip tests (getters/setters, simple formatting)
- Complex logic: Comprehensive tests (debouncing, state machines, error handling)
- Integration boundaries: Extensive tests (Docker, binary spawning, database health)

**Coverage Targets by Component:**
- Core logic (debouncer, branch watcher): 90-100%
- Managers (Docker, Indexing, Config): 70-80%
- UI components (status bar, wizard): 50-60%
- Utilities: 60-70%
- **Overall MVP target: 60% (reduced from 70% for pragmatic MVP)**

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

### CP3: File Change Watching

**Flow:**
```
File saved → Debounce 3s → Upsert spawned → Index updated → Status bar updated
```

**Risk:** Medium (daily workflow)

**Test Coverage:** 90%

**Tests:**
1. Single file change triggers upsert
2. Multiple rapid changes debounced correctly
3. Large file changes handled
4. Binary files ignored
5. .git directory changes ignored
6. Upsert failure doesn't crash watcher
7. File deletion handled

### CP4: Branch Switch Detection

**Flow:**
```
git checkout → .git/HEAD changes → Watcher detects → Incremental scan → Index updated
```

**Risk:** High (index staleness)

**Test Coverage:** 100%

**Tests:**
1. Branch switch detected within 1s (measure time from `git checkout` to callback)
2. Incremental scan triggered (verify `--worktree` parameter in spawned command)
3. Content-addressed deduplication works (binary handles this, verify scan completes)
4. Detached HEAD handled (40-char SHA parsed correctly, truncated to 7 chars for display)
5. .git/HEAD parse errors handled (empty file, corrupted ref, missing file)
6. Concurrent branch switches queued (rapid checkout A→B→C results in only C scan)
7. Rebase operations debounced (10 HEAD changes during rebase → 1 scan after 500ms stable)
8. Branch switch during active scan (existing scan cancelled, new scan starts)

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

**File:** `src/indexing/debouncer.test.ts`
```typescript
describe('FileChangeDebouncer', () => {
  it('should collect changes for 3 seconds', async () => {
    const debouncer = new FileChangeDebouncer(3000);
    const changes: string[][] = [];

    debouncer.onChange((files) => changes.push(files));

    debouncer.add('file1.ts');
    await sleep(1000);
    debouncer.add('file2.ts');
    await sleep(1000);
    debouncer.add('file3.ts');
    await sleep(3100); // Total > 3s from last change

    expect(changes).toHaveLength(1);
    expect(changes[0]).toEqual(['file1.ts', 'file2.ts', 'file3.ts']);
  });

  it('should reset timer on new changes', async () => {
    const debouncer = new FileChangeDebouncer(3000);
    const changes: string[][] = [];

    debouncer.onChange((files) => changes.push(files));

    debouncer.add('file1.ts');
    await sleep(2000);
    debouncer.add('file2.ts'); // Resets timer
    await sleep(2000); // Only 2s from file2, not enough

    expect(changes).toHaveLength(0); // Not fired yet
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

**File:** `src/indexing/branchWatcher.test.ts`
```typescript
describe('BranchWatcher', () => {
  it('should parse branch name from .git/HEAD', () => {
    const tempDir = createTempGitRepo();
    fs.writeFileSync(
      path.join(tempDir, '.git', 'HEAD'),
      'ref: refs/heads/main'
    );

    const watcher = new BranchWatcher(tempDir, async () => {});
    expect(watcher.currentBranch).toBe('main');
  });

  it('should detect detached HEAD', () => {
    const tempDir = createTempGitRepo();
    fs.writeFileSync(
      path.join(tempDir, '.git', 'HEAD'),
      'abc1234567890def'
    );

    const watcher = new BranchWatcher(tempDir, async () => {});
    expect(watcher.currentBranch).toBe('abc1234'); // Truncated
  });
});
```

**What NOT to Unit Test:**
- Simple getters/setters
- VSCode API calls (integration test instead)
- Configuration reading (use real config)
- Status bar updates (E2E test instead)

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

**File:** `src/test/e2e/file-watching.test.ts`
```typescript
describe('File Watching E2E', () => {
  beforeAll(async () => {
    await openConfiguredWorkspace(testWorkspace);
    await waitForStatusBar('Indexed');
  });

  it('should auto-update index on file save', async () => {
    // 1. Open file
    const doc = await vscode.workspace.openTextDocument(
      path.join(testWorkspace, 'src', 'test.ts')
    );

    // 2. Edit file
    const edit = new vscode.WorkspaceEdit();
    edit.insert(doc.uri, new vscode.Position(0, 0), '// New comment\n');
    await vscode.workspace.applyEdit(edit);

    // 3. Save file
    await doc.save();

    // 4. Wait for debounce + upsert
    await sleep(5000);

    // 5. Verify status bar updated
    const statusBar = await getStatusBarItem('Maproom');
    expect(statusBar.text).toContain('just now');
  });

  it('should batch multiple file changes', async () => {
    // Save 3 files rapidly
    for (const file of ['a.ts', 'b.ts', 'c.ts']) {
      const doc = await vscode.workspace.openTextDocument(
        path.join(testWorkspace, 'src', file)
      );
      const edit = new vscode.WorkspaceEdit();
      edit.insert(doc.uri, new vscode.Position(0, 0), '// Edit\n');
      await vscode.workspace.applyEdit(edit);
      await doc.save();
      await sleep(500); // Within 3s window
    }

    // Should only trigger ONE upsert after 3s
    await sleep(3500);

    // Verify all files updated (check logs)
    const logs = getOutputChannelLogs('Maproom');
    const upsertCalls = logs.filter(l => l.includes('upsert --paths'));
    expect(upsertCalls).toHaveLength(1);
    expect(upsertCalls[0]).toContain('a.ts,b.ts,c.ts');
  });
});
```

**File:** `src/test/e2e/branch-switching.test.ts`
```typescript
describe('Branch Switching E2E', () => {
  beforeAll(async () => {
    await openConfiguredWorkspace(testGitRepo);
    await waitForStatusBar('Indexed');
  });

  it('should re-index on branch switch', async () => {
    // 1. Switch branch via git command
    await exec('git checkout feature-branch', { cwd: testGitRepo });

    // 2. Wait for detection
    await sleep(2000);

    // 3. Verify notification
    await waitForNotification('Branch changed, re-indexing');

    // 4. Wait for completion
    await waitForStatusBar('Indexed', { timeout: 60000 });

    // 5. Verify correct worktree indexed
    const stats = await getIndexStats();
    expect(stats.worktree).toBe('feature-branch');
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
          # Fail if coverage below 60%
          pnpm run coverage:check --lines 60 --functions 60 --branches 60
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

1. **Detached HEAD State:**
   - Test: `git checkout <commit-sha>` → verify status bar shows truncated SHA
   - Test: Scan triggered with detached HEAD → verify `--worktree` uses SHA

2. **Corrupted .git/HEAD:**
   - Test: Write empty file to .git/HEAD → verify error notification
   - Test: Write invalid content to .git/HEAD → verify retry logic
   - Test: Delete .git/HEAD → verify error state, watching disabled

3. **Concurrent Operations:**
   - Test: Branch switch during active scan → verify cancel + restart
   - Test: File save during branch switch scan → verify queued for next batch
   - Test: Multiple rapid branch switches → verify only last one scanned

4. **Resource Constraints:**
   - Test: Scan repository with 10,000 files → verify doesn't crash
   - Test: Save 1000 files rapidly → verify MAX_WAIT flush
   - Test: Binary crashes mid-scan → verify error handling, retry offered

5. **Network Filesystem (NFS/SMB):**
   - Document as known limitation (file watcher may miss changes)
   - Provide manual rescan command as workaround

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
- [ ] Test coverage >70%
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

**3. File System Watching**
- **Risk:** Missed changes, excessive CPU usage
- **Mitigation:**
  - E2E tests for file watching
  - Debounce implementation tested
  - Performance benchmarks
  - Manual testing on slow filesystems (network mounts)

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
1. **Unit tests** for complex logic (debouncing, parsing, validation)
2. **Integration tests** for Docker, binary spawning, database
3. **E2E tests** for complete workflows (setup, indexing, watching)
4. **Manual testing** for platform-specific issues and UX
5. **CI automation** to catch regressions early

**Success Criteria:**
- All critical paths have 100% test coverage
- Overall coverage >70%
- All tests pass on CI
- Manual checklist complete
- Performance targets met

**Risk Acceptance:**
- Some edge cases untested (network filesystems, race conditions)
- E2E tests only on Linux (CI limitation)
- Large repo testing manual only

**Next:** Security review to identify and mitigate security risks.
