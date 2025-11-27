# Implementation Plan: VSCode Maproom Extension

## Executive Summary

**Project:** VSCode extension for automatic semantic code indexing
**Approach:** Thin orchestration layer over existing Rust binary and CLI
**Timeline:** 15-25 days (3-5 weeks)
**Complexity:** Low (delegate to existing infrastructure)
**Risk:** Low (reusing battle-tested components)

### Scope Reduction

**Before (Original Plan):** 37-52 days, ~3000 lines of custom implementation
**After (Revised Plan):** 15-25 days, ~300 lines of orchestration code

**Why the Change:**
- Rust binary already has `watch` and `branch-watch` commands
- CLI already has worktree management with auto-indexing
- Extension should orchestrate, not reimplement

## Implementation Phases

### Phase 0: Agent Creation (2-3 days)

**Agents needed (REDUCED from 3 to 2):**

1. **process-management-specialist** - Spawn/monitor Rust processes
   - Child process spawning (Node.js `spawn`)
   - Stdout/stderr parsing (NDJSON format)
   - Process lifecycle (start/stop/restart)
   - Crash recovery with exponential backoff
   - Platform-specific binary selection

2. **vscode-extension-specialist** - Extension activation, status bar, UI
   - Extension activation (<500ms requirement)
   - StatusBarItem updates
   - SecretStorage for credentials
   - QuickPick UI for setup wizard
   - Extension packaging (VSIX)

**Removed:** No need for separate watcher specialists (Rust handles it)

**Deliverables:**
- `.claude/agents/specialized/process-management-specialist.md`
- `.claude/agents/specialized/vscode-extension-specialist.md`

**Acceptance Criteria:**
- Agents can be invoked via CLI
- Agent definitions include training examples
- Agents have clear scope and responsibilities

---

### Phase 1: Core Infrastructure (5-7 days)

**Objective:** Get extension activating and spawning watch processes

#### Milestone 1.1: Docker Manager (2 days)

**Tasks:**
- Copy docker-compose.yml for postgres + maproom-mcp
- Implement `DockerManager` class
  - Start/stop services via `docker compose` CLI
  - Health check monitoring (pg_isready, TCP ping)
  - Error handling with user-friendly messages

**Agent:** process-management-specialist

**Files Created:**
- `src/docker/manager.ts`
- `config/docker-compose.yml`

**Tests:**
- Docker services start successfully
- Health checks timeout gracefully
- Services already running (no-op)

**Acceptance Criteria:**
- `DockerManager.ensureServicesRunning()` works
- PostgreSQL healthy within 30s
- Error shown if Docker not running

---

#### Milestone 1.2: Binary Spawner (2 days)

**Tasks:**
- Implement platform detection
- Spawn `crewchief-maproom watch` process
- Spawn `crewchief-maproom branch-watch` process
- Basic stdout parsing (log to console for now)

**Agent:** process-management-specialist

**Files Created:**
- `src/process/spawner.ts`
- `src/process/parser.ts`
- `src/utils/platform.ts`

**Tests:**
- Correct binary selected for current platform
- Process spawns successfully
- Stdout parsed to NDJSON events
- Process kills on extension deactivation

**Acceptance Criteria:**
- Both watch processes spawn and run
- Stdout logged to VSCode Output panel
- Processes die gracefully on deactivation

---

#### Milestone 1.3: Status Bar (1 day)

**Tasks:**
- Create StatusBarItem
- Wire to process output
- Show watching/indexing/error states

**Agent:** vscode-extension-specialist

**Files Created:**
- `src/ui/statusBar.ts`

**Tests:**
- Status bar shows correct text for each state
- Tooltip updates with details

**Acceptance Criteria:**
- Status bar visible after activation
- Status bar updates when processes start
- Clicking status bar shows output panel

---

**Phase 1 Acceptance Criteria:**
- ✅ Extension activates in <500ms
- ✅ Docker services start successfully
- ✅ Both watch processes spawn and run
- ✅ Status bar shows "Watching..."
- ✅ Processes kill on deactivation

---

### Phase 2: Setup Wizard (3-4 days)

**Objective:** Guide users through first-time configuration

#### Milestone 2.1: Provider Selection UI (1 day)

**Tasks:**
- QuickPick for provider selection (Ollama/OpenAI/Google)
- Detect running Ollama instance
- Show recommendations based on availability

**Agent:** vscode-extension-specialist

