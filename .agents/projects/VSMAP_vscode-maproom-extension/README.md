# VSCode Maproom Extension

**Project ID:** VSMAP
**Status:** Planning
**Location:** `packages/vscode-maproom/`

## Overview

A VSCode/Cursor extension that provides automatic code indexing for Maproom's semantic search capabilities. This extension manages the indexing lifecycle (scan, watch, update) and Docker service orchestration, making codebases continuously ready for MCP-based semantic search.

## Problem Statement

Developers using Maproom MCP for semantic code search must manually:
- Run setup commands to configure embedding providers
- Start Docker services (PostgreSQL + optional Ollama)
- Execute initial repository scans
- Remember to re-scan after branch switches
- Monitor index freshness

This friction prevents Maproom from reaching its potential as an "always-ready" search engine.

## Proposed Solution

A VSCode extension that:
- **Automates indexing:** Scans on workspace open, watches files and branch switches
- **Manages services:** Auto-starts/stops Docker containers with manual override
- **Simplifies setup:** Guided wizard for provider selection and credential configuration
- **Provides visibility:** Status bar integration showing index health

**Key principle:** Search happens via MCP (Claude/Cursor), extension handles indexing only.

## Scope

### In Scope (MVP)
- Automatic repository scanning on workspace open
- File change watching with debounced updates
- Branch switch detection and incremental re-indexing
- Docker lifecycle management (auto-start with manual override)
- Provider configuration wizard (Ollama/OpenAI/Google Vertex AI)
- Status bar item with index status
- Development installation documentation

### Out of Scope
- Search UI within VSCode (use MCP instead)
- Multi-workspace support
- Custom embedding model configuration
- Search result caching
- Marketplace publishing (phase 1)

## Technology Stack

- **Language:** TypeScript
- **Platform:** VSCode Extension API
- **Indexing:** Spawns `crewchief-maproom` Rust binary
- **Docker:** docker-compose via child process
- **Storage:** VSCode workspace settings + Secrets API

## Relevant Agents

- **TypeScript/VSCode Agent:** Extension implementation, API integration
- **Docker Engineer:** Service orchestration, health checks
- **Technical Writer:** Installation and usage documentation
- **Test Engineer:** Integration testing for indexing workflows

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space research and user needs
- [Architecture](planning/architecture.md) - Technical design and MVP scope
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security considerations
- [Agent Suggestions](planning/agent-suggestions.md) - Specialized agents needed
- [Execution Plan](planning/plan.md) - Phase-based implementation roadmap

## Success Criteria

1. ✅ Extension installs and activates in VSCode
2. ✅ Setup wizard configures provider and credentials
3. ✅ Docker services start automatically on workspace open
4. ✅ Repository scans complete successfully
5. ✅ File changes trigger index updates within 5 seconds
6. ✅ Branch switches trigger incremental re-indexing
7. ✅ Status bar reflects accurate index state
8. ✅ MCP searches return results from extension-managed index
9. ✅ Works identically in local and devcontainer environments
10. ✅ Development installation documented with clear steps
