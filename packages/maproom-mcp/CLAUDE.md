# CLAUDE.md - Maproom MCP

Working with the MCP server at `/packages/maproom-mcp`.

## Directory Structure

```
├── bin/cli.cjs              # CLI + Docker orchestration
├── config/
│   ├── docker-compose.yml   # PostgreSQL + pgvector
│   └── init.sql             # Database schema
├── src/
│   ├── index.ts             # MCP server
│   ├── indexer.ts           # Rust binary wrapper
│   └── tools/               # MCP tool handlers
└── tests/                   # Connection tests
```

## Development

```bash
# Build
pnpm build

# Setup provider (one-time)
node bin/cli.cjs setup --provider=ollama
node bin/cli.cjs setup --provider=openai

# Scan/watch
node bin/cli.cjs scan /path/to/repo
node bin/cli.cjs watch /path/to/repo

# Test
pnpm test
```

## Database

PostgreSQL via Docker Compose (`config/docker-compose.yml`):
- **Host**: `maproom-postgres` or `localhost:5432`
- **Database**: `maproom`
- **User/Password**: `maproom/maproom`
- **Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`

Schema in `config/init.sql`.

## MCP Tools

- `search` - Semantic search (FTS/vector/hybrid)
- `open` - Get code with line ranges
- `context` - Related chunks (imports, callers, tests)
- `status` - Index stats
- `scan` - Full repo indexing
- `upsert` - Update specific files
- `explain` - Symbol documentation

## Rust Binary

Wraps `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Spawned as subprocess
- JSON-RPC over stdin/stdout
- Rebuild only when changing `crates/maproom/`

## Docker Build

### Prerequisites

**CRITICAL**: Run `pnpm build` before building Docker image.

The Dockerfile requires pre-built workspace packages:
- daemon-client must be compiled to dist/ directory
- Run `pnpm build` at repository root before Docker build
- Failure to do so will cause "COPY failed: file not found" error

### Build Command

```bash
# From repository root
pnpm build  # Build all workspace packages first

docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:latest \
  .
```

### Multi-Platform Build

```bash
docker buildx build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:latest \
  --platform linux/amd64,linux/arm64 \
  .
```

### Troubleshooting

**"COPY failed: daemon-client/dist not found"**
- **Cause**: daemon-client not built before Docker build
- **Fix**: Run `pnpm build` at repository root
- **Verify**: `ls -la packages/daemon-client/dist/` should show index.js

**"workspace: protocol not resolved"**
- **Cause**: pnpm not installed or wrong version in Dockerfile
- **Fix**: Verify Dockerfile has `RUN npm install -g pnpm@10.12.1`
- **Check**: Version should match package.json packageManager field

**Image size larger than expected (>400MB)**
- **Cause**: node_modules or pnpm store copied to final image
- **Fix**: Verify .dockerignore excludes node_modules
- **Expected**: Final image ~360MB (pnpm only in builder stage)

## Key Points

- **ESM modules**
- **Zod** for MCP validation
- **Pino** for logging
- **pg** for PostgreSQL
- Fallback from Docker network to localhost
