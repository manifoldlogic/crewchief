## Maproom MCP Server

Maproom is a code-aware indexing and retrieval layer for multi-agent work. This package provides the MCP (Model Context Protocol) server used by editors like Cursor to search and open code using Maproom’s Postgres-backed index. It also orchestrates indexing by locating and executing the Rust indexer binary (`crewchief-maproom`).

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

### Using with Cursor (MCP)

Add to Cursor’s MCP configuration (`Settings → MCP Servers`), for example:

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

Then restart Cursor. You should see 3 tools enabled for Maproom.

### Tools

- **`search`**
  - Inputs: `{ repo: string, worktree?: string | null, query: string, k?: number }`
  - Behavior: FTS over `maproom.chunks.ts_doc`. If `worktree` is supplied, results are scoped to that worktree
- **`open`**
  - Inputs: `{ relpath: string, worktree: string, range?: { start?: number, end?: number } }`
  - Behavior: Opens a file slice from the on-disk worktree path
- **`upsert`**
  - Inputs: `{ paths: string[], commit: string, repo: string, worktree: string, root: string }`
  - Behavior: Invokes the Rust binary to (re)index specific files

#### How the indexer binary is located

Priority order:

1. `CREWCHIEF_MAPROOM_BIN` (exact path)
2. Bundled binary: `bin/<platform>-<arch>/crewchief-maproom`
3. `crewchief` on PATH (with `maproom upsert` subcommand)
4. `crewchief-maproom` on PATH
5. Local dev binary `./target/debug/crewchief-maproom`

### Indexing and Database Setup

Before searching, migrate the DB schema and index your project.

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

### License

Apache-2.0 (or project license).
