# CLAUDE.md

######## IMPORTANT! SHELL TARGET: ZSH ######## IMPORTANT! SHELL TARGET: ZSH ########
All commands execute in ZSH. Use POSIX-compatible syntax. Never use bash-only syntax.
Avoid: $RANDOM, [[ ]], bash arrays, `which`. Use: command -v, [ ], grep -E, portable syntax.
######## IMPORTANT! SHELL TARGET: ZSH ######## IMPORTANT! SHELL TARGET: ZSH ########

Guidance for Claude Code when working with this repository.

## Project Overview

CrewChief is a CLI tool combining:

- **Git worktree management** - Create, list, and manage git worktrees
- **Semantic code search (Maproom)** - Index and search code using SQLite and tree-sitter

## Key Concepts

**Two CLIs**: `crewchief` (TypeScript) for worktree/agent management, `crewchief-maproom` (Rust) for indexing/search/daemon.

**"Worktree" overloading**: Git worktrees are filesystem checkouts. Maproom worktrees are database records tracking indexed branches (1:1 with branches, not git worktrees).

**Release order**: CLI → maproom-mcp → vscode-maproom (CLI contains Rust binaries others depend on). See `release-config.json`.

**Type sync**: TypeScript types in `packages/daemon-client/src/client.ts` must match Rust structs in `crates/maproom/src/daemon/types.rs`. Rust is source of truth.

**Runtime deps**: Git required (file watching uses `git status`). sqlite-vec is statically linked (vector search degrades gracefully if missing).

## Quick Start

```bash
pnpm install && pnpm build   # Install and build
pnpm test                    # Run tests
pnpm lint && pnpm format     # Code quality
```

## Component Documentation

Each component has its own CLAUDE.md with detailed guidance:

- **`/packages/cli/CLAUDE.md`** - TypeScript CLI
- **`/packages/daemon-client/CLAUDE.md`** - Daemon client, type sync
- **`/packages/maproom-mcp/CLAUDE.md`** - MCP server
- **`/packages/vscode-maproom/CLAUDE.md`** - VSCode extension
- **`/crates/maproom/CLAUDE.md`** - Rust indexer
- **`.crewchief/CLAUDE.md`** - Project workflow and tickets
- **`.github/CLAUDE.md`** - CI/CD workflows

**Read the component's CLAUDE.md before working in it.**

## Development Practices

**Database**: SQLite at `~/.maproom/maproom.db` (override: `MAPROOM_DATABASE_URL`).

**New libraries**: Check if alternatives exist in codebase first. Be pragmatic. For major decisions, present top choices with reasoning during planning.

## Git Workflow

**CRITICAL**: Always sync with origin before committing.

```bash
git fetch origin main
git rebase origin/main  # Before any commit
```

This prevents divergent branches when CI pushes between your changes.

## LSP Tools

Prefer the LSP tool over Grep/Glob for semantic code navigation in Rust files:

- **Refactoring**: Use `LSP findReferences` instead of Grep — returns only real usages, no false positives from comments or strings
- **Trait/interface work**: Use `LSP goToImplementation` to find all trait impls — Grep cannot do this reliably
- **Type inspection**: Use `LSP hover` to check resolved types and docs without reading surrounding code
- **File overview**: Use `LSP documentSymbol` to list all symbols in a file instead of skimming manually
- **Definition lookup**: Use `LSP goToDefinition` for precise navigation to where a symbol is defined

LSP requires file + line + column. Typical flow: use Grep to locate a symbol, then use LSP for semantic queries about it.

**Known limitations**: `incomingCalls` and `outgoingCalls` do not return data with rust-analyzer. Use `findReferences` as the alternative for finding callers.

## Safety Rules

**File operations must stay within current worktree.**

Never modify:

- System directories (`/usr/`, `/etc//`)
- Home files outside worktree (`~/.bashrc`, `~/.gitconfig`)
- Other repositories or worktrees
- `.git` directory

If external modification seems needed: STOP, explain, wait for approval.
