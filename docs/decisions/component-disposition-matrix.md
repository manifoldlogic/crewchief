# Component Disposition Matrix: CrewChief/Maproom

**Status:** Final Decision Record
**Based on:** CRITIQUE.1001 (competitive landscape report), CRITIQUE.1002 (strategic recommendations)
**Cross-referenced:** planning/constraint-transmutation.md (Feature Triage Matrix)
**Date:** 2026-03-21

---

## Component Dispositions

### 1. Worktree Management

**Disposition: Keep**
Rationale: Worktree-per-agent isolation is the single capability no competitor offers. Stable, low-maintenance, core to the product vision.
Key evidence: CRITIQUE.1001 rates agent orchestration as CrewChief's only "Strong" dimension; worktree management is its foundation (`scheduler.ts:77-89`).
Constraint Alchemist triage verdict: SIMPLIFY -- already simple, keep as-is. Aligned.
Phase 2 action: None.

### 2. Agent Orchestration (Multi-Backend)

**Disposition: Keep and Fix Platform Reach**
Rationale: Most differentiating feature, but iTerm2-default positioning limits reach. The tmux backend exists and is tested; the gap is documentation and defaulting, not implementation. Three of five system integration points are absent, meaning spawned agents lack automatic search access.
Key evidence: CRITIQUE.1001 rates platform reach as "Weak" -- every competitor works cross-platform without backend selection.
Constraint Alchemist triage verdict: PRIORITIZE -- fix the surface, not the code. Aligned.
Phase 2 action: CRITIQUE.2001 (tmux validation and documentation), CRITIQUE.2004 (WorktreeMetadataService extension to close integration loop).

### 3. Maproom FTS Search

**Disposition: Keep**
Rationale: Handles 88% of agent search calls with sub-50ms latency, zero external dependencies. FTS-only must become the default mode, removing the embedding provider from the critical path.
Key evidence: CRITIQUE.1001 identifies embedding configuration as the Competence Quit trigger. Competition benchmark (162/180 vs. 152/180) achieved with FTS-dominant search.
Constraint Alchemist triage verdict: SIMPLIFY -- cut the embedding gate from the critical path. Aligned.
Phase 2 action: CRITIQUE.2002 (FTS-first default), CRITIQUE.2005 (auto-index on first search).

### 4. Maproom Vector Search / Multi-Provider Embeddings

**Disposition: Deprioritize**
Rationale: Three providers and dimension-specific vector tables serve 12% of search usage while creating the largest onboarding barrier. Maintained as opt-in, removed from critical path.
Key evidence: CRITIQUE.1002 identifies embedding configuration as the Competence Quit trigger. 88% FTS dominance in benchmarks.
Constraint Alchemist triage verdict: KILL OR TRADE. This disposition is more conservative (deprioritize rather than kill), preserving opt-in access.
Phase 2 action: Addressed indirectly by CRITIQUE.2002 (making embeddings opt-in rather than default).

### 5. Maproom Context Assembly (Token-Budgeted Bundles)

**Disposition: Keep**
Rationale: Unique capability. Token-budgeted context bundles (`--budget 4000 --callers --callees --tests`) are unmatched by any competitor. Benchmark reduction from 54.8 to 37.9 tool calls demonstrates measurable agent behavior change.
Key evidence: CRITIQUE.1001 "Where CrewChief Wins" section 2. Benchmark data in `packages/cli/src/search-optimization/`.
Constraint Alchemist triage verdict: PRIORITIZE -- unique, low maintenance cost. Aligned.
Phase 2 action: None directly; benefits from CRITIQUE.2005 (auto-index provides data for context assembly).

### 6. maproom-mcp (Stale Packaging)

**Disposition: Clean Up and Promote**
Rationale: Functionally active on SQLite, but package description reads "[DEPRECATED]" with dead `pg` imports. An active trust destroyer -- any evaluator who sees "[DEPRECATED]" stops reading. Bounded, low-effort fix.
Key evidence: CRITIQUE.1001 flags stale packages in maintenance burden analysis. CRITIQUE.1002 identifies this as first-priority action.
Constraint Alchemist triage verdict: SIMPLIFY -- cosmetic fix with outsized trust impact. Aligned.
Phase 2 action: CRITIQUE.2003 (maproom-mcp cleanup).

### 7. vscode-maproom (Deprecated Extension)

