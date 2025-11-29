# MULTICN: Multi-Agent Concurrency for Maproom

## Project Summary

Enable multiple Claude Code agents to use maproom simultaneously without write contention, primarily when working on different git worktrees. Currently, each MCP client spawns its own daemon process, causing N daemons to compete for SQLite write locks at the OS file level. This results in SQLITE_BUSY failures when multiple agents index or search concurrently.

## Problem Statement

- **Write contention**: N agents = N daemons = N competing SQLite writers
- **Memory waste**: Each daemon uses ~100MB (N × 100MB for N agents)
- **Unreliable indexing**: SQLITE_BUSY errors with 5s timeout cause failures
- **No daemon sharing**: Each MCP session starts its own isolated process

## Proposed Solution

**Shared Daemon via Unix Socket Server**

Convert the maproom daemon from stdin/stdout communication (one per MCP client) to a Unix socket server that multiple clients share. This serializes writes at the application level rather than relying on OS file locks.

**Key Benefits:**
- Multiple agents share ONE daemon → writes serialized at application level
- Memory: ~100MB shared vs ~100MB per agent
- WAL mode enables concurrent reads while writes queue
- No cross-worktree database complexity needed

## Phases

1. **SQLite Foundation** - Enhanced PRAGMA config, retry logic, configurable pools
2. **Shared Daemon Architecture** - Unix socket server, connect-or-spawn client

## Primary Agents

- `rust-indexer-engineer` - Rust daemon socket server implementation
- `process-management-specialist` - Process lifecycle, PID files, signals
- `vscode-extension-specialist` - TypeScript client socket handling

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space and research
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Execution phases and tickets
