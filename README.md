# CrewChief (WIP)

TypeScript CLI to orchestrate multiple AI agents working on isolated git worktrees with tmux visibility.

## Quick start

- pnpm install
- pnpm build
- crewchief --help  
  (or run without installing: `npx crewchief --help` or `pnpm dlx crewchief --help`)

## Core commands

- Initialization
  - `crewchief setup` — interactive config wizard
  - `crewchief init` — prepare `.crewchief/`
- Worktrees & session
  - `crewchief session start`
  - `crewchief worktree create <name> [--branch <branch>]`
  - `crewchief worktree list`
  - `crewchief worktree clean`
- Agents
  - `crewchief agent spawn <type> <task>`
  - `crewchief agent message <agentId> <message>`
  - `crewchief agent close <agentId>`
- Runs
  - `crewchief runs list`
  - `crewchief runs events <runId>`
  - `crewchief runs logs <runId> [--tail 200]`
- Evaluation & merge
  - `crewchief eval run <runId>`
  - `crewchief merge run <branch> [--target main] [--strategy squash|ff|cherry-pick]`
  - `crewchief merge auto <runId>`
- Tasks & competition
  - `crewchief task assign <agentTypeId> <description>`
  - `crewchief competition start <description> <agentIds...>`
  - `crewchief competition assign <competitionId>`
  - `crewchief competition evaluate <competitionId>`
  - `crewchief competition finalize <competitionId>`

## Notes

- ESM output, requires Node >= 18
- Tmux must be installed and on PATH
- Mock agent available via `mock-agent` for JSONL pipeline validation

### Install options

- Global install: `npm i -g crewchief` (or `pnpm add -g crewchief`), then run `crewchief ...`
- Run without install: `npx crewchief@latest ...` or `pnpm dlx crewchief@latest ...`