**Files Created:**
- `src/ui/setupWizard.ts`

**Tests:**
- Wizard shown on first activation
- Provider selection works
- Ollama detection accurate

**Acceptance Criteria:**
- QuickPick displays 3 providers
- "Recommended" badge for Ollama if running
- Selection persisted to settings

---

#### Milestone 2.2: Credential Storage (1 day)

**Tasks:**
- Integrate VSCode SecretStorage API
- Store OpenAI API key securely
- Store Google credentials (if selected)
- Environment variable setup for binary

**Agent:** vscode-extension-specialist

**Files Created:**
- `src/config/secrets.ts`

**Tests:**
- Credentials stored in SecretStorage
- Credentials retrieved for binary spawn
- Credentials never logged

**Acceptance Criteria:**
- API keys stored securely
- Environment variables passed to binary
- No credentials in VSCode logs

---

#### Milestone 2.3: Initial Scan (1 day)

**Tasks:**
- Trigger `crewchief-maproom scan` on setup complete
- Parse progress output (% complete, files scanned)
- Show progress notification

**Agent:** process-management-specialist

**Files Created:**
- `src/process/scan.ts`

**Tests:**
- Scan spawns successfully
- Progress parsed correctly
- Completion detected

**Acceptance Criteria:**
- Scan triggered after wizard complete
- Notification shows progress (0-100%)
- Status bar updates to "Indexed" on complete

---

**Phase 2 Acceptance Criteria:**
- ✅ Wizard runs on first activation
- ✅ Credentials stored securely
- ✅ Initial scan completes
- ✅ User can select provider
- ✅ Setup can be re-run via command

---

### Phase 3: Process Monitoring (2-4 days)

**Objective:** Robust process management with error recovery

#### Milestone 3.1: Stdout Parser (2 days)

**Tasks:**
- Parse NDJSON from watch processes
- Extract progress info (files indexed, errors)
- Update status bar in real-time

**Agent:** process-management-specialist

**Files Enhanced:**
- `src/process/parser.ts` (expand beyond basic logging)
- `src/ui/statusBar.ts` (wire to parsed events)

**Tests:**
- All NDJSON event types parsed
- Status bar updates correctly
- File counts accurate

**Acceptance Criteria:**
- Status bar shows file counts during indexing
- Errors displayed with actionable messages
- "Last indexed" timestamp shown

---

#### Milestone 3.2: Error Handling (1 day)

**Tasks:**
- Detect process crashes
- Restart with exponential backoff
- Show user-friendly error messages
- Offer "Show Logs" action

**Agent:** process-management-specialist

**Files Created:**
- `src/process/recovery.ts`

**Tests:**
- Process crash triggers restart
- Backoff increases exponentially
- Max restarts enforced (5 attempts)

**Acceptance Criteria:**
- Crashed process restarts automatically
- User notified on persistent failures
- Logs accessible via "Show Logs" button

---

**Phase 3 Acceptance Criteria:**
- ✅ Status bar updates show file counts
- ✅ Process crashes trigger restart
- ✅ Errors displayed with actionable messages
- ✅ Memory usage <50MB idle

---

### Phase 4: Polish & Testing (3-5 days)

**Objective:** Production-ready quality

#### Milestone 4.1: Integration Testing (2 days)

**Tasks:**
- Docker startup tests
- Process spawning tests
- Error recovery tests

**Agent:** ALL (collaborative)

**Files Created:**
- `src/test/integration/docker.test.ts`
- `src/test/integration/process.test.ts`
- `src/test/integration/recovery.test.ts`

**Tests:**
- All integration tests pass
- Coverage >50% for critical paths

**Acceptance Criteria:**
- Integration tests run in CI
- Tests pass on Linux (devcontainer)
- Docker cleanup after tests

---

#### Milestone 4.2: Manual Testing (1 day)

**Tasks:**
- Test on Linux (devcontainer)
- Test on macOS (if available)
- Test on Windows (optional for MVP)
- Verify watch processes work
- Test all error scenarios

**Agent:** ALL

**Deliverables:**
- Manual testing checklist (checked off)
- Bug fixes for critical issues

**Acceptance Criteria:**
- Manual testing checklist complete
- Critical bugs fixed
- Works in devcontainer

---

#### Milestone 4.3: Documentation (1 day)

**Tasks:**
- README with installation instructions
- Troubleshooting guide
- VSIX packaging
- Release notes

