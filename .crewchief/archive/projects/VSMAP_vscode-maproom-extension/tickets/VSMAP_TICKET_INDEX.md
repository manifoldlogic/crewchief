# VSMAP Ticket Index

**Project:** VSCode Maproom Extension
**Total Tickets:** 18
**Estimated Duration:** 20-30 days (15-25 days with concurrency)
**Last Updated:** 2025-11-16

## Project Overview

Thin orchestration layer VSCode extension that spawns `crewchief-maproom` Rust binary processes for automatic semantic code indexing. Delegates file watching, branch detection, and indexing to existing infrastructure.

## Ticket Summary

| Phase | Tickets | Estimated Days | Status |
|-------|---------|---------------|--------|
| Phase 0: Agent Creation | 3 | 2-3 | ⏳ Not Started |
| Phase 1: Core Infrastructure | 6 | 5-7 | ⏳ Not Started |
| Phase 2: Setup Wizard | 3 | 3-4 | ⏳ Not Started |
| Phase 3: Process Monitoring | 2 | 2-4 | ⏳ Not Started |
| Phase 4: Polish & Testing | 4 | 3-5 | ⏳ Not Started |
| **Total** | **18** | **15-23** | |

## Phase 0: Agent Creation (2-3 days)

**Objective:** Create 2 specialized agents for implementation

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-0001 | Create process-management-specialist | general-purpose | 1d | ⏳ | None |
| VSMAP-0002 | Create vscode-extension-specialist | general-purpose | 1d | ⏳ | None |
| VSMAP-0003 | Test agents with simple extension | vscode-ext + process-mgmt | 0.5d | ⏳ | 0001, 0002 |

**Deliverables:**
- `.claude/agents/specialized/process-management-specialist.md`
- `.claude/agents/specialized/vscode-extension-specialist.md`
- Test extension validates both agents work

**Phase Acceptance Criteria:**
- ✅ Both agents can be invoked via Task tool
- ✅ Agents include training examples
- ✅ Test extension works without human intervention

---

## Phase 1: Core Infrastructure (5-7 days)

**Objective:** Extension activates and spawns watch processes

### Milestone 1.1: Docker Manager (2 days)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-1001 | Implement DockerManager class | process-mgmt | 2d | ⏳ | 0003 |
| VSMAP-1002 | Add Docker Manager tests | process-mgmt | 0.5d | ⏳ | 1001 |

### Milestone 1.2: Binary Spawner (2 days)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-1003 | Implement binary spawner (ProcessOrchestrator) | process-mgmt | 2d | ⏳ | 0003, 1001 |
| VSMAP-1004 | Implement NDJSON stdout parser | process-mgmt | 1d | ⏳ | 1003 |

### Milestone 1.3: Status Bar (1 day)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-1005 | Implement StatusBarManager | vscode-ext | 1d | ⏳ | 1004 |
| VSMAP-1006 | Wire extension activation | vscode-ext | 0.5d | ⏳ | 1001, 1003, 1005 |

**Deliverables:**
- Docker services start automatically
- Watch processes spawn and run
- Status bar shows "Watching..."
- Extension activates in <500ms

**Phase Acceptance Criteria:**
- ✅ Extension activates in <500ms
- ✅ Docker services start successfully
- ✅ Both watch processes spawn and run
- ✅ Status bar shows "Watching..."
- ✅ Processes kill cleanly on deactivation

---

## Phase 2: Setup Wizard (3-4 days)

**Objective:** First-run configuration for users

### Milestone 2.1: Provider Selection (1 day)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-2001 | Implement provider selection QuickPick | vscode-ext | 1d | ⏳ | 1006 |

### Milestone 2.2: Credential Storage (1 day)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-2002 | Implement SecretStorage for API keys | vscode-ext | 1d | ⏳ | 2001 |

### Milestone 2.3: Initial Scan (1 day)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-2003 | Implement initial scan with progress | process-mgmt | 1d | ⏳ | 2002 |

**Deliverables:**
- Setup wizard for provider selection
- Secure credential storage (SecretStorage)
- Initial workspace scan with progress

**Phase Acceptance Criteria:**
- ✅ Wizard runs on first activation
- ✅ Credentials stored securely
- ✅ Initial scan completes successfully
- ✅ User can select Ollama/OpenAI/Google
- ✅ Setup re-runnable via command

---

## Phase 3: Process Monitoring (2-4 days)

**Objective:** Robust process management with error recovery

### Milestone 3.1: Enhanced Parsing (2 days)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-3001 | Enhance stdout parser with detailed events | process-mgmt | 2d | ⏳ | 1004, 1005 |

### Milestone 3.2: Crash Recovery (1 day)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-3002 | Implement crash recovery with backoff | process-mgmt | 1d | ⏳ | 1003 |

