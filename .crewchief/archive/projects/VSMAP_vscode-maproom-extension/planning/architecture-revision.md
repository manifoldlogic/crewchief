# Architecture Revision: VSMAP

**Date**: 2025-11-16
**Reason**: Eliminate duplication with existing Rust binary and CLI functionality
**Impact**: 70% scope reduction, 60% timeline reduction

## What Changed

### Before (Original Design)
- Extension implemented FileWatcher class with custom debouncing
- Extension implemented BranchWatcher class with .git/HEAD parsing
- Extension implemented incremental update logic
- Extension implemented worktree management
- Estimated 37-52 days, ~3000 lines of code

### After (Revised Design)
- Extension spawns `crewchief-maproom watch` (file watching)
- Extension spawns `crewchief-maproom branch-watch` (branch detection)
- Extension delegates all indexing to Rust binary
- Extension delegates worktree management to CLI
- Estimated 15-25 days, ~300 lines of code

## Critical Discovery

### Rust Binary Already Has Everything

**File Watching:**
```bash
crewchief-maproom watch --repo myrepo --worktree main --path /workspace --throttle 3s
```
- Built-in file watching (Rust `notify` crate)
- Automatic incremental upserts
- Configurable debouncing (--throttle flag)
- Cross-platform, battle-tested

**Branch Watching:**
```bash
crewchief-maproom branch-watch --repo /workspace
```
- Watches `.git/HEAD` for changes
- Automatic branch detection
- Triggers incremental updates
- Completed in BRWATCH project (2025-11-09)

**CLI Integration:**
```bash
crewchief worktree create feature-auth  # Auto-indexes!
```
- Creates worktree AND runs maproom scan
- Lists all worktrees
- Automatic Maproom integration

## Architectural Principles

1. **Delegate Heavy Lifting** - Use existing Rust binary for all indexing/watching
2. **Thin Orchestration** - Extension is just a VSCode UI layer
3. **Reuse CLI Integration** - Leverage existing worktree + maproom integration
4. **Simple Process Management** - Spawn, monitor, parse stdout

## New Architecture Overview

```typescript
// The ENTIRE extension is basically this:
class MaproomExtension {
  private dockerManager: DockerManager;
  private watchProcess: ChildProcess | null = null;
  private branchWatchProcess: ChildProcess | null = null;
  private statusBar: vscode.StatusBarItem;

  async activate(context: vscode.ExtensionContext) {
    // 1. Ensure Docker services running
    await this.dockerManager.ensureServicesRunning();

    // 2. Spawn watch process (long-running)
    this.watchProcess = spawn('crewchief-maproom', [
      'watch',
      '--throttle', '3s'
    ], { cwd: vscode.workspace.rootPath });

    // 3. Spawn branch-watch process (long-running)
    this.branchWatchProcess = spawn('crewchief-maproom', [
      'branch-watch',
      '--repo', vscode.workspace.rootPath
    ]);

    // 4. Parse stdout, update status bar
    this.watchProcess.stdout.on('data', this.handleWatchOutput);
    this.branchWatchProcess.stdout.on('data', this.handleBranchWatchOutput);

    // 5. Show status
    this.statusBar.text = "$(check) Maproom Active";
  }

  async deactivate() {
    this.watchProcess?.kill();
    this.branchWatchProcess?.kill();
    await this.dockerManager.stopServices();
  }
}
```

**That's it. ~300 lines instead of ~3000 lines.**

## Components Removed

### FileWatcher Class (REMOVED)
**Before:** ~150 lines implementing custom file watching with VSCode FileSystemWatcher
**After:** Delegated to `crewchief-maproom watch` command

**Why:** Rust binary already has superior file watching using `notify` crate

### BranchWatcher Class (REMOVED)
**Before:** ~160 lines parsing .git/HEAD and detecting changes
**After:** Delegated to `crewchief-maproom branch-watch` command

**Why:** Rust binary already has this exact functionality (BRWATCH project)

### DebounceManager (REMOVED)
**Before:** ~50 lines implementing custom debouncing algorithm
**After:** Use `--throttle` flag in Rust binary

**Why:** Binary handles debouncing better (configurable, tested)

