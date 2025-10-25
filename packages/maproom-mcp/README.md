## Maproom MCP Server

Maproom is a code-aware indexing and retrieval layer for multi-agent work. This package provides the MCP (Model Context Protocol) server used by editors like Cursor to search and open code using Maproom's Postgres-backed index. It also orchestrates indexing by locating and executing the Rust indexer binary (`crewchief-maproom`).

### Part of the CrewChief Ecosystem

Maproom MCP works in concert with [CrewChief CLI](https://www.npmjs.com/package/crewchief), a comprehensive tool for git worktree management and code indexing. While this MCP server enables AI assistants to search your code, CrewChief provides:

- **Full CLI commands** for indexing, searching, and managing your codebase
- **Git worktree management** for isolated development environments  
- **Agent orchestration** for multi-agent AI workflows
- **Integrated build tools** for the Maproom indexer

You can use maproom-mcp standalone with any PostgreSQL database, or as part of a complete CrewChief setup for advanced multi-agent development workflows.

### Features

- **MCP server over stdio**: `search`, `open`, `upsert` tools
- **PostgreSQL-backed storage** with FTS and worktree scoping
- **Seamless Rust indexer integration**
  - Uses bundled platform binary when available
  - Falls back to local Cargo build during install
  - Respects explicit `CREWCHIEF_MAPROOM_BIN` path

### Requirements

- **Node.js**: 18+ (tested with Node 20+)
- **PostgreSQL**: 14+ reachable via `DATABASE_URL`
- **Rust toolchain**: optional; only needed if a prebuilt indexer binary isn’t bundled for your platform

### Install

You can run ad-hoc with `npx` or install globally.

```bash
# Run ad-hoc
npx maproom-mcp

# Or install globally
npm i -g maproom-mcp
maproom-mcp
```

On install, the package will:

- Prefer a bundled binary at `bin/<platform>-<arch>/crewchief-maproom`
- If not found, try to build `crewchief-maproom` with `cargo build --release`
- Ensure the binary is executable

You can also point to a custom binary using `CREWCHIEF_MAPROOM_BIN`.

### Environment Variables

- **`DATABASE_URL`** (required): Connection string for Postgres
  - Example: `postgres://user:password@localhost:5432/dbname`
  - Tip: If your password contains `@`, percent-encode it (e.g., `@` → `%40`)
- **`CREWCHIEF_MAPROOM_BIN`** (optional): Absolute path to the Rust indexer binary
- **`LOG_LEVEL`** (optional): pino log level (`info`, `debug`, etc.). Logs go to stderr

### Client Integration

Maproom MCP server integrates with MCP-compatible clients including Claude Desktop, Cursor, and VS Code. Complete configuration examples and usage guides are available in the documentation:

- **[Claude Desktop Configuration](examples/claude_desktop_config.json)**: Full setup with environment variables and troubleshooting
- **[VS Code Configuration](examples/vscode_config.json)**: Workspace and user settings with MCP extension
- **[Usage Patterns Guide](docs/usage_patterns.md)**: Comprehensive patterns from beginner to advanced
- **[Example Workflows](docs/examples.md)**: Step-by-step workflows for common tasks

#### Quick Start: Claude Desktop

Configuration file location:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

Example configuration:
```json
{
  "mcpServers": {
    "maproom": {
      "command": "node",
      "args": ["/absolute/path/to/maproom-mcp/dist/index.js"],
      "env": {
        "DATABASE_URL": "postgresql://user:password@localhost:5432/maproom",
        "LOG_LEVEL": "info"
      }
    }
  }
}
```

Restart Claude Desktop after saving. See [examples/claude_desktop_config.json](examples/claude_desktop_config.json) for detailed setup instructions.

#### Quick Start: VS Code

Install the MCP extension from the marketplace, then add to your settings.json:

```json
{
  "mcp.servers": {
    "maproom": {
      "command": "node",
      "args": ["/absolute/path/to/maproom-mcp/dist/index.js"],
      "env": {
        "DATABASE_URL": "postgresql://user:password@localhost:5432/maproom"
      }
    }
  }
}
```

Reload the VS Code window (Cmd/Ctrl+Shift+P > Reload Window). See [examples/vscode_config.json](examples/vscode_config.json) for detailed setup.

#### Quick Start: Cursor (MCP)

Add to Cursor's MCP configuration (`Settings → MCP Servers`):

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["maproom-mcp"],
      "env": {
        "DATABASE_URL": "postgres://USER:PASSWORD@localhost:5432/DB",
        "CREWCHIEF_MAPROOM_BIN": "/absolute/path/to/crewchief-maproom"
      }
    }
  }
}
```

Then restart Cursor. You should see the Maproom tools enabled.

#### Available Tools

All clients have access to these MCP tools:

- **`status`**: Check index status (repos, worktrees, stats)
- **`search`**: Semantic code search with filters
- **`open`**: View file contents with line ranges
- **`context`**: Get related code (callers, callees, tests)
- **`upsert`**: Update the index for specific files
- **`explain`**: Generate detailed symbol documentation (experimental)

See the [Usage Patterns Guide](docs/usage_patterns.md) for detailed tool documentation and the [Examples](docs/examples.md) for step-by-step workflows.

### Tools

- **`search`**
  - Inputs:
    - `repo` (required): Repository name to search in
    - `worktree` (optional): Worktree name to scope results
    - `query` (required): Search query (1-3 words work best)
    - `k` (optional, default: 10): Number of results to return
    - `mode` (optional, default: "hybrid"): Search mode - "fts", "vector", or "hybrid"
    - `filter` (optional, default: "all"): File type filter - "all", "code", "docs", or "config"
    - `filters` (optional): Advanced filters object with:
      - `repo_id`: Filter by specific repository ID
      - `worktree_id`: Filter by specific worktree ID
      - `file_type`: Filter by file extension (e.g., "ts", "rs", "md")
      - `recency_threshold`: Filter by file modification time (PostgreSQL interval, e.g., "7 days", "1 month")
    - `debug` (optional, default: false): Enable debug mode for score breakdowns
  - Behavior: Searches code using the specified mode (FTS, vector similarity, or hybrid). Returns ranked results with metadata.
  - Search Modes:
    - **`fts`** (full-text search): Best for exact keyword matches, identifiers, specific terms
    - **`vector`** (semantic search): Best for conceptual queries, finding similar code (requires embeddings)
    - **`hybrid`** (default): Combines FTS and vector search using Reciprocal Rank Fusion for best overall results

- **`open`**
  - Inputs: `{ relpath: string, worktree: string, range?: { start?: number, end?: number }, context?: number }`
  - Behavior: Opens a file slice from the on-disk worktree path. Supports line ranges and context lines.

- **`upsert`**
  - Inputs: `{ paths: string[], commit: string, repo: string, worktree: string, root: string }`
  - Behavior: Invokes the Rust binary to (re)index specific files

#### Search Examples

**Basic search (uses hybrid mode by default):**
```json
{
  "repo": "crewchief",
  "query": "authentication logic"
}
```

**Full-text search for exact keywords:**
```json
{
  "repo": "crewchief",
  "query": "handleSearch",
  "mode": "fts",
  "k": 20
}
```

**Semantic search (requires embeddings):**
```json
{
  "repo": "crewchief",
  "query": "database connection management",
  "mode": "vector"
}
```

**Filter by file type and recency:**
```json
{
  "repo": "crewchief",
  "query": "configuration",
  "filter": "code",
  "filters": {
    "file_type": "ts",
    "recency_threshold": "30 days"
  }
}
```

**Debug mode to see score breakdowns:**
```json
{
  "repo": "crewchief",
  "query": "search pipeline",
  "debug": true
}
```

#### Vector Search Requirements

Vector and hybrid search modes require embeddings to be generated for your codebase:

1. **Generate Embeddings**: Run the embedding generation pipeline
   ```bash
   crewchief maproom:generate-embeddings
   ```

2. **Verify Embeddings**: Check that embeddings exist in the database
   ```sql
   SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL;
   ```

3. **Fallback Behavior**:
   - If embeddings are not available, vector mode will return an error with instructions
   - Hybrid mode will gracefully fall back to FTS-only search with a notice in debug output
   - FTS mode always works without embeddings

**Note**: Full hybrid search with RRF (Reciprocal Rank Fusion) is currently in development. The current implementation falls back to FTS when embeddings are unavailable, maintaining backward compatibility while the vector search backend is being completed.

#### How the indexer binary is located

Priority order:

1. `CREWCHIEF_MAPROOM_BIN` (exact path)
2. Bundled binary: `bin/<platform>-<arch>/crewchief-maproom`
3. `crewchief` on PATH (with `maproom upsert` subcommand)
4. `crewchief-maproom` on PATH
5. Local dev binary `./target/debug/crewchief-maproom`

### Indexing and Database Setup

Before searching, migrate the DB schema and index your project.

#### Using CrewChief CLI (Recommended)

If you have [CrewChief](https://www.npmjs.com/package/crewchief) installed:

```bash
# 1) Initialize database
crewchief maproom:db

