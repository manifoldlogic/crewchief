# CTXCLI Ticket Index

## Project: Context CLI Integration

Expose the Rust context assembler via CLI command and JSON-RPC daemon, enabling the MCP server to use the unified SQLite-based context assembly.

## Ticket Summary

**Status:** Complete
**Completion Date:** 2025-11-28

| ID | Title | Status | Phase | Effort |
|----|-------|--------|-------|--------|
| [CTXCLI-1001](./CTXCLI-1001_context-params-types.md) | Add Context Params Types | ✅ Complete | 1: Daemon | S |
| [CTXCLI-1002](./CTXCLI-1002_daemon-context-handler.md) | Daemon Context Handler + State Support | ✅ Complete | 1: Daemon | M |
| [CTXCLI-2001](./CTXCLI-2001_cli-context-command.md) | CLI Context Command Variant | ✅ Complete | 2: CLI | S |
| [CTXCLI-2002](./CTXCLI-2002_cli-context-handler.md) | CLI Context Handler | ✅ Complete | 2: CLI | M |
| [CTXCLI-2003](./CTXCLI-2003_human-readable-output.md) | Human-Readable Output Format | ✅ Complete | 2: CLI | S |
| [CTXCLI-3001](./CTXCLI-3001_mcp-context-schema.md) | Update MCP Context Schema | ✅ Complete | 3: MCP | S |
| [CTXCLI-3002](./CTXCLI-3002_replace-postgresql-daemon.md) | Replace PostgreSQL with Daemon Client | ✅ Complete | 3: MCP | M |
| [CTXCLI-3003](./CTXCLI-3003_daemon-client-mcp-server.md) | Daemon Client in MCP Server | ✅ Complete | 3: MCP | S |
| [CTXCLI-4001](./CTXCLI-4001_daemon-integration-tests.md) | Daemon Context Integration Tests | ✅ Complete | 4: Testing | M |
| [CTXCLI-4002](./CTXCLI-4002_cli-integration-tests.md) | CLI Context Integration Tests | ✅ Complete | 4: Testing | M |
| [CTXCLI-4003](./CTXCLI-4003_mcp-e2e-tests.md) | MCP Context E2E Tests | ✅ Complete | 4: Testing | M |
| [CTXCLI-4004](./CTXCLI-4004_documentation-updates.md) | Documentation Updates | ✅ Complete | 4: Testing | S |

**Total: 12 tickets** (5 Small, 7 Medium) - All Complete

## Execution Order

### Phase 1: Daemon Foundation
1. **CTXCLI-1001** → CTXCLI-1002

### Phase 2: CLI Context Command
2. **CTXCLI-2001** → CTXCLI-2002 → CTXCLI-2003

### Phase 3: MCP Integration
3. **CTXCLI-3001** → CTXCLI-3002 → CTXCLI-3003

### Phase 4: Testing & Documentation
4. **CTXCLI-4001**, **CTXCLI-4002**, **CTXCLI-4003** (can run in parallel)
5. **CTXCLI-4004** (after all implementation complete)

## Dependency Graph

```
CTXCLI-1001 ──► CTXCLI-1002 ──┬──► CTXCLI-4001
                              │
CTXCLI-2001 ──► CTXCLI-2002 ──► CTXCLI-2003 ──► CTXCLI-4002
                              │
CTXCLI-3001 ──┬──► CTXCLI-3002 ──► CTXCLI-3003 ──► CTXCLI-4003
              │        ▲
              │        │
              └────────┘ (CTXCLI-1002)
                              │
                              ▼
                        CTXCLI-4004
```

## Plan Reference

See [planning/plan.md](../planning/plan.md) for full implementation details.

## Success Criteria

- [x] `crewchief-maproom context --chunk-id <id>` returns valid bundle
- [x] MCP `context` tool uses daemon (no PostgreSQL)
- [x] All expand options work (callers, callees, tests, hooks, jsx, etc.)
- [x] Context assembly < 100ms (cached)
- [x] All tests pass in CI

---

✅ **Project Complete** - All tickets verified and committed (2025-11-28)
