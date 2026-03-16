# CLAUDE.md

######## IMPORTANT! SHELL TARGET: ZSH ######## IMPORTANT! SHELL TARGET: ZSH ########
All commands execute in ZSH. Use POSIX-compatible syntax. Never use bash-only syntax.
Avoid: $RANDOM, [[ ]], bash arrays, `which`. Use: command -v, [ ], grep -E, portable syntax.
######## IMPORTANT! SHELL TARGET: ZSH ######## IMPORTANT! SHELL TARGET: ZSH ########

## Key Concepts

**Two CLIs**: `crewchief` (TypeScript) for worktree/agent management, `maproom` (Rust) for indexing/search/daemon.

**"Worktree" overloading**: Git worktrees are filesystem checkouts. Maproom worktrees are database records tracking indexed branches (1:1 with branches, not git worktrees).

**Release order**: CLI → maproom-mcp → vscode-maproom (CLI contains Rust binaries others depend on). See `release-config.json`.

**Type sync**: Rust is source of truth. See `.claude/docs/type-sync-workflow.md`.

**Runtime deps**: Git required (file watching uses `git status`). sqlite-vec is statically linked (vector search degrades gracefully if missing).

## Component Index

| Path | What | CLAUDE.md |
|------|------|-----------|
| `packages/cli/` | TypeScript CLI | `packages/cli/CLAUDE.md` |
| `packages/daemon-client/` | Daemon RPC client | `packages/daemon-client/CLAUDE.md` |
| `packages/maproom-mcp/` | MCP server | `packages/maproom-mcp/CLAUDE.md` |
| `packages/vscode-maproom/` | VSCode extension | `packages/vscode-maproom/CLAUDE.md` |
| `crates/maproom/` | Rust indexer | `crates/maproom/CLAUDE.md` |
| `crates/maproom/migrations/` | Database migrations | `crates/maproom/migrations/CLAUDE.md` |
| `.github/` | CI/CD | `.github/CLAUDE.md` |
| `.devcontainer/` | Dev container | `.devcontainer/CLAUDE.md` |
| `docs/` | Documentation | `docs/CLAUDE.md` |
| `packages/cli/src/search-optimization/` | Search tuning | `packages/cli/src/search-optimization/CLAUDE.md` |

Key docs: `docs/architecture/MAPROOM_ARCHITECTURE.md`, `crates/maproom/docs/agent-usage.md`, `docs/troubleshooting/common-errors.md`

## Git Workflow

**CRITICAL**: Always sync with origin before committing.

```bash
git fetch origin main
git rebase origin/main  # Before any commit
```

This prevents divergent branches when CI pushes between your changes.

## LSP Tools

Prefer LSP over Grep/Glob for semantic code navigation in Rust files:

- **Refactoring**: `LSP findReferences` — real usages only, no false positives from comments/strings
- **Trait/interface work**: `LSP goToImplementation` — find all trait impls
- **Type inspection**: `LSP hover` — resolved types and docs
- **File overview**: `LSP documentSymbol` — all symbols in a file
- **Definition lookup**: `LSP goToDefinition` — precise navigation

Typical flow: Grep to locate a symbol, then LSP for semantic queries.

**Known limitation**: `incomingCalls`/`outgoingCalls` don't return data with rust-analyzer. Use `findReferences` instead.

## Safety Rules

**File operations must stay within current worktree.**

Never modify: system directories (`/usr/`, `/etc/`), home files outside worktree, other repositories/worktrees, `.git` directory.

If external modification seems needed: STOP, explain, wait for approval.