# 2) Index your codebase (auto-detects git context)
crewchief maproom:scan

# 3) Search from the command line
crewchief maproom:search "authentication logic"

# 4) Watch for changes and auto-index
crewchief maproom:watch
```

#### Using Standalone Binary

Alternatively, use the Rust binary directly:

```bash
# 1) Migrate schema
crewchief-maproom db migrate --database-url "$DATABASE_URL"

# 2) Index your repo
# From the repo root to index everything
crewchief-maproom scan --repo <repo_name> --worktree <worktree_name> --root $(pwd)

# Or selectively upsert files used by MCP tool
crewchief-maproom maproom upsert \
  --paths file1.ts,file2.rs \
  --commit <git_sha_or_label> \
  --repo <repo_name> \
  --worktree <worktree_name> \
  --root $(pwd)
```

Notes:

- **`scan`** supports `--exclude` globs via `.gitignore` semantics and extra override patterns
- **`watch`** provides a throttled file watcher for continuous indexing

### Development

```bash
# Build TypeScript
pnpm -C packages/maproom-mcp build

# Dev run (stdio)
pnpm -C packages/maproom-mcp dev

# Postinstall behavior (bundled binary or cargo build)
# is implemented in src/postinstall.ts → dist/postinstall.js
```

For complete development setup including the CrewChief CLI and Rust components, see the [CrewChief repository](https://github.com/danielbushman/crewchief).

#### Project structure

- `src/index.ts`: MCP server (JSON-RPC over stdio)
- `src/postinstall.ts`: Ensures `crewchief-maproom` binary is present
- `bin/cli.js`: ESM CLI shim for `npx maproom-mcp`
- `bin/<platform>-<arch>/crewchief-maproom`: Optional prebuilt binary

### Release

From `packages/maproom-mcp`:

```bash
# Patch release
pnpm release:patch

# Minor / Major
pnpm release:minor
pnpm release:major
```

Scripts will:

- Bump version and create a commit with message `chore(release): <version>`
- Create a Git tag `v<version>` and push with tags
- Publish to npm (`--access public`)

### Troubleshooting

- **No tools appear in Cursor**
  - Restart Cursor after configuring MCP
  - Ensure `DATABASE_URL` is set in the MCP config env
  - Ensure `maproom-mcp` starts (check Output panel). The server logs to stderr only
- **JSON-RPC framing errors**
  - The server autodetects `Content-Length` framing; avoid writing to stdout from plugins
- **Postgres connection errors**
  - Verify `DATABASE_URL`. Percent-encode special characters in the password
  - Run migrations: `crewchief-maproom db migrate --database-url "$DATABASE_URL"`
- **Indexer not found**
  - Set `CREWCHIEF_MAPROOM_BIN` to an absolute path, or ensure Cargo is installed to build locally, or place a prebuilt binary in `bin/<platform>-<arch>/`
