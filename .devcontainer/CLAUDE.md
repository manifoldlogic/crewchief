# CLAUDE.md - .devcontainer

Working with the development container at `/.devcontainer`.

## Quick Start

**VS Code**:
1. Install Docker Desktop, VS Code, Dev Containers extension
2. `F1` → "Dev Containers: Reopen in Container"

**Cursor**:
1. Install Docker Desktop, Cursor
2. `Cmd+Shift+P` → "Remote-Containers: Reopen in Container"
3. See `CURSOR_SETUP.md` for details

## Files

```
.devcontainer/
├── devcontainer.json    # Container config
├── docker-compose.yml   # Services
├── Dockerfile           # Base image
├── scripts/
│   ├── post-create.sh   # First-time setup
│   ├── post-start.sh    # Every start
│   └── post-attach.sh   # Editor attach
├── README.md
├── CURSOR_SETUP.md
└── TROUBLESHOOTING.md
```

## What's Included

- **Languages**: Node.js 20, Rust, Python
- **Features**: Git, GitHub CLI, Docker-in-Docker
- **Database**: PostgreSQL via maproom-mcp network
- **Tools**: bash, tmux, ripgrep, fd

## Database Access

Database runs via `packages/maproom-mcp/config/docker-compose.yml`.

Connection from devcontainer:
- **Host**: `maproom-postgres` (via network) or `localhost:5432`
- **Database**: `maproom`
- **User/Password**: `maproom/maproom`
- **Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`

## Lifecycle Scripts

- **post-create.sh** - Runs once on first build
  - Install pnpm, dependencies
  - Build TypeScript and Rust
  - Copy binaries
- **post-start.sh** - Runs on container start
- **post-attach.sh** - Runs when editor attaches

## Rebuild Container

```bash
# VS Code/Cursor
F1 → "Dev Containers: Rebuild Container"

# CLI
docker compose -f .devcontainer/docker-compose.yml down
docker compose -f .devcontainer/docker-compose.yml up --build
```

## Environment Variables

```bash
NODE_ENV=development
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
CLAUDE_DANGEROUS_MODE=true
```

See `docker-compose.yml` for full list.

## Troubleshooting

See `TROUBLESHOOTING.md` for:
- Build failures
- Database connection issues
- Port conflicts
- Volume permissions
