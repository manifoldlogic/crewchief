# VSMAP: VSCode Maproom Extension

**Status**: ✅ Ready for Implementation (Revised Architecture)
**Timeline**: 3-5 weeks (15-25 days)
**Complexity**: Low (thin orchestration layer)
**Last Updated**: 2025-11-16

## Problem Statement

Developers using Maproom semantic search must manually:
1. Start Docker services (PostgreSQL + MCP server)
2. Run `crewchief-maproom scan` after code changes
3. Remember to re-index when switching branches
4. Monitor indexing status manually

**This is friction.** Every context switch requires manual indexing steps.

## Solution

A **lightweight VSCode extension** that automates Maproom infrastructure by orchestrating existing tools.

### What This Extension Does

**Four simple responsibilities:**

1. **Manages Docker Services**
   - Starts PostgreSQL + Maproom MCP server on activation
   - Stops services on deactivation
   - Health check monitoring

2. **Spawns Watch Processes**
   - Launches `crewchief-maproom watch` (file watching)
   - Launches `crewchief-maproom branch-watch` (branch detection)
   - Both run as long-lived background processes

3. **Displays Status**
   - Parses stdout from watch processes
   - Updates status bar: "$(eye) Watching...", "$(sync~spin) Indexing 15 files..."
   - Click for detailed status

4. **Guides Setup**
   - First-run wizard for provider configuration
   - Securely stores credentials (SecretStorage)
   - Triggers initial scan

### What This Extension DOESN'T Do

The extension is a **thin orchestration layer**. All heavy lifting is delegated:

**To Rust Binary (`crewchief-maproom`):**
- ✅ File watching and debouncing
- ✅ Branch detection and .git/HEAD monitoring
- ✅ Incremental indexing and deduplication
- ✅ Embedding generation
- ✅ Cross-platform compatibility

**To TypeScript CLI (`crewchief`):**
- ✅ Worktree creation and management
- ✅ Automatic indexing on worktree creation

**Out of Scope:**
- ❌ Search UI (use MCP via Claude Code)
- ❌ Custom configuration UI (use VSCode settings)
- ❌ Marketplace publishing (Phase 5, post-MVP)

## Architecture

```
┌─────────────────────────────────────────────────┐
│         VSCode Extension (~300 lines)          │
├─────────────────────────────────────────────────┤
│                                                 │
│  1. DockerManager                               │
│     └─> docker-compose up/down                  │
│                                                 │
│  2. ProcessOrchestrator                         │
│     ├─> spawn('crewchief-maproom watch')       │
│     └─> spawn('crewchief-maproom branch-watch')│
│                                                 │
│  3. StatusBarManager                            │
│     └─> Parse stdout → Update UI                │
│                                                 │
│  4. SetupWizard                                 │
│     └─> Provider config + initial scan          │
│                                                 │
└─────────────────────────────────────────────────┘
           ↓ spawns                    ↓ spawns
    ┌──────────────┐          ┌──────────────────┐
    │ watch process│          │ branch-watch     │
    │ (Rust binary)│          │ process          │
    │              │          │ (Rust binary)    │
    │ - File watch │          │ - .git/HEAD watch│
    │ - Debouncing │          │ - Branch detect  │
    │ - Upserts    │          │ - Auto-index     │
    └──────────────┘          └──────────────────┘
```

## Success Criteria

| Metric | Target | Measured By |
|--------|--------|-------------|
| **Activation Time** | <500ms | performance.mark() in activate() |
| **Memory Usage** | <50MB idle | process.memoryUsage() |
| **Test Coverage** | >50% | c8 coverage report |
| **Docker Startup** | <30s | Health check completion time |
| **Status Updates** | <1s latency | Stdout parse to UI update |
| **Process Reliability** | 99% uptime | Crash recovery success rate |

## Timeline

**Total: 15-25 days (3-5 weeks)**

- Phase 0: Agent Creation (2-3 days)
- Phase 1: Core Infrastructure (5-7 days)
- Phase 2: Setup Wizard (3-4 days)
- Phase 3: Process Monitoring (2-4 days)
- Phase 4: Polish & Testing (3-5 days)

**60% faster than original estimate** (was 37-52 days)

## Scope

### In Scope (MVP)

**Extension Implementation:**
- Docker lifecycle management (docker-compose)
- Process spawning and monitoring
- Stdout parsing (NDJSON format)
- Status bar integration
- Setup wizard (provider selection, credentials)
- Error recovery (exponential backoff)
- VSIX packaging

**Platform Support:**
- Linux (x64, arm64)
- macOS (x64, arm64)
- Windows (x64)
- Devcontainers (DinD, DooD modes)

### Out of Scope

**Delegated to Existing Tools:**
- File/branch watching → Rust binary
- Indexing logic → Rust binary
- Worktree management → CLI
- Search UI → MCP

**Deferred to Post-MVP:**
- Marketplace publishing (Phase 5)
- Multi-workspace support (Phase 6)
- Custom search UI (Phase 8)
- Advanced configuration UI (Phase 8)

## Dependencies

**Runtime Requirements:**
- Docker Desktop (for PostgreSQL + MCP server)
- `crewchief-maproom` binary (bundled with extension)
- VSCode 1.85+

**Development Dependencies:**
- TypeScript 5.x
- Vitest (testing)
- @vscode/test-electron (E2E testing)
- Docker Compose

## Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| Rust binary stdout changes | Medium | Define NDJSON contract, version output |
| Process crashes | Low | Exponential backoff, auto-restart |
| Docker unavailable | Medium | Clear error message, setup guide |
| Platform-specific issues | Low | Test on all platforms, bundle binaries |
| Devcontainer compatibility | Medium | Test DinD and DooD modes |

**Overall Risk:** LOW (reusing battle-tested components)

## Why This Approach?

### Benefits
- ✅ **60% faster development** (15-25 days vs 37-52 days)
- ✅ **90% less code** (~300 lines vs ~3000 lines)
- ✅ **Consistent behavior** with CLI (same Rust binary)
- ✅ **Better performance** (Rust vs TypeScript for watching)
- ✅ **Lower risk** (reusing proven infrastructure)
- ✅ **Easier maintenance** (fewer moving parts)

### Trade-offs
- ⚠️ Coupled to Rust binary stdout format
- ⚠️ Less control over watching behavior
- ⚠️ Depends on binary stability

**The trade-offs are acceptable for MVP.** We can add direct integration later if needed (Phase 10+).

## Next Steps

1. ✅ Planning complete (this document)
2. **Run `/create-project-tickets VSMAP`** to generate implementation tickets
3. **Run `/work-on-project VSMAP`** to execute tickets
4. Ship VSIX in 3-5 weeks

## Related Documentation

- `planning/architecture.md` - Detailed component specifications
- `planning/plan.md` - Implementation phases and tickets
- `planning/architecture-revision.md` - Why we revised the architecture
- `planning/quality-strategy.md` - Testing approach
- `planning/security-review.md` - Security considerations
