# MCPSIMP Ticket Index

**Project:** MCP Server Simplification
**Created:** 2025-11-25
**Total Tickets:** 13

## Overview

Transform `@crewchief/maproom-mcp` from a ~2,000-line Docker orchestration tool into a ~50-line single-purpose MCP server.

## Ticket Summary by Phase

### Phase 1: Core Simplification (MCP Package)
| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| MCPSIMP-1001 | Replace CLI Entry Point | general-purpose | Not Started |
| MCPSIMP-1002 | Delete Unused Files | general-purpose | Not Started |
| MCPSIMP-1003 | Update Package.json | general-purpose | Not Started |

### Phase 2: VSCode Extension Updates
| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| MCPSIMP-2001 | Update MCP Config Writer | vscode-extension-specialist | Not Started |
| MCPSIMP-2002 | Update Version Constant | general-purpose | Not Started |
| MCPSIMP-2003 | Update Extension docker-compose.yml | vscode-extension-specialist | Not Started |
| MCPSIMP-2004 | Update DockerManager Service Startup | vscode-extension-specialist | Not Started |
| MCPSIMP-2005 | Update MCP Writer Tests | general-purpose | Not Started |

### Phase 3: Documentation & Testing
| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| MCPSIMP-3001 | Update CLAUDE.md | general-purpose | Not Started |
| MCPSIMP-3002 | Write resolveDatabase Unit Tests | general-purpose | Not Started |
| MCPSIMP-3003 | Manual Verification | verify-ticket | Not Started |

### Phase 4: Release
| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| MCPSIMP-4001 | Update README | general-purpose | Not Started |
| MCPSIMP-4002 | Final Version Verification | general-purpose | Not Started |

## Dependency Graph

```
Phase 1 (Sequential - Critical Dependencies):
  MCPSIMP-1001 → MCPSIMP-1002 → MCPSIMP-1003
       ↓              ↓
  [cli.cjs must be replaced before files deleted]

Phase 2 (Mostly Parallel):
  MCPSIMP-2001 ──────────────────→ MCPSIMP-2005
  MCPSIMP-2002 (after Phase 1)
  MCPSIMP-2003 → MCPSIMP-2004

Phase 3 (After Phase 1 & 2):
  MCPSIMP-3001
  MCPSIMP-3002 (needs MCPSIMP-1001)
  MCPSIMP-3003 (after all above)

Phase 4 (Final):
  MCPSIMP-4001 → MCPSIMP-4002
```

## Critical Path

The following tickets are on the critical path and should be prioritized:

1. **MCPSIMP-1001** - Replace CLI (blocks all other Phase 1)
2. **MCPSIMP-1002** - Delete files (blocks Phase 1 completion)
3. **MCPSIMP-2003** - Update docker-compose (blocks MCPSIMP-2004)
4. **MCPSIMP-3003** - Manual verification (gates release)
5. **MCPSIMP-4002** - Final verification (last step before publish)

## Execution Notes

### Parallel Execution Opportunities
- Phase 2 can start while Phase 1 is in progress (except version constant)
- MCPSIMP-2001 and MCPSIMP-2003 can be done in parallel
- MCPSIMP-3001 and MCPSIMP-3002 can be done in parallel

### Publishing Sequence
Per plan.md coordination notes:
1. Complete all Phase 1-3 tickets
2. Publish `@crewchief/maproom-mcp@3.0.0` to npm
3. Update extension version constant (MCPSIMP-2002)
4. Publish extension update

### Rollback Plan
If issues discovered after publishing:
- MCP Package: `npm deprecate @crewchief/maproom-mcp@3.0.0 "Breaking issues discovered"`
- Extension: Revert MAPROOM_MCP_VERSION to '2.2.3', publish patch

## Plan Traceability

| Plan Section | Tickets |
|--------------|---------|
| Phase 1.1 Replace CLI Entry Point | MCPSIMP-1001 |
| Phase 1.2 Delete Unused Files | MCPSIMP-1002 |
| Phase 1.3 Update Package.json | MCPSIMP-1003 |
| Phase 2.1 Update MCP Config Writer | MCPSIMP-2001 |
| Phase 2.2 Update Version Constant | MCPSIMP-2002 |
| Phase 2.3 Update docker-compose.yml | MCPSIMP-2003 |
| Phase 2.4 Update DockerManager | MCPSIMP-2004 |
| Phase 2.5 Update MCP Writer Tests | MCPSIMP-2005 |
| Phase 3.1 Update CLAUDE.md | MCPSIMP-3001 |
| Phase 3.2 Write Unit Tests | MCPSIMP-3002 |
| Phase 3.3 Manual Verification | MCPSIMP-3003 |
| Phase 4.1 Update README | MCPSIMP-4001 |
| Phase 4.2 Version Bump | MCPSIMP-4002 |

## Success Criteria

From plan.md:
1. CLI reduced to ~50 lines (from 1,971)
2. `npx @crewchief/maproom-mcp` runs MCP server directly (no subcommands)
3. Database auto-detection works for all three scenarios
4. VSCode extension unchanged for users
5. All existing tests pass
6. Version 3.0.0 published
