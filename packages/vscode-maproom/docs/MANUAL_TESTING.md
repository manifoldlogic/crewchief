# VSMAP Manual Testing Protocol

This document provides a comprehensive manual testing checklist for the VSCode Maproom extension across all supported platforms and configurations.

## Test Environment Setup

### Required Test Platforms
- **Linux x64**: Ubuntu 22.04+ or equivalent
- **macOS arm64**: M1/M2/M3/M4 Mac (recommended)
- **macOS x64**: Intel Mac (optional)
- **Windows x64**: Windows 10/11 (experimental)

### Required Software
- Visual Studio Code 1.85+
- Docker Desktop (or compatible Docker runtime)
- Git
- Test workspace with 1000+ code files

### DevContainer Modes
- **DinD** (Docker-in-Docker): Nested Docker containers
- **DooD** (Docker-outside-of-Docker): Access to host Docker socket

## Manual Testing Checklist

### 1. Extension Activation

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Extension loads without errors | No error notifications, extension activated | | |
| Docker services start within 30s | PostgreSQL container healthy | | |
| Status bar appears | Shows "Starting..." then "Watching" | | |
| Output channel shows logs | Docker startup logs visible | | |
| Activation completes <500ms | Extension responsive immediately | | |

**How to Test:**
1. Install extension from VSIX
2. Open workspace in VSCode
3. Check status bar (bottom right)
4. Open Output panel (View → Output → Maproom)
5. Verify Docker container: `docker ps | grep maproom`

### 2. Setup Wizard

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Wizard appears on first activation | QuickPick with provider options | | |
| Ollama detection works | Marked "Recommended" if running | | |
| Provider selection persists | Saved in workspace state | | |
| Credential input works | Password masked, stored in SecretStorage | | |
| Initial scan triggers automatically | Progress notification appears | | |

**How to Test:**
1. Fresh workspace (no previous config)
2. Activate extension
3. Select provider from QuickPick
4. Enter credentials (if required)
5. Verify scan starts automatically
6. Restart VSCode, verify provider remembered

### 3. Indexing Workflow

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Initial scan shows progress | Dismissible notification with percentage | | |
| Progress updates frequently | File counts increment smoothly | | |
| File counts formatted with commas | e.g., "1,234 files" not "1234 files" | | |
| Scan completes without errors | Status bar shows "Indexed: N files" | | |
| Completion timestamp recorded | Tooltip shows "Last indexed: X ago" | | |
| Large repositories handled | 10,000+ files index successfully | | |

**How to Test:**
1. Open large workspace (1000+ files)
2. Run setup wizard
3. Watch progress notification
4. Verify file count increments
5. Check status bar after completion
6. Hover over status bar for tooltip

### 4. Error Handling

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Docker not running | Clear error: "Docker not available" | | |
| Binary missing/corrupted | Actionable error with path to binary | | |
| Invalid credentials | Helpful message, re-prompt option | | |
| Process crash | Automatic restart with backoff | | |
| Network errors | Graceful degradation, retry logic | | |

**How to Test:**
1. Stop Docker, activate extension
2. Rename binary, trigger scan
3. Enter wrong API key, start scan
4. Kill watch process: `pkill -f maproom`
5. Disconnect network during indexing

### 5. Crash Recovery

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Single crash → auto-restart | 1s delay, process restarts | | |
| Multiple crashes → exponential backoff | 1s, 2s, 4s, 8s, 16s delays | | |
| 5 crashes → circuit breaker | Error notification with "Show Logs" | | |
| Manual restart command works | Command palette: "Maproom: Restart Watchers" | | |
| Reset after 60s success | Counter resets after stable runtime | | |

**How to Test:**
1. Start watch process
2. Kill process: `pkill -f "maproom watch"`
3. Observe Output channel for restart logs
4. Kill repeatedly to trigger circuit breaker
5. Use command palette to restart manually
6. Wait 60s, verify counter resets

### 6. Status Bar Updates

| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Starting state | "$(sync~spin) Starting..." | | |
| Watching state | "$(eye) Indexed: 1,234 files" | | |
| Indexing state | "$(sync~spin) Indexing: 1,234 files" | | |
| Error state | "$(error) Maproom Error" | | |
| Tooltip shows details | File counts, last indexed time, error info | | |
| Click opens Output | Output panel visible | | |

**How to Test:**
1. Observe status bar during each state
2. Hover for tooltip
3. Click status bar icon
4. Verify Output panel opens

### 7. Platform-Specific Tests

#### Linux
| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Binary has execute permissions | `chmod +x` not required | | |
| Docker socket accessible | `/var/run/docker.sock` readable | | |
| File watching works | inotify events captured | | |
| Systemd integration | Extension starts on login (if configured) | | |