### IncrementalUpdater (REMOVED)
**Before:** ~100 lines tracking changed files and calling upsert
**After:** Rust binary handles incremental updates automatically

**Why:** Binary already tracks changes and performs upserts

### WorktreeManager (REMOVED)
**Before:** ~80 lines managing worktrees
**After:** Use `crewchief worktree` CLI commands

**Why:** CLI already has worktree management with auto-indexing

## Components Added

### ProcessOrchestrator (NEW)
**Responsibility:** Spawn and monitor long-running Rust processes
**Code:** ~80 lines
**Key Methods:**
- `spawnWatcher()` - Starts watch process
- `spawnBranchWatcher()` - Starts branch-watch process
- `handleCrash()` - Restart with exponential backoff

### StdoutParser (NEW)
**Responsibility:** Parse NDJSON from Rust binary stdout
**Code:** ~40 lines
**Format:**
```jsonl
{"type":"watching","repo":"crewchief","worktree":"main"}
{"type":"indexing","files_count":2,"current_file":"src/index.ts"}
{"type":"complete","files_processed":2,"chunks_inserted":15}
```

### StatusBarManager (NEW)
**Responsibility:** Display indexing status based on parsed events
**Code:** ~30 lines
**States:** Idle, Watching, Indexing, Error

## Benefits

### Development Speed
- ✅ 60% faster development timeline (15-25 days vs 37-52 days)
- ✅ 70% less code to write (~300 lines vs ~3000 lines)
- ✅ Only 2 agents needed instead of 3

### Code Quality
- ✅ Consistent behavior with CLI
- ✅ Better performance (Rust vs TypeScript)
- ✅ Less code = fewer bugs
- ✅ Easier to maintain

### Reliability
- ✅ Reusing battle-tested code
- ✅ Lower risk (fewer moving parts)
- ✅ Easier debugging (simpler architecture)
- ✅ Clear separation of concerns

## Trade-offs

### Coupling to Rust Binary
- ⚠️ Depends on Rust binary being stable
- ⚠️ Changes to binary stdout format require extension updates
- ⚠️ Must bundle correct binary version
- **Mitigation:** Pin to known-good binary version, define NDJSON contract

### Less Control
- ⚠️ Can't customize watching behavior directly
- ⚠️ Must use --throttle flag (not custom debouncing)
- ⚠️ Rust binary decides indexing strategy
- **Mitigation:** Existing functionality is sufficient for MVP

### Stdout Parsing Dependency
- ⚠️ Must parse stdout for progress
- ⚠️ Coupling between extension and binary output format
- ⚠️ Changes to logging could break status bar
- **Mitigation:** Use structured NDJSON, version output format

## Migration Notes

### Removed Components

**FileWatcher class → `crewchief-maproom watch`**
```typescript
// BEFORE:
const watcher = new FileWatcher(workspaceRoot, {
  debounceMs: 3000,
  onBatch: (files) => this.indexing.upsert(files)
});

// AFTER:
const watchProcess = spawn('crewchief-maproom', [
  'watch',
  '--throttle', '3s'
]);
```

**BranchWatcher class → `crewchief-maproom branch-watch`**
```typescript
// BEFORE:
const branchWatcher = new BranchWatcher(workspaceRoot, {
  onBranchChange: (branch) => this.indexing.scan(branch)
});

// AFTER:
const branchWatchProcess = spawn('crewchief-maproom', [
  'branch-watch',
  '--repo', workspaceRoot
]);
```

**WorktreeManager → `crewchief worktree` CLI**
```typescript
// BEFORE:
await this.worktreeManager.create('feature-auth');
await this.indexing.scan();

// AFTER:
// Use CLI directly:
// crewchief worktree create feature-auth
// (auto-indexes!)
```

### New Components

**ProcessOrchestrator:**
- Spawns watch + branch-watch processes
- Monitors stdout/stderr
- Handles crashes with backoff
- Kills processes on deactivation

**StdoutParser:**
- Parses NDJSON from binary
- Maps to ProcessEvent types
- Handles malformed output gracefully

**StatusBarManager:**
- Displays parsed status
- Updates in real-time
- Shows errors with actions

## Updated Success Metrics

### Timeline
- **Original:** 37-52 days (7.5-10.5 weeks)
- **Revised:** 15-25 days (3-5 weeks)
- **Reduction:** 60% faster

