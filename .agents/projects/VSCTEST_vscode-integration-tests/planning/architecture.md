# Architecture: VS Code Integration Tests

## Overview

This project adds real VS Code integration tests using `@vscode/test-electron` to complement the existing Vitest unit tests. The focus is on verifying extension activation and setup sticking points in an actual VS Code environment.

## Architecture Decisions

### Decision 1: Use @vscode/test-electron Directly

**Choice**: Direct use of @vscode/test-electron without @vscode/test-cli wrapper

**Rationale**:
- More transparent control over VS Code download and launch
- Easier debugging when tests fail
- No additional abstraction layer to learn
- Aligns with existing project patterns (explicit over implicit)

**Trade-offs**:
- Slightly more boilerplate in test runner
- Manual configuration vs declarative config file

### Decision 2: Mocha for Integration Tests

**Choice**: Use Mocha for integration tests, keep Vitest for unit tests

**Rationale**:
- @vscode/test-electron is designed for Mocha
- Mocha supports synchronous test setup required for VS Code API
- Separates unit tests (fast, mocked) from integration tests (slow, real)
- No need to configure Vitest to work with @vscode/test-electron

**Trade-offs**:
- Two test frameworks in one package
- Different assertion styles (@types/mocha vs vitest expect)

### Decision 3: Separate Test Directory

**Choice**: `src/test/e2e/` for integration tests, separate from `src/test/` integration.test.ts

**Rationale**:
- Clear separation of test types
- Different execution requirements (Vitest vs Mocha)
- Allows different coverage configurations
- Prevents confusion about which tests run where

### Decision 4: Test Workspace Fixture

**Choice**: Create a minimal test workspace fixture in `src/test/e2e/fixtures/workspace/`

**Rationale**:
- Consistent test environment
- Predictable file structure for testing
- Avoids interference with actual project workspace
- Can include test SQLite database

## Directory Structure

```
packages/vscode-maproom/
├── src/
│   ├── test/
│   │   ├── integration.test.ts    # Existing Vitest integration tests
│   │   └── e2e/                   # NEW: VS Code integration tests
│   │       ├── runTests.ts        # Test runner (downloads VS Code)
│   │       ├── suite/
│   │       │   ├── index.ts       # Mocha test setup
│   │       │   ├── activation.test.ts
│   │       │   ├── commands.test.ts
│   │       │   ├── statusBar.test.ts
│   │       │   └── configuration.test.ts
│   │       └── fixtures/
│   │           └── workspace/     # Test workspace
│   │               ├── .vscode/
│   │               │   └── settings.json
│   │               └── sample.ts
│   └── ...
├── package.json                   # Add devDependencies
├── tsconfig.json                  # Exclude e2e from main build
├── tsconfig.e2e.json              # NEW: Separate tsconfig for e2e
└── .vscode/
    └── launch.json                # Debug configuration for e2e tests
```

## Component Design

### Test Runner (runTests.ts)

```typescript
import * as path from 'path';
import { runTests, downloadAndUnzipVSCode } from '@vscode/test-electron';

async function main() {
  try {
    const extensionDevelopmentPath = path.resolve(__dirname, '../../../');
    const extensionTestsPath = path.resolve(__dirname, './suite/index');
    const testWorkspace = path.resolve(__dirname, './fixtures/workspace');

    // Download VS Code if needed (cached)
    const vscodeExecutablePath = await downloadAndUnzipVSCode('stable');

    // Run tests
    await runTests({
      vscodeExecutablePath,
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [testWorkspace, '--disable-extensions'],
    });
  } catch (err) {
    console.error('Failed to run tests', err);
    process.exit(1);
  }
}

main();
```

### Test Suite Index (suite/index.ts)

```typescript
import * as path from 'path';
import * as Mocha from 'mocha';
import * as glob from 'glob';

export function run(): Promise<void> {
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 30000, // Extension tests need longer timeout
  });

  const testsRoot = path.resolve(__dirname, '.');

  return new Promise((resolve, reject) => {
    glob('**/**.test.js', { cwd: testsRoot }, (err, files) => {
      if (err) return reject(err);

      files.forEach(f => mocha.addFile(path.resolve(testsRoot, f)));

      try {
        mocha.run(failures => {
          if (failures > 0) {
            reject(new Error(`${failures} tests failed.`));
          } else {
            resolve();
          }
        });
      } catch (err) {
        reject(err);
      }
    });
  });
}
```

