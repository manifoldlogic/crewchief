# CrewChief (WIP)

TypeScript CLI to orchestrate multiple AI agents working on isolated git worktrees with tmux visibility.

## Quick start

- pnpm install
- pnpm build
- node dist/index.js --help

## Core commands

- Initialization
  - `node dist/index.js setup` — interactive config wizard
  - `node dist/index.js init` — prepare `.crewchief/`
- Worktrees & session
  - `node dist/index.js session start`
  - `node dist/index.js worktree create <name> [--branch <branch>]`
  - `node dist/index.js worktree list`
  - `node dist/index.js worktree clean`
- Agents
  - `node dist/index.js agent spawn <type> <task>`
  - `node dist/index.js agent message <agentId> <message>`
  - `node dist/index.js agent close <agentId>`
- Runs
  - `node dist/index.js runs list`
  - `node dist/index.js runs events <runId>`
  - `node dist/index.js runs logs <runId> [--tail 200]`
- Evaluation & merge
  - `node dist/index.js eval run <runId>`
  - `node dist/index.js merge run <branch> [--target main] [--strategy squash|ff|cherry-pick]`
  - `node dist/index.js merge auto <runId>`
- Tasks & competition
  - `node dist/index.js task assign <agentTypeId> <description>`
  - `node dist/index.js competition start <description> <agentIds...>`
  - `node dist/index.js competition assign <competitionId>`
  - `node dist/index.js competition evaluate <competitionId>`
  - `node dist/index.js competition finalize <competitionId>`

## Notes

- ESM output, requires Node >= 18
- Tmux must be installed and on PATH
- Mock agent available via `mock-agent` for JSONL pipeline validation