#### macOS
| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Rosetta translation works (Intel binary on Apple Silicon) | Runs without translation layer | | |
| Native arm64 binary works | Optimized performance | | |
| FSEvents file watching | Git operations trigger updates | | |
| Docker Desktop integration | Containers start via Docker Desktop | | |

#### Windows
| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Binary execution | `.exe` runs without admin | | |
| Path separators handled | Windows paths work correctly | | |
| Docker Desktop on WSL2 | Containers accessible | | |
| File watching on network drives | CIFS/SMB paths supported | | |

### 8. DevContainer Testing

#### DinD (Docker-in-Docker)
| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Extension activates in container | No "Docker unavailable" errors | | |
| Nested Docker works | PostgreSQL container runs inside devcontainer | | |
| Database persists | Data survives container restart | | |
| Performance acceptable | Indexing not significantly slower | | |

#### DooD (Docker-outside-of-Docker)
| Test | Expected Result | Pass/Fail | Notes |
|------|----------------|-----------|-------|
| Host Docker accessible | Socket mounted at `/var/run/docker.sock` | | |
| Containers visible on host | `docker ps` shows maproom containers | | |
| Network connectivity | Extension can reach containers | | |
| Resource sharing | Uses host Docker resources efficiently | | |

**How to Test:**
1. Open project in VSCode Dev Container
2. Let container build completely
3. Verify extension activates
4. Check `docker ps` inside container (DinD) or on host (DooD)
5. Run indexing workflow
6. Rebuild container, verify database persists

## Test Report Template

```markdown
# VSMAP Manual Test Report

**Date**: YYYY-MM-DD
**Extension Version**: 0.1.0
**Tester**: [Your Name]
**Git Commit**: [commit hash]

## Test Summary
- **Total Tests**: X
- **Passed**: X
- **Failed**: X
- **Skipped**: X

## Platform: Linux x64 (Ubuntu 22.04)
**Docker Version**: 24.0.7
**VSCode Version**: 1.85.0
**Node Version**: v20.10.0

| Scenario | Result | Duration | Notes |
|----------|--------|----------|-------|
| Extension Activation | ✅ PASS | 3.2s | No errors |
| Setup Wizard | ✅ PASS | 15s | Ollama detected |
| Initial Scan | ✅ PASS | 45s | 1,234 files indexed |
| Crash Recovery | ✅ PASS | 30s | 5 crashes handled correctly |
| Status Bar Updates | ✅ PASS | N/A | All states displayed |
| Error Handling | ⚠️ PARTIAL | N/A | See bug VSMAP-4005 |

**Issues Found:**
- Status bar flickers when switching tabs (Low severity)

---

## Platform: macOS arm64 (M2)
**Docker Version**: 24.0.7
**VSCode Version**: 1.85.0
**macOS Version**: 14.1.2

| Scenario | Result | Duration | Notes |
|----------|--------|----------|-------|
| Extension Activation | ✅ PASS | 2.8s | Native arm64 binary |
| Setup Wizard | ✅ PASS | 12s | |
| Initial Scan | ✅ PASS | 38s | Faster than Linux |
| ... | ... | ... | ... |

---

## Bugs Found

### VSMAP-4005: Status bar flickers on Linux
**Severity**: Low
**Platform**: Ubuntu 22.04 x64
**Reproduction Steps**:
1. Activate extension
2. Switch between editor tabs rapidly
3. Observe status bar icon

**Expected**: Status bar stable
**Actual**: Icon flickers/disappears briefly

**Screenshot**: [link]

---

## DevContainer Testing

### DinD Mode
**Base Image**: mcr.microsoft.com/devcontainers/typescript-node:20
**Result**: ✅ PASS

- Nested Docker works correctly
- Database persists across rebuilds
- Performance: ~20% slower than native

### DooD Mode
**Result**: ✅ PASS

- Host Docker accessible
- Performance: Same as native
- Resource sharing efficient

---

## Recommendations

1. **Release Blockers**: None
2. **Known Issues**: Document status bar flicker (VSMAP-4005)
3. **Windows Support**: Mark as experimental in README
4. **Performance**: Excellent on macOS arm64, acceptable on all platforms

**Overall**: READY FOR RELEASE
```

## Filing Bugs

When critical issues are found during manual testing:

1. **Create ticket**: `.crewchief/projects/VSMAP_vscode-maproom-extension/tickets/VSMAP-40XX_bug-description.md`
2. **Include**:
   - Platform details (OS, version, architecture)
   - Reproduction steps (numbered, specific)
   - Expected vs actual behavior
   - Screenshots/logs
   - Severity: Critical (blocks release), High, Medium, Low
3. **Link to test report** for context

## Test Execution

Manual testing should be performed:
- **Before every release**
- **After major features** (Phase completion)
- **On new platforms** (when adding support)
- **After critical bugs** (regression testing)

Estimated time: 2-4 hours per platform (first run), 1-2 hours (subsequent)
