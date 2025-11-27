# Analysis: VSCode Extension Daemon Migration

## Problem Definition

The VSCode extension (`packages/vscode-maproom`) uses an outdated architecture that doesn't align with the current Maproom infrastructure. Several issues need resolution:

### 1. Dual Watch Processes (Outdated)

**Current State**: The extension spawns TWO separate processes via `ProcessOrchestrator`:
- `crewchief-maproom watch` - File change monitoring
- `crewchief-maproom branch-watch` - Git branch switch monitoring

**Problem**: The Rust `watch` command was unified in the UNIWATCH project to handle both file watching AND branch detection in a single process. The extension still uses the old dual-process approach.

**Evidence** (`orchestrator.ts:167-184`):
```typescript
await this.startProcess('watch', ['watch', '--repo', ...])
await this.startProcess('branch-watch', ['branch-watch', ...])
```

### 2. Docker Container Dependency (To Be Removed)

**Current State**: The extension manages Docker containers via `DockerManager`:
- Starts PostgreSQL container (`maproom-postgres`)
- Optionally starts Ollama container
- Manages container lifecycle tied to extension activation

**Problem**: The target architecture uses:
- **SQLite** as the primary local database (no PostgreSQL container needed)
- **Host Ollama** instead of containerized Ollama
- No Docker dependency for local development

**Evidence** (`extension.ts:275-322`):
```typescript
async function ensureDockerRunning(context, provider): Promise<void> {
  const dockerManager = new DockerManager(outputChannel!)
  await dockerManager.ensureServicesRunning()
}
```

### 3. No Scan-on-Startup (Missing Feature)

**Current State**: The extension either:
- Runs initial scan (first-time setup only)
- Or jumps directly to watch mode (returning users)

**Problem**: When the extension starts, it should catch up on any changes that occurred while it wasn't running. The watch command should automatically scan/reconcile before entering watch mode.

**Desired Behavior**:
```
Extension starts → Watch command starts → Scan/catchup runs → Watch mode begins
```

### 4. Ollama Model Management (Missing Feature)

**Current State**: The extension detects if Ollama is running (`setupWizard.ts:77`):
```typescript
const ollamaRunning = await detectOllama()
```

**Problem**: If Ollama is running but the embedding model isn't downloaded, indexing will fail. The extension should:
1. Detect Ollama running
2. Check if required model exists
3. Pull model if missing (`ollama pull nomic-embed-text`)

### 5. Cruft from Previous Architecture

The codebase contains leftover code from the old architecture:

| Component | Location | Issue |
|-----------|----------|-------|
| `DockerManager` | `src/docker/manager.ts` | No longer needed |
| `docker/index.ts` | `src/docker/index.ts` | Exports for removed feature |
| `example-usage.ts` | `src/docker/example-usage.ts` | Dead code |
| `postgres-checker.ts` | `src/services/postgres-checker.ts` | Only for PostgreSQL mode |
| Dual watch process logic | `orchestrator.ts` | Needs update to single process |
| PostgreSQL settings | `package.json` contributions | Settings UI for removed feature |

## Current Project State

### What Exists

| Component | Status | Notes |
|-----------|--------|-------|
| Unified `watch` command | ✅ Complete | Rust side handles file + branch watching |
| SQLite backend | ✅ Complete | Full SQLite support with sqlite-vec |
| `daemon-client` package | ✅ Complete | For MCP server, reusable by extension |
| Ollama detection | ✅ Complete | `setupWizard.ts` pings localhost:11434 |
| `upsert` CLI command | ✅ Complete | Can be used for reconciliation |

### Reusable Extension Infrastructure

These components should be **reused**, not replaced:

| Component | Location | Capability |
|-----------|----------|------------|
| `StdoutParser` | `process/parser.ts` | NDJSON parsing with line buffering |
| `CrashRecovery` | `process/recovery.ts` | Exponential backoff, circuit breaker |
| `detectOllama()` | `ui/setupWizard.ts` | HTTP ping to localhost:11434 |
| `SecretsManager` | `config/secrets.ts` | VSCode SecretStorage for API keys |
| `getRepoName()` | `utils/git.ts` | Git remote origin detection |
| `getBranchName()` | `utils/git.ts` | Current branch detection |
| `showNoSqliteGuidance()` | `ui/setupWizard.ts` | First-run user guidance |