**Disposition: Sunset Formally**
Rationale: Genuinely deprecated. IDE extensions require ongoing maintenance the team cannot support. The MCP server is the recommended integration path going forward. Archive with a clear notice redirecting to maproom-mcp.
Key evidence: CRITIQUE.1001 notes IDE integration is Cursor/Windsurf territory; competing on IDE extensions is not viable for a small team.
Constraint Alchemist triage verdict: CUT -- already deprecated, formalize the exit. Aligned.
Phase 2 action: Project-lead action item (see Phase 2 Go/No-Go below). Not covered by any agent task.

### 8. Competition/Optimization Framework

**Disposition: Keep as Internal Tool**
Rationale: Useful for internal quality measurement but has leaked into the public CLI surface. Keep running in CI; remove from user-facing CLI.
Key evidence: CRITIQUE.1002 cites systems-design-critique.md Section 5: complexity without interaction.
Constraint Alchemist triage verdict: CUT from public surface. Aligned (keep internally, remove from public surface).
Phase 2 action: None in current Phase 2 scope. Public surface removal is a future task.

### 9. Search A/B Testing Infrastructure

**Disposition: Deprioritize**
Rationale: Over-engineered for zero known external users. Keep in CI; remove from user-facing documentation.
Key evidence: CRITIQUE.1002 places this in the high-effort, low-centrality quadrant.
Constraint Alchemist triage verdict: CUT. Aligned.
Phase 2 action: None.

---

## Phase 2 Go/No-Go

Based on the disposition decisions above, the following Phase 2 actions are **authorized**:

**Phase 2A (Core Fixes):**
- [ ] **CRITIQUE.2001 (tmux backend validation and documentation, FR-1)** -- Authorized. Agent orchestration disposition is "Keep and Fix Platform Reach." The tmux backend exists in code; the work is end-to-end validation and documentation, not implementation.
- [ ] **CRITIQUE.2002 (FTS-first default, FR-3)** -- Authorized. Maproom FTS disposition is "Keep." Embedding complexity is the single largest adoption barrier; defaulting to FTS-only removes it from the critical path.
- [ ] **CRITIQUE.2003 (maproom-mcp cleanup, FR-5)** -- Authorized. maproom-mcp disposition is "Clean Up and Promote." Stale deprecation label is an active trust destroyer requiring bounded, low-effort cleanup.

**Phase 2B (Systems Integration and Onboarding):**
- [ ] **CRITIQUE.2004 (WorktreeMetadataService extension, FR-9 + FR-10)** -- Authorized. Agent orchestration requires closing the integration loop. Adding `index_state`, `agent_run_id`, `mcp_socket_path` converts co-located systems into integrated ones.
- [ ] **CRITIQUE.2005 (auto-index on first search, FR-6)** -- Authorized. FTS search disposition is "Keep" and the onboarding analysis identifies auto-index as the concrete implementation of the "install, search" narrative.
- [ ] **CRITIQUE.2006 (top-level command aliases, FR-7)** -- Authorized. Hides the TypeScript/Rust seam from new users by providing `crewchief search`, `crewchief index`, `crewchief context` as transparent aliases.
- [ ] **CRITIQUE.2007 (doctor capability-tier redesign, FR-8)** -- Authorized. Restructures `crewchief doctor` from pass/fail checklist to capability tiers, eliminating deficiency-first messaging that discourages evaluators.

**FR-2 Disposition:**
FR-2 ("crewchief maproom commands must add value beyond raw passthrough"): Addressed by combination of CRITIQUE.2004 (genuine integration beyond passthrough), CRITIQUE.2005 (auto-index adds behavior the raw binary does not offer), and CRITIQUE.2006 (aliases conceal the passthrough seam).

**Project-Lead Action Item:**
- **vscode-maproom formal sunset:** Package archival on npm, README redirect to maproom-mcp, and removal from active CI pipelines are required. These are project-lead action items not covered by any agent task.

No Phase 2 actions are **deferred** or **cancelled**. All seven tasks are authorized based on the disposition decisions above.

---

## Decisions Updated from Preliminary Assessment

All preliminary recommendations from plan.md are confirmed. No changes were required based on Phase 1 findings. The CRITIQUE.1001 competitive landscape data and CRITIQUE.1002 strategic analysis corroborate each preliminary disposition, and the Constraint Alchemist triage verdicts align with every final decision.