### Example Test (activation.test.ts)

```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension Activation', () => {
  suiteSetup(async function () {
    this.timeout(30000);
    // Wait for extension to activate
    const ext = vscode.extensions.getExtension('manifoldlogic.vscode-maproom');
    if (ext && !ext.isActive) {
      await ext.activate();
    }
  });

  test('Extension should be present', () => {
    const ext = vscode.extensions.getExtension('manifoldlogic.vscode-maproom');
    assert.ok(ext, 'Extension should be found');
  });

  test('Extension should activate', async function () {
    this.timeout(10000);
    const ext = vscode.extensions.getExtension('manifoldlogic.vscode-maproom');
    assert.ok(ext?.isActive, 'Extension should be active');
  });

  test('Commands should be registered', async () => {
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes('maproom.showOutput'), 'showOutput command');
    assert.ok(commands.includes('maproom.setup'), 'setup command');
    assert.ok(commands.includes('maproom.restartWatchers'), 'restartWatchers command');
    assert.ok(commands.includes('maproom.showStatus'), 'showStatus command');
  });
});
```

## Integration Points

### With Existing Unit Tests

- Unit tests (Vitest): `pnpm test` - Fast, mocked, runs everywhere
- Integration tests (Mocha): `pnpm test:e2e` - Slow, real VS Code, requires display

### With Devcontainer

The devcontainer needs xvfb for headless display:

```dockerfile
# Add to .devcontainer/Dockerfile or as feature
RUN apt-get update && apt-get install -y xvfb
```

Test script with xvfb:
```bash
xvfb-run -a pnpm test:e2e
```

### With CI/CD

GitHub Actions workflow addition:

```yaml
test-e2e:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
    - run: pnpm install
    - run: pnpm build
    - run: xvfb-run -a pnpm test:e2e
```

## Performance Considerations

### VS Code Download Caching

- VS Code is downloaded once per version
- Cache in CI: `~/.vscode-test/`
- Download takes ~30-60 seconds first time

### Test Execution Time

- Extension activation: ~2-5 seconds
- Each test: ~1-3 seconds
- Total suite (5-8 tests): ~30-60 seconds

### Parallel Execution

Tests run sequentially within the VS Code instance. Multiple VS Code instances for parallelism is complex and not recommended.

## Technology Choices

| Component | Choice | Reason |
|-----------|--------|--------|
| Test framework | @vscode/test-electron | Official, well-maintained |
| Test runner | Mocha | VS Code test standard |
| Assertions | Node assert | Simple, built-in |
| Display | xvfb | Headless Linux testing |
| Fixtures | Static workspace | Predictable, fast |

## Error Handling

### Missing Display
```typescript
if (!process.env.DISPLAY && process.platform === 'linux') {
  console.error('No display available. Use xvfb-run or set DISPLAY.');
  process.exit(1);
}
```

### Extension Not Found
```typescript
const ext = vscode.extensions.getExtension('manifoldlogic.vscode-maproom');
if (!ext) {
  throw new Error('Extension not found. Check publisher and name in package.json');
}
```

### Timeout Handling
```typescript
test('Activation completes', async function () {
  this.timeout(10000); // Per-test timeout
  // ... test code
});
```

## Dependencies

### New devDependencies
```json
{
  "@vscode/test-electron": "^2.4.1",
  "@types/mocha": "^10.0.0",
  "@types/glob": "^8.1.0",
  "mocha": "^10.0.0",
  "glob": "^10.0.0"
}
```

### Package.json Scripts
```json
{
  "scripts": {
    "test": "vitest run",
    "test:e2e": "node ./out/test/e2e/runTests.js",
    "test:e2e:compile": "tsc -p ./tsconfig.e2e.json",
    "pretest:e2e": "pnpm run test:e2e:compile"
  }
}
```

## Future Considerations

### Not Implemented Now (MVP Focus)
- Visual regression testing
- Performance benchmarking
- Multiple VS Code versions
- Web extension testing

### Extension Points
- Add more test files as needed
- Increase test workspace complexity
- Add mock SQLite database to fixtures
- Test with different VS Code settings
