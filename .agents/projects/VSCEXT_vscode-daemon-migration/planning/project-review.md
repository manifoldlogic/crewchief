# Project Review: VSCode Extension Daemon Migration

**Review Date:** 2025-11-27
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

This project is well-scoped and addresses real technical debt. The planning documents are comprehensive, the architecture is sound, and the security posture actually improves by removing Docker. However, there are several concerns that should be addressed before ticket creation:

1. **Significant reuse opportunity missed**: The project plans to create a new `WatchProcessManager` from scratch, but most of the infrastructure already exists in `ProcessOrchestrator` - it just needs simplification, not replacement.

2. **Phase 1 Rust changes may be unnecessary**: The plan assumes startup reconciliation requires Rust changes, but `git diff` can be run from TypeScript before spawning watch. This could simplify the project significantly.

3. **Ollama detection already exists**: `setupWizard.ts` already has `detectOllama()` - the new `OllamaClient` should extend this, not replace it.

4. **Events type needs `branch_switched`**: The existing `events.ts` doesn't include the `branch_switched` event type that the unified watch emits.

Overall, the project should proceed but with scope adjustments to leverage existing code better and potentially eliminate Phase 1.

## Critical Issues (Blockers)

### Issue 1: Startup Reconciliation Can Be Done in TypeScript

**Severity:** High (affects Phase 1 scope)
**Category:** Architecture
**Description:** The plan assumes we need Rust changes to add startup reconciliation. However, the TypeScript extension can simply run `git diff --name-only <last-commit>..HEAD` and call `crewchief-maproom upsert` before starting watch. This is already a pattern used elsewhere (scan before watch).

**Impact:** Phase 1 (2 Rust tickets) may be completely unnecessary, adding complexity to the Rust codebase when the problem can be solved at the orchestration layer.

**Required Action:**
1. Evaluate if TypeScript-based reconciliation is sufficient
2. If so, move reconciliation logic to WatchProcessManager
3. Remove Phase 1 or reduce it to just tracking last indexed commit

**Documents Affected:** plan.md, architecture.md

### Issue 2: Missing `branch_switched` Event Type

**Severity:** Critical (will cause runtime errors)
**Category:** Requirements
**Description:** The existing `events.ts` defines `WatchEvent` as a union of `progress`, `error`, `complete`, `status`, and `file_processed` events. It does NOT include `branch_switched`, which the unified watch command emits (see `crates/maproom/src/indexer/mod.rs:100`).

**Impact:** The NDJSON parser will reject `branch_switched` events as invalid, logging parse errors instead of handling branch switches properly.

**Required Action:**
1. Add `BranchSwitchedEvent` interface to `events.ts`
2. Update `isWatchEvent()` type guard to handle it
3. Update status bar to display branch name from these events

**Documents Affected:** plan.md, quality-strategy.md

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

**Existing Solution:** `ProcessOrchestrator` class with full lifecycle management
**Proposed Duplication:** New `WatchProcessManager` class
**Wasted Effort:** ~4-8 hours of unnecessary rewrite
**Recommendation:** Refactor `ProcessOrchestrator` to spawn single watch process instead of replacing it. The existing code has:
- Platform-aware binary selection ✅
- Graceful shutdown (SIGTERM → SIGKILL) ✅
- NDJSON parsing via `StdoutParser` ✅
- Crash recovery via `CrashRecovery` ✅
- EventEmitter integration ✅

### Missed Reuse Opportunities

| Available Component | Could Solve | Integration Method | Effort |
|---------------------|-------------|-------------------|--------|
| `detectOllama()` in setupWizard.ts | Ollama running check | Function call | Low |
| `StdoutParser` in parser.ts | NDJSON parsing | Direct use | None |
| `CrashRecovery` in recovery.ts | Auto-restart logic | Direct use | None |
| `SecretsManager` | API key storage | Already planned | None |

### Pattern Violations

**Existing Pattern:** Extension uses `ProcessOrchestrator` with `StdoutParser` for NDJSON events
**Proposed Deviation:** Creating new `WatchProcessManager` with inline parsing
**Consistency Impact:** Two different process management patterns in codebase
**Recommendation:** Refactor existing orchestrator, don't replace it

## High-Risk Areas (Warnings)

### Risk 1: Watch Command May Not Support All Required Flags

**Risk Level:** Medium
**Category:** Technical
**Description:** The architecture assumes `crewchief-maproom watch --path <workspace>` is the correct invocation. However, the current Rust code uses `--repo`, `--worktree`, and `--throttle` flags.
**Probability:** Medium
**Impact:** High - watch won't start correctly
**Mitigation:** Verify exact CLI interface before ticket creation. Check `crates/maproom/src/main.rs` for Watch command definition.

### Risk 2: SQLite Database Path Resolution

**Risk Level:** Medium
**Category:** Integration
**Description:** The architecture mentions `~/.maproom/maproom.db` as the default SQLite path, but the extension must coordinate with how the Rust binary resolves this path.
**Probability:** Medium
**Impact:** Medium - index not found errors
**Mitigation:** Ensure `MAPROOM_DATABASE_URL` format is correct for SQLite (`sqlite:///path/to/db` vs `file://path`).

### Risk 3: OpenAI/Google Provider Users

**Risk Level:** Low
**Category:** Execution
**Description:** The project focuses heavily on Ollama but doesn't address what happens when users select OpenAI or Google providers. They don't need model pulling, but do they need different error handling?
**Probability:** Low
**Impact:** Low - just skip model pull
**Mitigation:** Ensure `ensureOllamaModel()` is only called when provider === 'ollama'.

## Gaps & Ambiguities

### Requirements Gaps

