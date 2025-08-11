# Vision and Principles

## North Star

Turn the terminal into a command bridge where multiple AI agents each work in their own tmux stations, collaborating, competing, and communicating through a transparent channel, all under your direction.

- Coordinate or compete: agents can hand off work or run tournaments
- Benchmark and improve: repeatable scenarios and metrics to raise quality
- Observe everything: a transparent message bus and per-run logs
- Shape the flow: a central realm defines meaning ontology and semantic retrieval

## Product Principles

- Transparency first: every message and action is inspectable and replayable
- Isolation by default: each agent gets its own git worktree, pane, and vector index
- Composability: plug in agent types, evaluation metrics, and storage backends
- Non‑interactive defaults: all commands usable in scripts; prompts are optional
- Portability: Node ≥ 18, tmux, git; avoids heavyweight infra by default

## What We Will Add Next

1) Realm & Semantic Retrieval

- Meaning ontology describing entities/tasks/artifacts
- Embedding provider abstraction and pluggable vector index
- Per‑agent and global indexes sourced from the repo and bus transcripts
- CLI: `realm build`, `realm query`

2) Cross‑Agent Input Injection

- Safe, policy‑controlled keystroke routing between panes
- CLI: `agent inject <from> <to> "<keys>" [--enter]`
- Bus events for traceability and auditing

3) Benchmarking & Tournaments

- Scenario definitions with tasks, fixtures, and metrics
- Batch execution across agent sets with comparable scoring
- CLI: `eval benchmark <scenario> [--agents a,b]`

4) Observability Upgrades

- Rich JSONL envelopes with correlation ids, worktree context, and phases
- `runs logs --tail`, `runs events` improved summaries

## Glossary

- Realm: central module that manages ontology, embeddings, and vector indexes
- Ontology: typed graph of domain entities (tasks, files, runs, artifacts)
- Vector Index: search structure for semantic retrieval; per‑agent and global
- Agent Pane: dedicated tmux pane where an agent operates
- Run: lifecycle instance capturing all events and artifacts for a unit of work
- Bus: message/event fabric used by orchestrator, agents, and tools
