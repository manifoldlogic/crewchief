# crewchief

Semantic code search and AI-assisted development toolkit.

![CI](https://github.com/manifoldlogic/crewchief/actions/workflows/test.yml/badge.svg)
![Release CLI](https://github.com/manifoldlogic/crewchief/actions/workflows/release-cli.yml/badge.svg)

## The CLI: worktrees, agents, and search in one tool

<p align="center">
  <img src="docs/images/crewchief_cli_hero.svg" alt="CrewChief CLI architecture — worktree management creates isolated branches, agent orchestration spawns and coordinates AI coding agents in iTerm2, and maproom integration provides semantic code search. All accessible through the crewchief command.">
</p>

The `crewchief` CLI brings together git worktree management, AI agent orchestration, and semantic code search. Create isolated worktrees for parallel development, spawn AI agents that work in their own terminal tabs, and search your codebase by meaning — all from one command.

```bash
npm install -g @crewchief/cli
crewchief doctor          # verify dependencies
crewchief worktree create feature-branch
crewchief maproom scan    # index your codebase
crewchief maproom search "authentication middleware"
```

See the [CLI README](packages/cli/README.md) for full installation, configuration, and command reference.

## The indexer: how maproom understands your code

<p align="center">
  <img src="docs/images/maproom_hero.svg" alt="Maproom indexer architecture — tree-sitter parses source files into AST chunks, embeddings encode semantic meaning via Ollama or OpenAI, SQLite stores chunks with full-text and vector indexes, and hybrid search combines lexical and semantic ranking with recency and churn signals.">
</p>

`maproom` is the Rust binary that powers crewchief's code search. It parses your codebase with tree-sitter (15 languages), stores chunks in a local SQLite database, and runs hybrid search combining full-text matching with vector similarity. Results are ranked using lexical relevance, semantic similarity, git recency, and churn signals.

```bash
cargo install maproom
maproom scan --repository .
maproom search "error handling in API routes"
```

Full-text search works immediately after indexing. Add an embedding provider (Ollama for local/free, OpenAI for hosted) to unlock vector and hybrid search modes. See the [maproom README](crates/maproom/README.md) for provider configuration.

## Getting Started

### 1. Install

```bash
# CLI (recommended — includes the maproom binary)
npm install -g @crewchief/cli

# Or install the Rust binary directly
cargo install maproom
```

### 2. Index your code

```bash
crewchief maproom scan                    # full index
crewchief maproom watch                   # auto-index on file changes
crewchief maproom generate-embeddings     # enable vector search (requires Ollama or OpenAI)
```

### 3. Search

```bash
crewchief maproom search "database connection pooling"
```

### 4. Set up worktrees (optional)

```bash
crewchief worktree create feature-branch  # isolated working copy
crewchief worktree list                   # see all worktrees
crewchief worktree merge feature-branch   # merge back and clean up
```

### 5. Spawn AI agents (optional, requires iTerm2 on macOS)

```bash
crewchief spawn claude "implement the caching layer"
crewchief agent list
crewchief agent message claude "also add cache invalidation"
```

## Supported Languages

maproom parses these languages with tree-sitter for symbol-level chunking:

TypeScript, JavaScript, Python, Rust, Go, Ruby, C, C++, C#, Java, Markdown, JSON, YAML, TOML

## Spec-Driven Development Plugin

crewchief ships with an SDD plugin for Claude Code that adds structured project workflow management. It decomposes work into epics, tickets, and tasks — each with planning documents, acceptance criteria, and verification gates.

Governance-inspired features for teams that care about traceability:

- **Decision audit trails** — every task carries a verification audit table logging who verified what, when, and the outcome
- **Workflow event logging** — task completions, ticket milestones, and verification outcomes are appended to a timestamped `workflow.log`
- **Mandatory verification gates** — no task can be committed without passing acceptance criteria checks by a dedicated verification agent
- **12-section code review** — automated post-implementation review with confidence scoring, security analysis, and categorized recommendations
- **Structured planning artifacts** — analysis, architecture, PRD, quality strategy, and security review documents created before any code is written

This is not a heavyweight process tool. It runs inside Claude Code as slash commands (`/sdd:plan-ticket`, `/sdd:do-task`, `/sdd:code-review`) and produces markdown files that live in your repo.

## Monorepo Layout

| Package / Crate | Description | Docs |
| --- | --- | --- |
| [`packages/cli`](packages/cli/) | TypeScript CLI — worktrees, agents, search | [README](packages/cli/README.md) |
| [`crates/maproom`](crates/maproom/) | Rust indexer — tree-sitter parsing, embeddings, SQLite | [README](crates/maproom/README.md) |
| [`packages/daemon-client`](packages/daemon-client/) | Daemon RPC client library | [README](packages/daemon-client/README.md) |

### Deprecated

- **`@crewchief/maproom-mcp`** — Standalone MCP server. Use [`@crewchief/cli`](https://www.npmjs.com/package/@crewchief/cli) instead, which wraps the maproom binary directly.
- **`vscode-maproom`** — VS Code extension. Use [`@crewchief/cli`](https://www.npmjs.com/package/@crewchief/cli) and the [`maproom`](https://crates.io/crates/maproom) binary directly.

These packages remain in the repository for reference but are not receiving new features or bug fixes.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the [MIT License](LICENSE).
