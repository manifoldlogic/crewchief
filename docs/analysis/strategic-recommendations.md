# Strategic Recommendations: CrewChief/Maproom

**Date:** 2026-03-21
**Scope:** Component-level keep/fix/deprioritize/sunset recommendations grounded in competitive landscape data and expert analyses

---

## Executive Summary

CrewChief/Maproom's core differentiator -- worktree-per-agent isolation with token-budgeted context assembly -- is unique and validated by competition benchmarks (162/180 vs. 152/180, 37.9 vs. 54.8 tool calls). However, three of five system connection points are absent (systems-design-critique.md), setup complexity triggers a Competence Quit before most evaluators reach the differentiating workflow (psychology-analysis.md), and the embedding provider requirement blocks the critical path despite FTS handling 88% of agent search calls. The recommendations below prioritize closing these gaps over building new capabilities.

---

## Component Recommendations

### 1. Worktree Management
**Recommendation: Keep**
Rationale: Core to the product vision, low maintenance, and well-implemented. No competitor offers automated worktree creation with agent spawning in a single command.
Key evidence: `scheduler.ts:77-89` links agent spawn to worktree creation (Connection B -- the one fully active integration point). Constraint Alchemist verdict: SIMPLIFY -- already simple, keep as-is.
Open questions: None. Clear to act on.

### 2. Agent Orchestration (Multi-Backend)
**Recommendation: Fix**
Rationale: Most differentiating feature but undermined by iTerm2-default positioning and missing system integrations. The systems-design-critique.md finding that three of five connection points are absent (C: no Maproom scan after spawn, D: bus events inert, E: no MCP socket injection) means spawned agents do not automatically receive search access. The WorktreeMetadataService extension (adding `index_state`, `agent_run_id`, `mcp_socket_path`) is the architectural fix that converts co-location into integration. Constraint Alchemist verdict: PRIORITIZE -- fix the surface, not the code.
Key evidence: `scheduler.ts:128` injects `CREWCHIEF_BUS_PATH` but not `MAPROOM_MCP_SOCKET`. tmux backend exists and is tested but not surfaced as the Linux/Windows default.
Open questions: **Requires human decision** -- whether to invest in closing the integration loop or to accept the current co-located architecture.

### 3. Maproom FTS Search
**Recommendation: Keep**
Rationale: Handles 88% of agent search calls, sub-50ms latency, zero external dependencies. The onboarding-dx-analysis.md frames FTS-only as the default mode (not a fallback), with auto-index on first search (FR-6) as the concrete implementation. Constraint Alchemist verdict: SIMPLIFY -- cut the embedding gate from the critical path.
Key evidence: Competition benchmark achieved with FTS-dominant search. `competitive-landscape-report.md` rates search quality as Adequate -- parity with the field.

### 4. Maproom Vector Search / Multi-Provider Embeddings
**Recommendation: Deprioritize**
Rationale: Three providers, dimension-specific vector tables, and auto-detection for 12% of search usage. The psychology-analysis.md quit point map identifies embedding configuration as the Competence Quit trigger. Constraint Alchemist verdict: KILL OR TRADE -- 88% FTS dominance, marginal demonstrated value.
Key evidence: `architecture.md` rates multi-provider embeddings as over-engineered for demonstrated value.
Open questions: **Requires human decision** -- maintain as opt-in or sunset entirely.

### 5. Maproom Context Assembly (Token-Budgeted Bundles)
**Recommendation: Keep**
Rationale: Unique capability with measured impact. The `--budget 4000 --callers --callees --tests` flag produces context bundles no competitor offers. Constraint Alchemist verdict: PRIORITIZE -- unique, low maintenance cost.
Key evidence: Competition benchmark improvement (37.9 vs. 54.8 tool calls) demonstrates changed agent behavior (`packages/cli/src/search-optimization/`).

### 6. maproom-mcp (Stale Packaging)
**Recommendation: Fix**
Rationale: Functionally active on SQLite but package description reads "[DEPRECATED] MCP server for semantic code search with PostgreSQL." Dead `pg` imports remain in `src/index.ts`. This is an active trust destroyer (constraint-transmutation.md). Constraint Alchemist verdict: SIMPLIFY -- cosmetic fix with outsized trust impact.
Key evidence: `competitive-landscape-report.md` adoption barrier table lists stale deprecation labels as a break point.

### 7. vscode-maproom (Deprecated Extension)
**Recommendation: Sunset**
Rationale: Genuinely deprecated, IDE extensions require ongoing maintenance the team cannot support. Constraint Alchemist verdict: CUT -- already deprecated, formalize the exit. Archive with a clear notice pointing to the MCP server.
Key evidence: `competitive-landscape-report.md` notes IDE integration is Cursor/Windsurf territory.

### 8. Competition/Optimization Framework
**Recommendation: Deprioritize**
Rationale: Internal quality tool that has leaked into the public CLI surface, adding cognitive load for new users. Constraint Alchemist verdict: CUT from public surface. Keep running in CI; remove from user-facing CLI.
Key evidence: `systems-design-critique.md` Section 5 identifies this as complexity without interaction.

### 9. Search A/B Testing Infrastructure
**Recommendation: Deprioritize**
Rationale: Over-engineered for zero known external users. Constraint Alchemist verdict: CUT. Keep in CI; remove from public surface.
Key evidence: Constraint Alchemist triage matrix places this in high-effort, low-centrality quadrant.

---

## Priority Order for Action

Following the psychology-analysis.md intervention sequence (fix hook, fix floor, fix retention):

1. **Make FTS-only the default and clean up maproom-mcp** -- Removes the Competence Quit (setup barrier) and the trust-destroying deprecation label. These are the two changes that most reduce activation energy for first-time evaluators.

2. **Surface tmux as the Linux/Windows default and validate end-to-end** -- Removes the platform lock-in objection. The code exists; the communication does not. This expands the addressable audience without new implementation.

3. **Wire the integration loop (WorktreeMetadataService extension + MCP socket injection)** -- Closes the retention gap by ensuring spawned agents have search access automatically. Without this, the "why not Claude Code native?" Motivation Quit at day 3-7 remains unanswered in practice.

---

## Decisions Requiring Human Review

The following cannot be resolved by analysis alone:

- **Vector search disposition**: Deprioritize (make opt-in) vs. sunset entirely. The 12% usage figure suggests opt-in, but maintaining three embedding providers has ongoing cost.
- **Agent orchestration investment level**: Whether to invest in closing the integration loop (WorktreeMetadataService + bus wiring + MCP injection) or accept the current co-located architecture. This is the largest engineering decision in the assessment.
- **Naming strategy**: The "CrewChief" name collision with the racing simulator is a pure cost with no creative angle (constraint-transmutation.md). Evaluate whether leading with the "maproom" brand reduces collision surface area enough, or whether a rename is warranted.
- **maproom CLI passthrough**: Whether to merge Maproom as native subcommands, separate as a standalone binary, or maintain the current `spawnSync` hybrid. The systems-design-critique.md identifies this as a boundary failure, but the fix direction depends on the integration investment decision above.

---

*This document is a strategy memo, not a decision record. The component disposition matrix (CRITIQUE.3001) is where keep/pivot/sunset decisions are formally recorded. All recommendations are inputs to human decision-making.*
