# Project Overview

CrewChief is a TypeScript-based multi-agent orchestration tool that enables AI agents to collaborate on repositories using isolated git worktrees with visual coordination through iTerm2 on macOS.

## Commands

### Package Management

- **Use pnpm** - This project uses pnpm as the package manager (not npm or yarn)
- `pnpm install` - Install dependencies
- `pnpm test` - Run tests (currently not configured)

### CLI Entry Points

- `node dist/index.js --help` — after `pnpm build`
- `pnpm build` emits ESM output to `dist/`

### Core CLI Commands

- Setup & Init
  - `node dist/index.js setup` — interactive configuration wizard
  - `node dist/index.js init` — prepare `.crewchief/`
- Session & Worktrees
  - `node dist/index.js session start`
  - `node dist/index.js worktree create <name> [--branch <branch>] [--base-path <dir>]` — create a worktree from a base branch
  - `node dist/index.js worktree list` — list all repo worktrees and annotate known agent-owned ones
  - `node dist/index.js worktree clean` — prune stale or remove all non-current worktrees
  - `node dist/index.js worktree use <name> [--print]` — use a worktree (creates if needed) by starting a subshell in it, or print its path with `--print`
- Agents
  - `node dist/index.js agent spawn <type> <task>`
  - `node dist/index.js agent message <agentId> <message>`
  - `node dist/index.js agent close <agentId>`
- Runs & Observability
  - `node dist/index.js runs list`
  - `node dist/index.js runs events <runId>`
  - `node dist/index.js runs logs <runId> [--tail 200]`
- Evaluation & Merge
  - `node dist/index.js eval run <runId>`
  - `node dist/index.js merge run <branch> [--target main] [--strategy squash|ff|cherry-pick]`
  - `node dist/index.js merge auto <runId>`
- Tasks & Competition
  - `node dist/index.js task assign <agentTypeId> <description>`
  - `node dist/index.js competition start <description> <agentIds...>`
  - `node dist/index.js competition assign <competitionId>`
  - `node dist/index.js competition evaluate <competitionId>`
  - `node dist/index.js competition finalize <competitionId>`

## Architecture & Structure

### Core Components

1. **Orchestrator** - Main control pane for project planning, task distribution, quality evaluation, and worktree merging
2. **Agent Management** - Handles multiple AI agents (Claude, Gemini, custom) working in isolated worktrees
   - Claude agents use `.claude/agents/` for native agent definitions and `.claude/commands/` for reusable commands
   - Gemini agents use `.gemini/agents/` and `.gemini/commands/` for gemini.
   - Codex agents use `.codex/agents/` and `.codex/commands/` for codex.
3. **Tmux Integration** - Visual layout and pane management for real-time agent visibility
4. **Git Worktree Management** - Abstracts complex worktree operations into simple commands

### Key Concepts

- **Worktrees** - Isolated git environments to isolate work without conflicts; available for agent and non‑agent workflows alike
- **Competition Mode** - Compare different agents/configurations on identical tasks
- **Task Distribution** - Automatic assignment and evaluation of work across agents
- **Message Bus** - Bidirectional communication protocol between orchestrator and agents

### Documentation

- All technical documentation is in the `crewchief_context/` folder
- `crewchief_context/cli/specification.md` contains the full system specification and implementation roadmap
- `crewchief_context/cli/project-plan.md` contains the detailed implementation plan

## Development Guidelines

### Code Principles

- Use the crewchief_context folder for all documentation and diligently keep it up to date
- Focus on ease of use and simplicity
- Make the terminal interface rich and delightful
- Use colors with meaning not haphazardly

### Implementation Phases

1. **Phase 1**: Core foundation - Git worktree wrapper, tmux session management
2. **Phase 2**: Agent integration - Claude/Gemini CLI patterns, message passing
3. **Phase 3**: Orchestration - Task distribution, quality evaluation, merging
4. **Phase 4**: Polish - Interactive setup wizard, competition features

### Technology Stack

- TypeScript for all implementation
- Commander for CLI framework
- Simple-git for git operations
- Node-pty for terminal control
- Chalk for terminal colors
- Inquirer for interactive prompts
- Zod for schema validation

---

## Realm & Semantic Retrieval

- Modules (proposed):
  - `src/realm/ontology.ts` — types and graph storage for entities/relations
  - `src/realm/indexer.ts` — builders for repo files and bus transcripts
  - `src/realm/providers/embedding.ts` — provider SPI (`openai`, `vertex`, `local`)
  - `src/realm/vectorIndex/memory.ts` — in‑memory vector index (MVP)
  - `src/cli/realm.ts` — `realm build` and `realm query` commands
- Config additions (`crewchief.config.ts`):
  - `realm.provider`: `openai` | `vertex` | `local`
  - `realm.model`: string
  - `realm.paths`: include/exclude globs for files and transcripts

## Cross‑Agent Input Injection

- Implementation:
  - Use `src/tmux/tmux.service.ts` for `sendKeys`
  - Add `src/orchestrator/injection.ts` for policy checks and auditing
  - Extend `src/cli/agent.ts` with `inject` subcommand
- Policy surface in config:
  - `injection.policy.allowList`, `denyList`, `maxPerMinute`

## Benchmarking & Tournaments

- Modules:
  - `src/evaluation/benchmark.ts` — scenario loader and batch runner
  - `src/evaluation/leaderboard.ts` — results aggregation and persistence
  - CLI in `src/cli/eval.ts`: `eval benchmark <scenarioId> [--agents ...]`
- Data locations:
  - Scenarios: `.crewchief/benchmarks/scenarios/*.json`
  - Results: `.crewchief/benchmarks/results/*.jsonl`

## Observability Upgrades

- Extend `src/bus/message.types.ts` with `correlationId`, `parentId`, `phase`
- Summarizers:
  - `src/bus/logFollower.ts` additions for tailing and formatting
  - `src/bus/index.ts` helpers to group by correlation id/phase
- Output locations:
  - `.crewchief/runs/<runId>/events.jsonl`
  - `.crewchief/runs/<runId>/summary.json`
