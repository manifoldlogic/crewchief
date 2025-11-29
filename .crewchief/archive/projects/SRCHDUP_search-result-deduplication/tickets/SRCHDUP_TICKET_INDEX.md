# SRCHDUP Ticket Index

**Project:** Search Result Deduplication
**Total Tickets:** 13
**Created:** 2025-11-26

## Overview

This index tracks all tickets for the SRCHDUP project, organized by phase.

## Phase 1: Core Implementation (2 tickets)

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [SRCHDUP-1001](SRCHDUP-1001_create-dedup-module.md) | Create dedup.rs module with ChunkIdentity and deduplicate() | rust-indexer-engineer | Not Started | None |
| [SRCHDUP-1002](SRCHDUP-1002_unit-tests-dedup-module.md) | Unit tests for dedup module | rust-indexer-engineer | Not Started | SRCHDUP-1001 |

## Phase 2: Pipeline Integration (4 tickets)

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [SRCHDUP-2001](SRCHDUP-2001_extend-searchoptions.md) | Extend SearchOptions with deduplicate flag | rust-indexer-engineer | Not Started | SRCHDUP-1001 |
| [SRCHDUP-2002](SRCHDUP-2002_pipeline-integration.md) | Integrate dedup into SearchPipeline | rust-indexer-engineer | Not Started | SRCHDUP-2001 |
| [SRCHDUP-2003](SRCHDUP-2003_cli-flag.md) | Add --deduplicate CLI flag to search command | rust-indexer-engineer | Not Started | SRCHDUP-2002 |
| [SRCHDUP-2004](SRCHDUP-2004_integration-tests.md) | Integration tests for pipeline dedup | integration-tester | Not Started | SRCHDUP-2002 |

## Phase 3: MCP Exposure (4 tickets)

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [SRCHDUP-3001](SRCHDUP-3001_daemon-client-update.md) | Update daemon-client SearchParams interface | vscode-extension-specialist | Not Started | SRCHDUP-2002 |
| [SRCHDUP-3002](SRCHDUP-3002_json-rpc-handler.md) | Update Rust daemon JSON-RPC handler for deduplicate | rust-indexer-engineer | Not Started | SRCHDUP-2002 |
| [SRCHDUP-3003](SRCHDUP-3003_mcp-schema-update.md) | Add deduplicate parameter to MCP search schema | vscode-extension-specialist | Not Started | SRCHDUP-3001, SRCHDUP-3002 |
| [SRCHDUP-3004](SRCHDUP-3004_mcp-e2e-tests.md) | MCP E2E tests for deduplication | integration-tester | Not Started | SRCHDUP-3003 |

## Phase 4: Documentation & Cleanup (3 tickets)

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [SRCHDUP-4001](SRCHDUP-4001_benchmarks.md) | Add dedup benchmarks | rust-indexer-engineer | Not Started | SRCHDUP-2002 |
| [SRCHDUP-4002](SRCHDUP-4002_documentation.md) | Update search documentation | technical-researcher | Not Started | SRCHDUP-3003 |
| [SRCHDUP-4003](SRCHDUP-4003_final-verification.md) | Final verification and cleanup | verify-ticket | Not Started | All previous |

## Critical Path

```
SRCHDUP-1001 → SRCHDUP-1002 → SRCHDUP-2001 → SRCHDUP-2002 → SRCHDUP-2003
                                                    ↓
                              SRCHDUP-3001 → SRCHDUP-3003 → SRCHDUP-3004
                              SRCHDUP-3002 ↗
                                                    ↓
                                             SRCHDUP-4002 → SRCHDUP-4003
```

## Execution Order

1. **SRCHDUP-1001** - Core module (no dependencies)
2. **SRCHDUP-1002** - Unit tests (after 1001)
3. **SRCHDUP-2001** - SearchOptions (after 1001)
4. **SRCHDUP-2002** - Pipeline integration (after 2001)
5. **SRCHDUP-2003** - CLI flag (after 2002)
6. **SRCHDUP-2004** - Integration tests (after 2002, can parallel with 2003)
7. **SRCHDUP-3001** - Daemon-client (after 2002)
8. **SRCHDUP-3002** - JSON-RPC handler (after 2002, can parallel with 3001)
9. **SRCHDUP-3003** - MCP schema (after 3001, 3002)
10. **SRCHDUP-3004** - MCP E2E tests (after 3003)
11. **SRCHDUP-4001** - Benchmarks (after 2002, can run anytime after)
12. **SRCHDUP-4002** - Documentation (after 3003)
13. **SRCHDUP-4003** - Final verification (last)

## Plan Reference

See [plan.md](../planning/plan.md) for detailed phase descriptions and success criteria.
