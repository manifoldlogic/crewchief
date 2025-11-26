# MCPDB - MCP Server SQLite Support

## Project Overview

Enable the Maproom MCP server (`packages/maproom-mcp/`) to work with SQLite database backends, completing the SQLite integration started in VECSTORE and MAPCLI.

## Problem Statement

The MCP server currently assumes PostgreSQL as the only database backend. With MAPCLI enabling SQLite support in the Rust daemon, the TypeScript MCP layer needs updates to:

- Parse and validate `sqlite://` database URLs
- Auto-detect SQLite databases at default locations
- Run tests without PostgreSQL service containers
- Provide helpful error messages for SQLite-specific scenarios

## Proposed Solution

Add SQLite URL detection and handling to the MCP server:

1. **URL Parsing**: New `resolveDatabaseConfig()` function that returns backend type and validated URL
2. **Auto-Detection**: Check for `~/.maproom/maproom.db` as zero-config default
3. **Daemon Integration**: Pass SQLite URLs to daemon with appropriate validation
4. **Test Infrastructure**: SQLite-based test helpers for PostgreSQL-free testing

## Key Deliverables

- [ ] `DatabaseConfig` type with `type`, `url`, `path` fields
- [ ] SQLite URL parsing with path expansion (`~` handling)
- [ ] Auto-detection of default SQLite location
- [ ] SQLite integration tests
- [ ] CI job for SQLite tests

## Relevant Agents

| Agent | Role |
|-------|------|
| General TypeScript | URL parsing, daemon integration |
| integration-tester | Test infrastructure and integration tests |
| github-actions-specialist | CI workflow updates |
| verify-ticket | Acceptance criteria verification |
| commit-ticket | Conventional commit creation |

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition and research
- [Architecture](planning/architecture.md) - Solution design and data flow
- [Quality Strategy](planning/quality-strategy.md) - Test approach and criteria
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Phased execution plan

## Dependencies

### Prerequisites (Completed)
- **VECSTORE** - VectorStore trait with SQLite implementation
- **MAPCLI** - CLI and daemon SQLite support

### External
- Pre-indexed SQLite fixture at `crates/maproom/tests/fixtures/pre-indexed-maproom.db`

## Success Criteria

1. `MAPROOM_DATABASE_URL=sqlite:///path/to/db.sqlite` works in MCP server
2. `~/.maproom/maproom.db` auto-detected when no URL specified
3. MCP tools (`search`, `status`, `open`) return valid results with SQLite
4. Tests can run without PostgreSQL service container
5. All existing PostgreSQL tests continue to pass

## Known Limitations (SQLite Mode)

This project enables SQLite support with graceful degradation for features that require PostgreSQL:

| MCP Tool | SQLite Support | Limitation |
|----------|---------------|------------|
| `search` | Full | `chunk_id` field is 0 (warning logged) |
| `open` | Full | No limitations |
| `status` | Partial | Returns degraded response (no detailed stats) |

### Technical Details

- **chunk_id=0**: SQLite search results don't include database chunk IDs because the Rust daemon doesn't return them. This is logged as a warning. Chunk IDs are primarily used for internal correlation and don't affect search result quality.

- **Status tool degradation**: The status tool uses direct PostgreSQL queries for detailed statistics. In SQLite mode, it returns basic information with a hint guiding users to use the search tool.

### Full Feature Access

For full feature access including detailed statistics, use PostgreSQL:
```bash
export MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5433/maproom
```

## Timeline

**Estimated**: 2-3 days

## Tickets

See [tickets/](tickets/) directory for individual work items (to be created via `/create-project-tickets MCPDB`).

## Related Projects

- **VSCODEDB** - VSCode extension SQLite support (downstream)
- **SQLINFRA** - Infrastructure documentation updates (downstream)
