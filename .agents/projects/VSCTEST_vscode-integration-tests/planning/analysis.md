# Analysis: VS Code Integration Tests with @vscode/test-electron

## Problem Definition

The vscode-maproom extension currently has comprehensive unit tests (270+ tests, 71% coverage) using Vitest with mocked VS Code APIs. However, these tests cannot catch issues that only manifest in a real VS Code environment:

1. **Activation timing issues** - Extension activation event handling, startup performance
2. **Real VS Code API behavior** - Quirks and version differences in actual APIs
3. **Command registration verification** - Commands actually appear in command palette
4. **Status bar rendering** - Status bar items display correctly
5. **Extension context behavior** - Real workspace state, secrets storage
6. **Devcontainer compatibility** - Extension works correctly in devcontainer environments

The previous VSMAP-4001 ticket explicitly chose to use Vitest mocks instead of `@vscode/test-electron` due to infrastructure complexity. This project addresses that gap.

## Current State

### Existing Test Infrastructure
- **Test framework**: Vitest 1.0.0
- **Coverage**: @vitest/coverage-v8
- **Test files**: 18 test files in `packages/vscode-maproom/src/`
- **Mocking approach**: vi.mock('vscode', ...) with manual mocks

### Extension Activation Points
The extension has several sticking points that warrant real-environment testing:

1. **onStartupFinished activation** - Extension activates after VS Code startup
2. **Workspace folder requirement** - Extension requires open workspace
3. **Provider configuration check** - First-time setup wizard flow
4. **Database availability check** - SQLite database detection
5. **Ollama model verification** - Embedding model availability (ollama provider)
6. **Process spawning** - crewchief-maproom binary execution

### Devcontainer Environment
The project uses a Docker-based devcontainer with:
- Node.js 20, Rust, Python
- Docker-in-Docker capability
- PostgreSQL via docker-compose
- Volume mounts for SSH and git config
- Host network access for Ollama (host.docker.internal)

## Industry Solutions

### @vscode/test-electron
The official Microsoft solution for VS Code extension integration testing:
- Downloads and runs real VS Code instances
- Full access to VS Code Extension API
- Supports test workspaces and fixtures
- Works with Mocha test framework
- Requires display server (X11/xvfb) on Linux

### @vscode/test-cli (Optional)
A CLI wrapper that simplifies configuration:
- Declarative config file (.vscode-test.mjs)
- Automatic VS Code download
- Watch mode and coverage support
- Less boilerplate than direct test-electron use

### Alternative: vscode-extension-tester
A third-party tool with WebDriver-based UI testing. More complex, designed for UI automation rather than API integration testing. Not needed for this project's scope.

## Research Findings

### Key Integration Test Scenarios

Based on extension.ts analysis, these are the critical paths to test:

1. **Activation Success Path**
   - Extension activates in <500ms
   - Output channel created
   - Status bar shows "Starting..."
   - Commands registered

2. **First-Time Setup Flow**
   - No provider configured → Setup wizard appears
   - User selects provider → Configuration saved
   - Services initialize → Status bar shows "Watching"

3. **Database Not Found Scenario**
   - SQLite database missing
   - Guidance message shown
   - Extension enters degraded mode (idle status)

4. **Ollama Not Running Scenario**
   - Ollama provider selected but service unavailable
   - Error notification displayed
   - Retry option offered

5. **Binary Spawning**
   - crewchief-maproom binary found
   - Process spawns successfully
   - NDJSON events parsed correctly

### Devcontainer Challenges

Running @vscode/test-electron in a devcontainer requires:

1. **Display Server**: xvfb-run for headless testing
2. **VS Code Download**: Sufficient disk space and network access
3. **Path Resolution**: Binary paths must work in container context
4. **Docker Access**: Extension tests may need Docker socket access

### Minimal Test Approach

The user requested focused tests on "sticking points" rather than comprehensive coverage. Priority areas:

1. **Extension Activation** - Does it activate without errors?
2. **Command Registration** - Do commands appear in palette?
3. **Status Bar Creation** - Does status bar item appear?
4. **Configuration Access** - Can extension read/write settings?
5. **Error Handling** - Does it fail gracefully when dependencies missing?

## Scope Definition

### In Scope
- @vscode/test-electron installation and configuration
- Test runner setup (runTests.ts)
- 5-8 focused integration tests for sticking points
- Devcontainer compatibility (xvfb configuration)
- CI workflow updates for integration tests

### Out of Scope
- UI automation testing (vscode-extension-tester)
- Full E2E testing of all extension features
- Performance benchmarking
- Visual regression testing
- @vscode/test-cli (adds complexity without clear benefit)

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| xvfb issues in devcontainer | High | Pre-test xvfb verification |
| VS Code download failures in CI | Medium | Cache downloaded VS Code |
| Test flakiness | Medium | Timeouts, retry logic, sequential execution |
| Binary path resolution | Medium | Environment-aware path resolution |
| Ollama unavailable in test | Low | Skip Ollama tests if not running |

## Recommendations

1. **Use @vscode/test-electron directly** - More control than test-cli, clearer debugging
2. **Keep tests focused** - 5-8 tests on sticking points, not comprehensive coverage
3. **Make tests optional in CI** - Allow CI to pass if VS Code tests skipped
4. **Document devcontainer setup** - Clear instructions for running locally
5. **Create test workspace fixture** - Minimal workspace for consistent test environment