**Agent:** vscode-extension-specialist

**Files Created:**
- `packages/vscode-maproom/README.md`
- `packages/vscode-maproom/TROUBLESHOOTING.md`
- `packages/vscode-maproom/CHANGELOG.md`

**Acceptance Criteria:**
- README explains installation
- Troubleshooting covers common issues
- VSIX builds successfully

---

**Phase 4 Acceptance Criteria:**
- ✅ 50% test coverage (critical paths)
- ✅ Manual testing passes on all platforms
- ✅ VSIX installs and activates correctly
- ✅ Documentation complete

---

## Total Timeline: 15-25 days (3-5 weeks)

**Breakdown:**
- Phase 0: 2-3 days (agent creation)
- Phase 1: 5-7 days (core infrastructure)
- Phase 2: 3-4 days (setup wizard)
- Phase 3: 2-4 days (process monitoring)
- Phase 4: 3-5 days (polish & testing)
- Buffer: ~20% built into ranges

**Comparison:**
- Original plan: 37-52 days (7.5-10.5 weeks)
- Revised plan: 15-25 days (3-5 weeks)
- **Reduction: 60% faster**

---

## Success Criteria

### Functional Requirements

**MVP Must-Have:**
- ✅ Extension activates in <500ms
- ✅ Docker services start automatically
- ✅ `watch` process spawns and runs continuously
- ✅ `branch-watch` process spawns and runs continuously
- ✅ Status bar shows real-time indexing status
- ✅ Setup wizard guides first-time configuration
- ✅ Ollama and OpenAI providers supported
- ✅ Credentials stored securely (SecretStorage)
- ✅ Process crashes recover automatically

**MVP Nice-to-Have (Defer if Time-Constrained):**
- ⚠️ Google provider fully tested
- ⚠️ Windows testing (document as experimental)
- ⚠️ Custom throttle configuration

### Non-Functional Requirements

**Performance:**
- Activation time <500ms
- Memory usage <50MB idle, <200MB indexing
- CPU usage <5% idle

**Quality:**
- Test coverage >50%
- No critical bugs
- Works in devcontainer

**Documentation:**
- README explains installation
- Troubleshooting guide exists
- VSIX packaging documented

---

## Risk Assessment

### High Risks (Monitor Closely)

**Risk 1: Rust Binary Stability**
- **Probability:** Low
- **Impact:** High (blocks all functionality)
- **Mitigation:** Binary already battle-tested, use known-good version
- **Contingency:** Pin to specific binary version