1. **Reconciliation event type undefined**: Plan says "NDJSON event emitted for reconciliation progress" but doesn't define what this event looks like. Is it a new event type or reusing `progress`?

2. **First-run behavior unclear**: What happens on absolutely first run when there's no SQLite database? The current `showNoSqliteGuidance()` handles this, but plan doesn't mention preserving it.

3. **Multi-workspace behavior**: Current extension handles multiple VSCode workspaces. Does single watch process work for this?

### Technical Gaps

1. **Exact watch CLI arguments**: Need to verify the exact command-line interface for the unified watch command.

2. **`branch_switched` event schema**: Need to add this to `events.ts` with proper type guard.

3. **Reconciliation trigger**: How does the watch process know to reconcile? Command-line flag? Automatic on startup?

### Process Gaps

1. **Migration testing**: How do we test that users with existing Docker+PostgreSQL setups transition smoothly?

2. **Rollback plan**: If the new architecture fails, can users revert to previous extension version?

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Phase 1 Rust changes**: Adding features to the Rust binary when TypeScript orchestration could suffice. Consider deferring to keep scope minimal.

2. **New Ollama settings**: Adding `maproom.ollama.host` setting when Ollama should only ever be localhost. Over-configuring for edge cases.

### Feasibility Challenges

1. **Unified watch assumption**: The plan assumes the unified watch command is fully ready, but we should verify it handles all the cases `branch-watch` handled separately.

2. **Model pull duration**: Large models (nomic-embed-text is ~270MB) can take significant time to pull. The progress notification UX needs to handle this gracefully.

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate
- Good: Removing Docker is a real simplification
- Concern: Phase 1 Rust changes may be overengineering
- Recommendation: Evaluate if TypeScript-only solution is sufficient first

### Pragmatism Score
**Rating:** Adequate
- Good: Quality strategy is focused on critical paths
- Concern: Creating new `WatchProcessManager` instead of refactoring existing
- Recommendation: Leverage existing `ProcessOrchestrator` infrastructure

### Agent Compatibility
**Rating:** Strong
- Tickets are appropriately scoped (2-8 hours)
- Agent assignments match capabilities
- Verification criteria are explicit

### Codebase Integration
**Rating:** Weak
- Plan doesn't leverage existing `StdoutParser`, `CrashRecovery`
- Proposes new component instead of refactoring existing
- Misses existing Ollama detection code

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] Dependencies on existing systems documented - **Missing: exact CLI interface**

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] Integration points are well-defined - **Need CLI verification**
- [x] Performance requirements are clear
- [x] Error handling is specified
- [ ] Existing tools/libraries identified for reuse - **Missed opportunities**
- [ ] No unnecessary duplication of functionality - **WatchProcessManager duplicates**

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [ ] Rollback plan exists - **Not addressed**
- [ ] Integration with existing workflows considered - **Partial**

### Integration & Reuse
- [ ] Existing solutions evaluated before building new - **Not fully**
- [ ] Current patterns and conventions followed - **Deviates from orchestrator pattern**
- [ ] Reusable components identified - **Missed parser, recovery**
- [x] Integration points with existing systems mapped
- [ ] No reinvention of available functionality - **WatchProcessManager**
- [x] CLI for high-level orchestration ✓
- [x] Binary execution for standalone operations ✓
- [x] Component boundaries respected
- [x] Public interfaces used

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [ ] Dependencies have fallbacks - **What if Ollama not installed?**
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Add `BranchSwitchedEvent` to events.ts** - This is a prerequisite for handling unified watch output correctly.

2. **Verify exact watch CLI interface** - Run `crewchief-maproom watch --help` and document the exact flags needed.

3. **Evaluate Phase 1 necessity** - Test if TypeScript-based reconciliation (`git diff` + `upsert`) is sufficient before committing to Rust changes.

4. **Update architecture to refactor, not replace** - Change plan from "new WatchProcessManager" to "refactor ProcessOrchestrator".

### Phase 1 Adjustments

- Consider renaming to "Extension Flow Update" if Rust changes are removed
- Add ticket for `BranchSwitchedEvent` type definition
- Ensure `detectOllama()` is reused, not recreated

### Risk Mitigations

1. **Add explicit Ollama-not-installed handling** - Show "Install Ollama" button with link to https://ollama.ai

2. **Document SQLite path format** - Ensure consistency between TypeScript and Rust path handling

3. **Test multi-workspace scenario** - Verify single watch process handles multiple VSCode windows

### Documentation Updates

- **architecture.md**: Change WatchProcessManager to ProcessOrchestrator refactor
- **plan.md**: Add ticket for `BranchSwitchedEvent`, reconsider Phase 1
- **quality-strategy.md**: Add test for branch_switched event parsing

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. Phase 1 Rust changes may be unnecessary complexity
2. Missing `branch_switched` event type will cause runtime errors
3. Significant existing code can be reused but isn't planned for

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the critical `branch_switched` event issue and evaluate whether Phase 1 Rust changes are necessary before creating tickets. The project is well-scoped overall but should leverage existing code better.

### Success Probability
Given current state: 70%
After recommended changes: 90%

### Final Notes

This is a good project that addresses real technical debt. The main concerns are about unnecessary complexity (Rust changes, new class instead of refactor) rather than fundamental issues. With the recommended adjustments, this project should succeed and significantly improve the extension architecture.

Key wins:
- Removing Docker dependency is the right direction
- Single watch process simplifies the architecture
- Ollama model management improves UX for new users
- Security posture improves (no network-exposed DB)

The project should proceed after:
1. Adding `BranchSwitchedEvent` to events.ts
2. Verifying watch CLI interface
3. Deciding on Phase 1 scope (Rust vs TypeScript reconciliation)
4. Updating plan to refactor ProcessOrchestrator instead of replacing it