**Deliverables:**
- Detailed status bar updates (file counts, timestamps)
- Automatic process restart on crashes
- Circuit breaker after 5 failures
- User-friendly error messages

**Phase Acceptance Criteria:**
- ✅ Status bar shows file counts during indexing
- ✅ Process crashes trigger automatic restart
- ✅ Exponential backoff implemented (1s, 2s, 4s, 8s, 16s)
- ✅ Errors displayed with actionable messages
- ✅ Memory usage <50MB idle

---

## Phase 4: Polish & Testing (3-5 days)

**Objective:** Production-ready quality

### Milestone 4.1: Testing (3 days)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-4001 | Add integration tests | process-mgmt | 2d | ⏳ | All Phase 1-3 |
| VSMAP-4002 | Manual testing protocol | general-purpose | 1d | ⏳ | 4001 |

### Milestone 4.2: Documentation & Packaging (2 days)

| Ticket ID | Title | Agent | Est. | Status | Dependencies |
|-----------|-------|-------|------|--------|--------------|
| VSMAP-4003 | Create documentation | general-purpose | 1d | ⏳ | None (parallel) |
| VSMAP-4004 | Package VSIX for distribution | vscode-ext | 1d | ⏳ | 4002 |

**Deliverables:**
- Integration test suite (>50% coverage)
- Manual testing on 3 platforms
- README and troubleshooting docs
- VSIX package ready for distribution

**Phase Acceptance Criteria:**
- ✅ Test coverage >50%
- ✅ Manual tests pass on Linux, macOS, Windows
- ✅ DevContainer modes tested (DinD, DooD)
- ✅ Documentation complete
- ✅ VSIX <50MB, installs successfully

---

## Execution Guidance

### Recommended Execution Order

**Sequential (by dependencies):**
1. Phase 0 (all 3 tickets sequentially)
2. Phase 1, Milestone 1.1 (Docker Manager)
3. Phase 1, Milestone 1.2 & 1.3 (parallel after 1001)
4. Phase 2 (all 3 tickets sequentially)
5. Phase 3 (both tickets can run in parallel)
6. Phase 4 (4001→4002→4004, 4003 can run early in parallel)

**Parallel Opportunities:**
- Phase 0: 0001 and 0002 can run in parallel (both create agents)
- Phase 1: After 1001, tickets 1003-1006 can partially overlap
- Phase 3: 3001 and 3002 can run in parallel (different files)
- Phase 4: 4003 (docs) can start early, parallel to Phase 3

### Agent Workload Distribution

| Agent | Ticket Count | Estimated Days |
|-------|--------------|---------------|
| general-purpose | 5 | 4.5 |
| process-management-specialist | 9 | 11.5 |
| vscode-extension-specialist | 4 | 4.5 |

### Critical Path

1. VSMAP-0001, 0002, 0003 (agents)
2. VSMAP-1001 (DockerManager)
3. VSMAP-1003 (ProcessOrchestrator)
4. VSMAP-1004 (StdoutParser)
5. VSMAP-1005, 1006 (Status + Activation)
6. VSMAP-2001, 2002, 2003 (Setup Wizard)
7. VSMAP-3001, 3002 (Monitoring)
8. VSMAP-4001, 4002, 4004 (Testing + Package)

**Critical path duration:** ~18-20 days (with some parallelization)

---

## References

- **Project Plan:** `.crewchief/projects/VSMAP_vscode-maproom-extension/planning/plan.md`
- **Architecture:** `.crewchief/projects/VSMAP_vscode-maproom-extension/planning/architecture.md`
- **Quality Strategy:** `.crewchief/projects/VSMAP_vscode-maproom-extension/planning/quality-strategy.md`
- **Project Review:** `.crewchief/projects/VSMAP_vscode-maproom-extension/planning/project-review.md`

---

## Notes

### Architecture Highlights
- **Thin orchestration layer** (~300 lines) - NOT reimplementing file watching
- **Delegates to Rust binary** - Uses existing `crewchief-maproom watch` and `branch-watch`
- **Process spawning approach** - No direct function calls to Rust code
- **NDJSON parsing** - Structured events from stdout

### Key Principles
- ✅ **Reuse existing infrastructure** (Rust binary, CLI, Docker)
- ✅ **Proper separation** (process spawning maintains boundaries)
- ✅ **MVP discipline** (ship value, not ceremonies)
- ✅ **Pragmatic testing** (50% coverage, critical paths only)

### Success Metrics
- Extension activates in <500ms
- Memory usage <50MB idle
- Test coverage >50%
- Works on Linux, macOS, Windows
- Works in devcontainers

---

**Last Updated:** 2025-11-16
**Status:** ⏳ Ready for Execution
**Next Command:** `/work-on-project VSMAP` or `/single-ticket VSMAP-0001`
