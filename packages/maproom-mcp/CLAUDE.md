# Maproom MCP

Single-purpose MCP server for semantic code search via stdio.

## MCP Tools

- `search` — Semantic search (FTS/vector/hybrid)
- `open` — Get code with line ranges
- `context` — Context assembly via daemon (imports, callers, tests, React components)
- `status` — Index stats
- `scan` — Full repo indexing (via daemon)
- `upsert` — Update specific files
- `explain` — Symbol documentation

## Pitfalls

- **CJS entry point**: `bin/cli.cjs` is the CLI entry (~50 lines), not the main source. Source is `src/index.ts`.
- **Daemon binary path**: Wraps `../../packages/cli/bin/<platform>/crewchief-maproom` — binary must exist
- **ESM modules** with Zod for MCP validation

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `DAEMON_START_FAILED` | Binary not found | Ensure crewchief-maproom built (`cd packages/cli && pnpm build:rust`) |
| `CHUNK_NOT_FOUND` | Invalid chunk_id | Use search tool to find valid chunk IDs |
| `CONTEXT_TIMEOUT` | Request too slow | Reduce budget_tokens or check database |
| `INVALID_PARAMS` | Bad parameters | Check chunk_id is positive integer |

## Docs

- Agent integration: `crates/maproom/docs/agent-usage.md`
- MCP tool reference: `docs/api/mcp-tools.md`
