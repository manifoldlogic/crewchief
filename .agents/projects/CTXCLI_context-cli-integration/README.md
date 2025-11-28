# CTXCLI: Context CLI Integration

## Overview

Expose the Rust context assembler via CLI command and JSON-RPC daemon, enabling the MCP server to use the unified SQLite-based context assembly instead of the legacy PostgreSQL implementation.

## Problem

The Rust context assembler (SQLIMPL Phase 4) is complete but not exposed:
- MCP `context` tool uses PostgreSQL directly
- Duplicates assembly logic in TypeScript
- Missing language-specific strategies (React hooks, JSX, etc.)

## Solution

Add a `context` method to the JSON-RPC daemon and update the MCP tool to use it:

```
MCP context tool → daemon client → Rust daemon → BasicContextAssembler → SQLite
```

## Scope

### In Scope
- CLI `context` command for standalone use
- Daemon `context` JSON-RPC method
- MCP tool update to use daemon
- Extended schema with React-specific options

### Out of Scope
- New context features (additional relationships)
- Performance optimization beyond caching
- UI/visualization of context bundles

## Structure

```
planning/
├── analysis.md       # Current state and gap analysis
├── architecture.md   # Technical design and data flow
├── quality-strategy.md # Testing approach
└── plan.md           # Implementation phases and tickets
tickets/
└── (to be generated)
```

## Tickets

| ID | Description | Phase |
|----|-------------|-------|
| CTXCLI-1001 | Context params types | 1: Daemon |
| CTXCLI-1002 | Daemon context handler | 1: Daemon |
| CTXCLI-1003 | DaemonState context support | 1: Daemon |
| CTXCLI-2001 | CLI context command variant | 2: CLI |
| CTXCLI-2002 | CLI context handler | 2: CLI |
| CTXCLI-2003 | Human-readable output | 2: CLI |
| CTXCLI-3001 | MCP context schema update | 3: MCP |
| CTXCLI-3002 | Replace PostgreSQL with daemon | 3: MCP |
| CTXCLI-3003 | Daemon client in MCP server | 3: MCP |
| CTXCLI-4001 | Daemon integration tests | 4: Testing |
| CTXCLI-4002 | CLI integration tests | 4: Testing |
| CTXCLI-4003 | MCP E2E tests | 4: Testing |
| CTXCLI-4004 | Documentation updates | 4: Testing |

## Success Criteria

- [ ] `crewchief-maproom context --chunk-id <id>` returns valid bundle
- [ ] MCP `context` tool uses daemon (no PostgreSQL)
- [ ] All expand options work (callers, callees, tests, hooks, jsx, etc.)
- [ ] Context assembly < 100ms (cached)
- [ ] All tests pass in CI

## Dependencies

- **SQLIMPL Phase 4** - Context assembler implementation (complete)
- **daemon-client** - TypeScript JSON-RPC client (exists)
- **SQLite backend** - Database (exists)

## Status

- [x] Analysis complete
- [x] Architecture defined
- [x] Quality strategy defined
- [x] Implementation plan created
- [ ] Tickets generated
- [ ] Implementation started
