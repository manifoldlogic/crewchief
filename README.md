# `crewchief` — Delegate to AI agents without crossing your fingers. 🤞

## Orchestrate AI Agents. Multiply Output

`crewchief` is a CLI that turns your terminal into a **ops bridge** — where multiple AI agents each work in their own `tmux` stations, collaborating, competing, and communicating through an inspectable channel, all under your direction.

* **Coordinate or compete** — Configure agents to type into each other’s panes, hand off work, or compete in quality tournaments.
* **Benchmark & improve** — Run competition models to push quality higher, with built-in benchmarking tools.
* **Observe everything** — A logged message bus lets you inspect and debug every inter-agent exchange.
* **Smart context management** — Shared vector database ensures agents access the same knowledge and maintain consistency across tasks.
* **Isolated workspaces** — Each agent operates in its own git worktree, preventing conflicts and enabling clean review, testing, and merge workflows for every task.

---

## Multi-Agent Orchestration

Zsh commands for orchestrating `claude`, `gemini`, `cursor-agent`, and more — all running in isolated `tmux` panes. Extend functionality through pluggable agent modules.

## Agent Resource Isolation

Each active agent works in its own sandbox with a dedicated `tmux` pane, `git worktree`, and `vector-index` for clean separation.

## Interactive Command Hierarchies

Build multi-level agent structures where senior agents delegate to juniors, while you maintain the ability to intervene in any active session at any time. Delegate without crossing your fingers.🤞

## Quality Optimization Framework

Run agent competitions and benchmarking tools to continuously improve output quality.

## Inter-Agent Communication Bus

Inspect and trace all agent-to-agent communications through a clear, transparent message bus that's logged to disk.

---

## Quick start

* pnpm install
* pnpm build
* crewchief --help  
  (or run without installing: `npx crewchief --help` or `pnpm dlx crewchief --help`)

## Core commands

* Initialization
  * `crewchief init` — prepare `.crewchief/`
  * `crewchief setup` — interactive config wizard
* Worktrees & session
  * `crewchief session start`
  * `crewchief worktree create <name> [--branch <branch>]`
  * `crewchief worktree list`
  * `crewchief worktree clean`
* Agents
  * `crewchief agent spawn <type> <task>`
  * `crewchief agent message <agentId> <message>`
  * `crewchief agent close <agentId>`
* Runs
  * `crewchief runs list`
  * `crewchief runs events <runId>`
  * `crewchief runs logs <runId> [--tail 200]`
* Evaluation & merge
  * `crewchief eval run <runId>`
  * `crewchief merge run <branch> [--target main] [--strategy squash|ff|cherry-pick]`
  * `crewchief merge auto <runId>`
* Tasks & competition
  * `crewchief task assign <agentTypeId> <description>`
  * `crewchief competition start <description> <agentIds...>`
  * `crewchief competition assign <competitionId>`
  * `crewchief competition evaluate <competitionId>`
  * `crewchief competition finalize <competitionId>`

## Notes

* ESM output, requires Node >= 18
* Tmux must be installed and on PATH
* Mock agent available via `mock-agent` for JSONL pipeline validation

### Install options

* Global install: `npm i -g crewchief` (or `pnpm add -g crewchief`), then run `crewchief ...`
* Run without install: `npx crewchief@latest ...` or `pnpm dlx crewchief@latest ...`