### Verified CLI Interfaces

**Watch command** (verified):
```bash
crewchief-maproom watch [OPTIONS]
  --repo <REPO>          # Repository name (defaults to git remote origin)
  --path <PATH>          # Path to watch (defaults to current directory)
  --throttle <THROTTLE>  # Debounce interval [default: 2s]
  --worktree <WORKTREE>  # DEPRECATED: auto-detected from branch
```

**Upsert command** (for reconciliation):
```bash
crewchief-maproom upsert --commit <COMMIT> --repo <REPO> --worktree <WORKTREE> --root <ROOT> [--paths <PATHS>]
```

### What Needs Work

| Component | Status | Work Required |
|-----------|--------|---------------|
| Single watch process | ❌ Missing | Refactor `ProcessOrchestrator` |
| Startup reconciliation | ❌ Missing | TypeScript using `git diff` + `upsert` |
| `BranchSwitchedEvent` | ❌ Missing | Add to `events.ts` |
| Ollama model pull | ❌ Missing | New `OllamaClient` class |
| Docker removal | ❌ Not started | Remove `DockerManager`, update flows |
| Cruft cleanup | ❌ Not started | Remove dead code, unused settings |

## Industry Solutions Research

### Scan-on-Startup Patterns

**Git's approach**: `git status` scans the worktree against the index each time. For large repos, this can be slow but is always correct.

**IDE indexers (IntelliJ, VSCode built-in)**:
- Maintain persistent index on disk
- On startup, walk filesystem comparing mtimes
- Only re-index changed files
- Use file watcher after initial reconciliation

**Recommended approach for Maproom**:
- Use git to identify changed files since last indexed commit
- Upsert only changed files (incremental)
- Then enter watch mode

### Ollama Model Management

**Ollama CLI patterns**:
```bash
ollama pull nomic-embed-text  # Downloads if missing
ollama list                    # Check if model exists
```

**API approach** (more robust):
```bash
# Check model
curl http://localhost:11434/api/tags | jq '.models[].name'

# Pull model (streaming progress)
curl http://localhost:11434/api/pull -d '{"name":"nomic-embed-text"}'
```

## Constraints and Considerations

### Interface Stability

| Interface | Stable? | Notes |
|-----------|---------|-------|
| SQLite schema | ✅ Yes | Well-defined, migration system |
| Watch command CLI | ✅ Yes | `--repo`, `--path` flags finalized |
| NDJSON events | ✅ Yes | Documented event types |
| Ollama API | ✅ Yes | Standard HTTP endpoints |
| daemon-client API | ✅ Yes | TypeScript package with types |

### Context Coherence

The project touches a focused area:
- ~10 files in `packages/vscode-maproom/src/`
- Single domain: extension lifecycle and process management
- Clear boundaries: extension ↔ Rust binary ↔ SQLite

### Testable Completion

Success criteria are measurable:
- [ ] Single `watch` process spawned (no `branch-watch`)
- [ ] No Docker containers started
- [ ] Ollama model pulled automatically
- [ ] Changed files indexed on startup
- [ ] Extension activates < 500ms
- [ ] No TypeScript errors
- [ ] All tests pass

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| `branch_switched` event not in events.ts | High | Resolved | Add event type in Phase 1 |
| Watch CLI flags incorrect | Medium | Resolved | CLI verified via `--help` |
| SQLite URL format mismatch | Medium | Low | Documented: `sqlite://` prefix |
| Ollama not installed | Medium | Medium | Error with "Install Ollama" link |
| First run no last commit | Low | Low | Skip reconciliation gracefully |

## Summary

This project is well-bounded:
- **Interface Stability**: All external interfaces (SQLite, watch CLI, Ollama API) are stable and verified
- **Context Coherence**: ~10 files in single package, one domain
- **Testable Completion**: Clear binary criteria
- **Reuse Opportunity**: Significant existing infrastructure to leverage

The work involves:
1. Adding `BranchSwitchedEvent` to events.ts (prerequisite)
2. Refactoring `ProcessOrchestrator` for single watch (not replacing)
3. Implementing TypeScript-based reconciliation (no Rust changes)
4. Adding Ollama model management (extends `detectOllama()`)
5. Removing Docker dependency
6. Cleaning up cruft