### Code Volume
- **Original:** ~3000 lines (estimated)
- **Revised:** ~300 lines (estimated)
- **Reduction:** 90% less code

### Test Coverage
- **Original:** 60% target
- **Revised:** 50% target (less code to test)
- **Focus:** Process spawning, stdout parsing, status display

### Agent Count
- **Original:** 3 specialized agents
- **Revised:** 2 specialized agents
- **Removed:** File/branch watcher specialists (not needed)

### Ticket Count
- **Original:** 40-60 tickets
- **Revised:** 15-20 tickets
- **Reduction:** 60% fewer tickets

## Updated Project Phases

### Phase 0: Agent Creation (2-3 days)
**Agents:**
1. process-management-specialist
2. vscode-extension-specialist

### Phase 1: Core Infrastructure (5-7 days)
**Deliverables:**
- Docker Manager
- Binary Spawner (watch + branch-watch)
- Status Bar

### Phase 2: Setup Wizard (3-4 days)
**Deliverables:**
- Provider Selection UI
- Credential Storage
- Initial Scan

### Phase 3: Process Monitoring (2-4 days)
**Deliverables:**
- Stdout Parser (NDJSON)
- Error Recovery (exponential backoff)

### Phase 4: Polish & Testing (3-5 days)
**Deliverables:**
- Integration Tests
- Manual Testing
- Documentation

**Total:** 15-25 days (was 37-52 days)

## Risk Mitigation Updates

### New Risks

**Risk: Rust Binary Stdout Changes**
- **Probability:** Low
- **Impact:** Medium (status bar breaks)
- **Mitigation:** Define NDJSON contract, version output format
- **Contingency:** Graceful degradation (show "Indexing..." without details)

**Risk: Process Doesn't Restart**
- **Probability:** Low
- **Impact:** Medium (manual restart required)
- **Mitigation:** Comprehensive crash recovery tests
- **Contingency:** Manual "Restart" command

### Removed Risks

**Risk: File Watching Edge Cases** (REMOVED)
- **Why:** Rust binary handles this, not extension

**Risk: Branch Detection Bugs** (REMOVED)
- **Why:** Rust binary handles this, not extension

**Risk: Debouncing Implementation** (REMOVED)
- **Why:** Rust binary handles this via --throttle

## Documentation Updates Required

### README.md
- Update scope to "thin orchestration layer"
- Timeline 3-5 weeks (not 7.5-10.5)
- ~300 lines of code (not ~3000)

### architecture.md
- Remove FileWatcher/BranchWatcher sections
- Add ProcessOrchestrator section
- Add StdoutParser section
- Emphasize delegation to Rust binary

### plan.md
- Update phases (4 instead of 5)
- Update timeline (15-25 days)
- Update ticket count (15-20 instead of 40-60)
- Update agent count (2 instead of 3)

### quality-strategy.md
- Lower coverage target (50% instead of 60%)
- Remove file/branch watching tests
- Add process spawning tests
- Add stdout parsing tests

### agent-suggestions.md
- Remove file/branch watcher specialists
- Keep only 2 agents:
  - process-management-specialist
  - vscode-extension-specialist

## Future: Direct Integration (Phase 10+)

If we need more control in the future, we could:
- Link directly to Rust library (N-API)
- Implement custom watching in TypeScript
- Build custom UI for index management

**Current approach (spawn processes) is sufficient for MVP through Phase 9.**

## Conclusion

**Architectural Changes Summary:**

1. **Removed 5 major classes** (FileWatcher, BranchWatcher, DebounceManager, IncrementalUpdater, WorktreeManager)
2. **Added 3 simple classes** (ProcessOrchestrator, StdoutParser, StatusBarManager)
3. **70% scope reduction** (~3000 lines → ~300 lines)
4. **60% timeline reduction** (37-52 days → 15-25 days)
5. **Lower risk** (reusing battle-tested components)

**Key Insight:** Extension is NOT implementing indexing - it's orchestrating existing tools.

**Decision:** Ship MVP with thin orchestration approach. Revisit direct integration only if user feedback requires it.

**Approval Date:** _____________
**Approved By:** _____________
