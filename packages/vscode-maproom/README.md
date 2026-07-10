# Maproom Semantic Search (RETIRED)

> **RETIRED — This extension is no longer maintained and should not be installed by new users.**
>
> All capabilities (search, status, context — SQLite and PostgreSQL) are now covered by
> [`@crewchief/maproom-mcp`](../maproom-mcp/README.md), which works with any MCP-aware editor
> including VS Code. See [Migration to maproom-mcp](#migration-to-maproom-mcp) below.
>
> The source tree is kept in-tree for history and a future revive path. No code has been deleted.

---

## Migration to maproom-mcp

Replace this extension with the `@crewchief/maproom-mcp` MCP server.

### VS Code / Cursor

Add `.vscode/mcp.json` to your workspace (or user-level MCP config):

```json
{
  "servers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_DATABASE_URL": "sqlite://~/.maproom/maproom.db",
        "MAPROOM_EMBEDDING_PROVIDER": "ollama"
      }
    }
  }
}
```

For PostgreSQL or OpenAI/Google embedding providers, see the full configuration reference in
[`packages/maproom-mcp/README.md`](../maproom-mcp/README.md).

### Claude Code / other MCP-capable editors

Add the same `maproom` server block to your editor's MCP settings file. The `@crewchief/maproom-mcp`
package is published to npm and supports stdio transport, so it works wherever MCP is supported.

---

## Historical information (as of 0.4.x, now retired)

The sections below document what the extension did. They are kept for reference only.

### What it did

- Auto-indexed workspace files into a local SQLite database
- Watched for file changes and re-indexed incrementally
- Exposed search via VS Code Command Palette
- Spawned the Maproom daemon and MCP server internally

### Why it was retired

- Hardwired to a single SQLite database and `workspaceFolders[0]` — incompatible with the
  shared-PostgreSQL, multi-repo architecture shipped in the maproom ecosystem (2026).
- Every capability is now covered by `@crewchief/maproom-mcp` working natively in any
  MCP-capable editor without a VS Code extension wrapper.
- Was self-described as `[DEPRECATED]` in its own metadata and not installed in the
  development environment.

### Platform support (historical)

| Platform              | Status          |
| --------------------- | --------------- |
| macOS (Apple Silicon) | Was supported   |
| macOS (Intel)         | Was supported   |
| Linux (x64)           | Was supported   |
| Linux (arm64)         | Was supported   |
| Windows (x64)         | Was experimental|

### Settings (historical)

| Setting                       | Description          | Default                  |
| ----------------------------- | -------------------- | ------------------------ |
| `maproom.database.sqlitePath` | Custom database path | `~/.maproom/maproom.db`  |
| `maproom.ollama.endpoint`     | Ollama API URL       | `http://127.0.0.1:11434` |

---

## Support

- **Issues**: https://github.com/manifoldlogic/crewchief/issues
- **maproom-mcp**: [`packages/maproom-mcp`](../maproom-mcp/README.md)

## License

MIT License
