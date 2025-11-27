# Project: MCP Server Simplification

**Slug**: MCPSIMP
**Status**: Planning Complete
**Version**: 3.0.0 (breaking change)

## Summary

Transform `@crewchief/maproom-mcp` from a 2,000-line Docker orchestration tool into a ~50-line single-purpose MCP server with no subcommands.

**Before**:
```bash
npx @crewchief/maproom-mcp setup --provider=openai
npx @crewchief/maproom-mcp scan /path/to/repo
npx @crewchief/maproom-mcp  # Finally runs MCP server
```

**After**:
```bash
npx @crewchief/maproom-mcp  # Just runs MCP server
```

## Problem Statement

The MCP server package grew to include Docker orchestration, container management, setup commands, and complex configuration detection. This complexity:

1. **Wrong responsibility**: MCP servers should serve MCP protocol, not manage infrastructure
2. **Duplicated work**: VSCode extension already handles Docker
3. **Unusable feature**: Ollama is too slow to recommend
4. **Hard to maintain**: 2,000 lines of code for what should be 50 lines

## Proposed Solution

### Architecture

```
MCP Client → CLI (~50 lines) → MCP Server → Rust Daemon → PostgreSQL
```

Only PostgreSQL runs in a container. Everything else runs on host.

### Database Auto-Detection

```javascript
// 1. Explicit: MAPROOM_DATABASE_URL
// 2. DevContainer: IN_DEVCONTAINER=true → container hostname
// 3. Default: localhost:5433
```

### What's Removed

- 1,920 lines of Docker orchestration code
- Setup, scan, watch subcommands
- Ollama container management
- Complex configuration detection
- Multiple Dockerfiles

### What Remains

- ~50 line CLI entry point
- MCP server tool handlers
- Rust daemon integration
- Database connection

## Relevant Agents

| Agent | Role |
|-------|------|
| general-purpose | CLI replacement, file deletion, package.json updates |
| vscode-extension-specialist | MCP config writer, extension updates |
| verify-ticket | Manual verification checklist |
| commit-ticket | Version bump and release |

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition and research
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Execution phases and agent assignments

## Success Criteria

1. CLI reduced from 1,971 to ~50 lines
2. No subcommands - `npx @crewchief/maproom-mcp` runs server directly
3. Database auto-detection works for all scenarios
4. VSCode extension users unaffected
5. All existing tests pass
6. Version 3.0.0 published