**Risk 2: Stdout Parsing Breaks**
- **Probability:** Medium
- **Impact:** Medium (status bar doesn't update)
- **Mitigation:** Define NDJSON contract upfront, test thoroughly
- **Contingency:** Graceful degradation (show "Indexing..." without details)

**Risk 3: Process Doesn't Restart**
- **Probability:** Low
- **Impact:** Medium (manual restart required)
- **Mitigation:** Comprehensive crash recovery tests
- **Contingency:** Manual "Restart" command

### Medium Risks (Accept for MVP)

**Risk 4: Platform-Specific Issues**
- **Probability:** Medium
- **Impact:** Low (affects subset of users)
- **Mitigation:** Test on Linux/macOS, document Windows as experimental
- **Contingency:** Platform-specific bug fixes post-MVP

**Risk 5: Docker Not Installed**
- **Probability:** High (some users)
- **Impact:** Low (clear error message)
- **Mitigation:** Detect Docker, show helpful error
- **Contingency:** Documentation for Docker installation

### Low Risks (Acceptable)

**Risk 6: Extension Activation Slow**
- **Probability:** Low
- **Impact:** Low (slightly annoying)
- **Mitigation:** Lazy-load heavy modules
- **Contingency:** Document as known issue

---

## Out of Scope (Post-MVP)

**Explicitly NOT in MVP:**
- ❌ Search UI in extension (use MCP)
- ❌ Custom embedding models
- ❌ Multi-workspace support
- ❌ Index statistics dashboard
- ❌ Marketplace publishing
- ❌ Advanced configuration UI
- ❌ Audit logging
- ❌ Enterprise features

**Why Deferred:**
- MCP search works well (no UI needed)
- Fixed models sufficient for MVP
- Single workspace covers majority use case
- Statistics nice-to-have, not critical
- Marketplace requires publisher account setup
- Settings JSON sufficient for MVP
- Logging for enterprise, not MVP users

See `post-mvp-roadmap.md` for full roadmap.

---

## Dependencies

### External Dependencies

**Required:**
- Docker Desktop (or Docker daemon)
- VSCode 1.85+
- Node.js 18+ (for development)

**Bundled:**
- `crewchief-maproom` binary (all platforms)
- PostgreSQL (via Docker)
- Ollama (via Docker, optional)
- Maproom MCP server (via Docker)

### Internal Dependencies

**From Other CrewChief Projects:**
- Rust binary (`crewchief-maproom`) - COMPLETED
- Branch-watch command (`branch-watch`) - COMPLETED (BRWATCH project)
- Docker Compose config - EXISTS
- Database schema - EXISTS

**No Blockers:** All dependencies already implemented!

---

## Agent Workflow

### Sequential Workflow

**Phase 1:**
1. `process-management-specialist` implements DockerManager
2. `process-management-specialist` implements BinarySpawner
3. `vscode-extension-specialist` implements StatusBar

**Phase 2:**
1. `vscode-extension-specialist` implements SetupWizard
2. `vscode-extension-specialist` implements SecretStorage
3. `process-management-specialist` implements initial scan

**Phase 3:**
1. `process-management-specialist` enhances StdoutParser
2. `process-management-specialist` implements error recovery

**Phase 4:**
1. Both agents write integration tests
2. Both agents perform manual testing
3. `vscode-extension-specialist` writes documentation

### Parallel Opportunities

**Can Run in Parallel:**
- Phase 1.2 (Binary Spawner) + Phase 1.3 (Status Bar) - different agents
- Phase 2.1 (Provider Selection) + Phase 2.2 (Secrets) - same agent, can interleave
- Phase 4.1 (Tests) - both agents write tests simultaneously

---

## Ticket Creation Strategy

**Ticket Naming:** `VSMAP-{number}_{description}.md`

**Estimated Ticket Count:** 15-20 tickets

**Example Tickets:**
- `VSMAP-1001_agent-creation.md` - Create 2 specialized agents
- `VSMAP-1002_docker-manager.md` - Implement Docker orchestration
- `VSMAP-1003_binary-spawner.md` - Spawn watch processes
- `VSMAP-1004_status-bar.md` - StatusBarItem integration
- `VSMAP-1005_setup-wizard.md` - First-run wizard
- `VSMAP-1006_secret-storage.md` - SecretStorage integration
- `VSMAP-1007_initial-scan.md` - Trigger scan on setup
- `VSMAP-1008_stdout-parser.md` - Parse NDJSON from binary
- `VSMAP-1009_error-recovery.md` - Process crash recovery
- `VSMAP-1010_integration-tests.md` - Docker + process tests
- `VSMAP-1011_manual-testing.md` - Cross-platform testing
- `VSMAP-1012_documentation.md` - README + troubleshooting
- `VSMAP-1013_vsix-packaging.md` - VSIX build and release

**Ticket Assignment:**
- Phases 1-3: Mostly `process-management-specialist`
- Phase 2: Mostly `vscode-extension-specialist`
- Phase 3: Mostly `process-management-specialist`
- Phase 4: Both agents

---

## Daily Progress Tracking

**Daily Standup Questions:**
1. What did I complete yesterday?
2. What am I working on today?
3. Any blockers?

**Progress Metrics:**
- Tickets completed
- Tests passing
- Coverage %
- Manual testing checklist items

**Weekly Goals:**
- Week 1: Phase 0 + Phase 1 complete
- Week 2: Phase 2 complete
- Week 3: Phase 3 complete
- Week 4: Phase 4 complete (buffer week)

---

## Conclusion

**Plan Summary:**
- **Duration:** 15-25 days (3-5 weeks)
- **Agents:** 2 specialized agents
- **Tickets:** ~15-20 tickets
- **Risk:** Low (reusing existing infrastructure)
- **Complexity:** Low (thin orchestration layer)

**Key Success Factors:**
1. Delegate to existing Rust binary and CLI
2. Keep extension code minimal (~300 lines)
3. Focus on process orchestration, not implementation
4. Leverage battle-tested components
5. Ship fast, iterate based on feedback

**Next Steps:**
1. Review and approve this plan
2. Create specialized agents (Phase 0)
3. Generate tickets via `/create-project-tickets VSMAP`
4. Execute sequentially via `/work-on-project VSMAP`
