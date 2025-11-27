# Project Review Updates

**Original Review Date:** 2025-11-27
**Updates Completed:** 2025-11-27
**Update Status:** Complete ✅

## Critical Issues Addressed

### Issue 1: Startup Reconciliation Can Be Done in TypeScript

**Original Problem:** Plan assumed Rust changes needed for startup reconciliation, but TypeScript can run `git diff` + `upsert` before watch.

**Changes Made:**
- architecture.md: Moved reconciliation to TypeScript layer using existing `upsert` command
- plan.md: Removed Phase 1 (Rust changes), added reconciliation to ProcessOrchestrator refactor
- Verified `upsert` CLI: `crewchief-maproom upsert --commit <COMMIT> --repo <REPO> --worktree <WORKTREE> --root <ROOT> --paths <PATHS>`

**Result:** Issue resolved - Reconciliation handled at orchestration layer without Rust changes

### Issue 2: Missing `branch_switched` Event Type

**Original Problem:** `events.ts` didn't include `branch_switched` event that unified watch emits

**Changes Made:**
- plan.md: Added ticket for `BranchSwitchedEvent` interface
- quality-strategy.md: Added test for branch_switched event parsing
- architecture.md: Documented branch_switched event schema

**Result:** Issue resolved - New ticket ensures event type is added before watch integration

## Reinvention Fixed

### ProcessOrchestrator Refactor (Not Replace)

**Original Problem:** Plan proposed new `WatchProcessManager` class duplicating existing infrastructure

**Changes Made:**
- architecture.md: Changed from "new WatchProcessManager" to "refactor ProcessOrchestrator"
- plan.md: Updated Phase 2 to refactor existing code
- Documented reuse of: StdoutParser, CrashRecovery, platform detection

**Result:** Leverages existing infrastructure (~4-8 hours saved)

### Ollama Detection Reuse

**Original Problem:** Plan proposed new OllamaClient ignoring existing `detectOllama()`

**Changes Made:**
- architecture.md: OllamaClient extends existing detection
- plan.md: Ticket specifies extending setupWizard.ts code

**Result:** Builds on existing code instead of replacing

## High-Risk Mitigations Implemented

### Risk 1: Watch CLI Flags

**Mitigation Applied:**
- Verified CLI: `crewchief-maproom watch --path <PATH> [--repo <REPO>] [--throttle <THROTTLE>]`
- `--worktree` is deprecated (auto-detected from branch)
- Documented exact invocation in architecture.md

**Risk Level:** Reduced from Medium to Low

### Risk 2: SQLite Path Format

**Mitigation Applied:**
- Verified format: `sqlite://` prefix, `file:` prefix, or bare path all work
- Documented in architecture.md: Use `MAPROOM_DATABASE_URL=sqlite:///path/to/db`

**Risk Level:** Reduced from Medium to Low

### Risk 3: OpenAI/Google Providers

**Mitigation Applied:**
- Documented in architecture.md: Skip model pull for non-Ollama providers
- Added explicit condition in activation flow

**Risk Level:** Confirmed Low

## Gaps Filled

### Requirements Gaps
- ✅ Reconciliation approach → TypeScript `git diff` + `upsert` (no new event type needed)
- ✅ First-run behavior → Preserve existing `showNoSqliteGuidance()`
- ✅ Multi-workspace → Each workspace spawns own watch process

### Technical Gaps
- ✅ Watch CLI args → Documented: `--path`, `--repo` (optional), `--throttle` (optional)
- ✅ `branch_switched` schema → Added to plan as prerequisite ticket
- ✅ SQLite URL format → Documented: `sqlite:///path/to/db` or bare path

### Process Gaps
- ✅ Rollback plan → Document extension version pinning
- ✅ Migration testing → Added manual test scenario

## Scope Adjustments

### Removed from MVP
- Phase 1 Rust changes → Eliminated (reconciliation in TypeScript)
- `maproom.ollama.host` setting → Removed (hardcode localhost for security)

### Clarified Boundaries
- Phase 1 now: Event types + Ollama client (TypeScript only)
- Phase 2 now: ProcessOrchestrator refactor (not replacement)
- Out of scope: Rust binary changes

## Document Change Summary

### analysis.md
- Added: Reusable components inventory
- Added: Existing infrastructure documentation

### architecture.md
- Changed: WatchProcessManager → ProcessOrchestrator refactor
- Added: Verified CLI flags
- Added: SQLite URL format
- Added: branch_switched event schema
- Added: Component reuse section

### plan.md
- Removed: Phase 1 Rust tickets
- Added: BranchSwitchedEvent ticket
- Updated: Phase numbers (now 5 phases)
- Updated: Agent assignments

### quality-strategy.md
- Added: branch_switched event parsing test
- Added: Reconciliation integration test

### README.md
- Updated: Phase count (6 → 5)
- Updated: Ticket estimate (13 → ~12)
- Updated: Architecture diagram (ProcessOrchestrator refactor, TypeScript reconciliation)
- Updated: Phase descriptions (removed Rust phase)
- Updated: Agent assignments (removed rust-indexer-engineer)
- Updated: Risk assessment (added CLI flags, SQLite format)
- Updated: Next steps (removed review step - already done)

## Verification

**All Updates Complete**

The project planning documents have been updated to address all findings from the project review:

1. **Critical issues resolved**: TypeScript reconciliation, BranchSwitchedEvent added
2. **Reinvention fixed**: Refactoring ProcessOrchestrator instead of replacing
3. **High-risk areas mitigated**: CLI flags verified, SQLite URL documented
4. **Gaps filled**: Component reuse documented, multi-workspace behavior clarified

**Next Steps:**
1. Proceed to `/create-project-tickets VSCEXT`
2. Execute: `/work-on-project VSCEXT`

**Success Metrics:**
- [x] All critical issues resolved
- [x] High-risk areas mitigated
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for ticket creation
