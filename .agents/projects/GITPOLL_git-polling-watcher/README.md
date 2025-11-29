# GITPOLL: Git Polling File Watcher

## Overview

Replace the notify-based file watcher with git status polling to eliminate "too many open files" errors on large repositories.

## Problem Statement

The current file watcher uses the `notify` crate with `RecursiveMode::Recursive`, which creates file descriptors for every watched directory. On repositories with deep directory structures (e.g., `node_modules`), this causes "Too many open files" (EMFILE) errors, making the watch command unusable.

## Proposed Solution

Replace native file watching with periodic `git status --porcelain` polling:

- **Zero file descriptors**: Git polling uses no persistent file handles
- **Platform-independent**: Same behavior on Linux, macOS, Windows
- **Git-aware**: Automatically respects `.gitignore`
- **Reliable**: No watcher state to corrupt
- **Acceptable latency**: 2-5 second polling interval is fine for development

## Relevant Agents

- **rust-indexer-engineer**: All implementation work (pure Rust in maproom crate)

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition, research, and approach selection
- [Architecture](planning/architecture.md) - Component design and integration strategy
- [Quality Strategy](planning/quality-strategy.md) - Testing approach and acceptance criteria
- [Security Review](planning/security-review.md) - Security analysis and mitigations
- [Plan](planning/plan.md) - Phased implementation plan

## Key Design Decisions

1. **Git status as change source**: Leverages git's efficient file tracking
2. **In-memory state**: Simple, fast, acceptable to lose on restart
3. **Preserve FileEvent interface**: Drop-in replacement, no downstream changes
4. **Configurable polling interval**: Default 3 seconds, adjustable

## Success Criteria

1. Zero "too many open files" errors on large repositories
2. File changes detected within configured polling interval
3. No regression in existing watch functionality
4. Works on all platforms (Linux, macOS, Windows)

## Status

**Phase**: Planning Complete

**Next Step**: Create tickets and begin implementation
