## CrewChief Test Plan

This document defines the test strategy, scope, cases, and execution approach for CrewChief as specified in `crewchief_context/cli/specification.md` and implemented per `crewchief_context/cli/technology.md` and `crewchief_context/cli/project-plan.md`.

### Objectives

- Verify core functionality across Phases 1–3 and setup from Phase 4
- Ensure safe behavior for git/tmux side effects (worktrees, panes, merges)
- Establish a path to CI-ready automation (unit + integration + manual smoke)

### Scope

- CLI surface: setup, init, session, worktree, agent, runs, eval, merge, task, competition
- Services: config loader/schema, git worktrees/merge, tmux service, message bus, JSONL follower, run manager, evaluation
- Environments: macOS/Linux with tmux and git installed; Node ≥ 18; pnpm

### Out of Scope (initial)

- Real Claude/Gemini integration (covered by mock agent)
- Advanced UX (progress bars/spinners)

---

## Test Types

### 1) Unit Tests (automated, Vitest)

- JSONL encode/decode
- MessageBus send/subscribe and waitForResponse
- Evaluation default checks (environment presence + config-driven checks)

Run: `pnpm test` (non-watch) or `pnpm test:watch` (watch)

### 2) Integration Tests (staged)

Target: add in subsequent iterations (partial manual now, see E2E flows)

- Config loader validation (valid/invalid schema)
- Worktree lifecycle against a temporary git repo
- Tmux service (guarded; skip on CI if tmux not present)
- Agent lifecycle using mock agent (spawn/message/close) with per-run logs
- Evaluation and merge (squash/ff/cherry-pick) in a temp repo

Strategy: use temporary directories and initialize a fixture git repo per test to avoid polluting the working tree.

### 3) End-to-End Smoke (manual for now)

Validates the happy path end-to-end using the mock agent.

---

## Test Matrix (features → coverage)

- Setup Wizard
  - Positive: writes `crewchief.config.ts` with chosen values
  - Negative: invalid inputs (n/a, handled by prompts)

- Init & Session
  - `init` creates `.crewchief/` directories
  - `session start` starts tmux session; re-run is idempotent

- Worktrees
  - `create` from main; `list` shows entries; `clean` prunes
  - Negative: duplicate name; missing branch

- Agent Lifecycle (mock agent)
  - `spawn` creates worktree and pane, writes logs, events
  - `message` sends keys to pane; echoed as JSONL
  - `close` closes pane and updates run status

- Runs & Observability
  - `runs list`, `runs events <runId>`, `runs logs <runId> --tail 50`

- Evaluation
  - Default checks return results and score ≥ 0
  - Config-driven `qualityChecks` run and affect score

- Merge
  - `merge run` with strategies: squash, ff, cherry-pick (on temp repo)
  - Negative: dirty working tree → fails gracefully

- Auto-merge
  - `merge auto <runId>` merges only if score ≥ threshold

- Tasks
  - `task assign <agentType> <desc>` creates run; logs captured

- Competition
  - `competition start/assign/evaluate` picks winner by score
  - `competition finalize` attempts auto-merge winner

- Error Recovery
  - Pane/worktree cleanup on failures (best-effort)

- Realm & Retrieval (new)
  - `realm build` populates index; query returns topK with metadata
  - Indexers handle repo files and bus transcripts

- Cross-Agent Injection (new)
  - Policy denies/allows based on allow/deny lists
  - Dry-run shows intended actions without sending keys

- Benchmarking (new)
  - Scenario parsing and batch execution produce results
  - Leaderboard sorts by weighted scores

- Observability (new)
  - Correlation ids propagate across related events
  - `runs logs --tail` shows concise, formatted output

---

## Detailed Manual E2E Flows

Prereqs: tmux, git, Node ≥ 18, pnpm; clean git repo on `main`.

1) Setup & Init

- `pnpm build`
- `node dist/index.js setup` → accept defaults
- `node dist/index.js init` → `.crewchief/` created

2) Session

- `node dist/index.js session start` → tmux session present

3) Agent (mock)

- `node dist/index.js agent spawn mock-agent "Validate JSONL"`
- `node dist/index.js agent message mock-agent "ping"`
- `node dist/index.js runs list` → capture runId
- `node dist/index.js runs events <runId>` → JSONL events present
- `node dist/index.js runs logs <runId> --tail 50`
- `node dist/index.js eval run <runId>` → score computed
- `node dist/index.js merge auto <runId>` → may merge depending on score/threshold

4) Task & Merge

- `node dist/index.js task assign mock-agent "Do work"` → run created, logs
- Manually run: `node dist/index.js merge run <branch> --strategy squash`

5) Competition

- `node dist/index.js competition start "Feat X" mock-agent project-manager`
- `node dist/index.js competition assign <compId>` → multiple runs
- `node dist/index.js competition evaluate <compId>` → winner shown
- `node dist/index.js competition finalize <compId>` → auto-merge if above threshold

6) Cleanup

- `node dist/index.js agent close mock-agent` (if still running)
- `node dist/index.js worktree clean`

Expected: no unhandled exceptions; logs written under `.crewchief/runs/`; worktrees/panes cleaned when requested.

---

## Additional Tests for New Areas

### Realm

- Unit
  - Embedding provider adapter returns vectors of correct dim
  - In‑memory vector index: upsert/query happy and edge cases
  - Indexer emits canonical nodes for files and messages
- CLI
  - `realm build --incremental` only reindexes changed inputs
  - `realm query` returns stable ordering for identical inputs

### Cross‑Agent Injection

- Unit
  - Policy engine evaluates allow/deny and rate limits
  - Audit events include correlation ids and payload redaction where needed
- Manual
  - `agent inject <from> <to> "echo hi" --enter` types into destination pane

### Benchmarking

- Unit
  - Scenario loader validates schema and defaults
  - Aggregator computes weighted totals correctly
- Manual
  - `eval benchmark <scenarioId> --agents a,b` writes results and leaderboard

### Observability

- Unit
  - Envelope enrichment adds correlation and phase without mutation
  - Log follower tailing respects `--tail` and formats correctly
- Manual
  - `runs events <runId>` groups by correlation id/phase for quick scanning

---

## Tooling & Execution

- Unit: `pnpm test` (non-watch), `pnpm test:watch` (watch)
- Build: `pnpm build`
- Manual: execute E2E flows; use `runs logs` for diagnostics

CI (later):

- Run unit tests on push
- Conditional integration tests on Linux with tmux available (or mocked)
- Cache pnpm store; upload logs on failure

---

## Test Data & Fixtures

- Temp git repos for merge/worktree tests (in `/tmp` or OS temp dirs)
- Mock agent `scripts/mock-agent.js` emits JSONL for pipeline validation

---

## Risks & Mitigations

- Tmux not installed → guard tests; friendly errors
- OS compatibility (darwin/Linux) → avoid OS-specific assumptions
- Git state pollution → always use temp repos for destructive tests

---

## Acceptance Criteria (Release Gate)

- Unit tests green
- Manual E2E happy path verified
- No dangling panes/worktrees after normal flows
- Merge operations succeed or fail with clear errors; rollback helpers invoked on failure
