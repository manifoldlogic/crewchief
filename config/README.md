# Maproom Database Infrastructure

This directory contains Docker Compose configuration for running the PostgreSQL database required by Maproom.

**Note**: The MCP server now runs on the host via `npx @crewchief/maproom-mcp`, not in a container.

## Quick Start

```bash
# Start PostgreSQL (from this directory)
docker compose up -d

# Run the MCP server
npx @crewchief/maproom-mcp

# View logs
docker compose logs -f

# Stop services
docker compose down
```

## Services

### PostgreSQL (maproom-postgres)
- **Image**: pgvector/pgvector:pg16
- **Purpose**: Database with pgvector extension for hybrid search
- **Port**: 5433 (mapped to host for MCP server access)
- **Database**: maproom
- **Credentials**: maproom/maproom (change in production!)
- **Health Check**: pg_isready every 10s

### Ollama (Optional)
Ollama configuration is commented out in docker-compose.yml. Uncomment if you want to use local embeddings with `MAPROOM_EMBEDDING_PROVIDER=ollama`.

## Data Persistence

Named volume stores persistent data:
- **maproom-data**: PostgreSQL database files

To reset all data:
```bash
docker compose down -v
```

## Environment Variables

The MCP server auto-detects the database URL:
1. `MAPROOM_DATABASE_URL` (explicit override)
2. `IN_DEVCONTAINER=true` → uses `maproom-postgres:5432`
3. Default → `localhost:5433`

## VSCode Extension Users

If using the VSCode Maproom extension, use its bundled docker-compose instead:
```bash
cd packages/vscode-maproom/config
docker compose up -d
```

The extension automatically manages Docker containers and configures the MCP server.

## Troubleshooting

### Database connection errors
```bash
# Check postgres health
docker compose exec postgres pg_isready -U maproom -d maproom

# View postgres logs
docker compose logs postgres

# Reset database (WARNING: deletes all data)
docker compose down -v
docker compose up -d
```

### Port conflicts
If port 5433 is already in use, set a custom database URL:
```bash
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom npx @crewchief/maproom-mcp
```

## Files in This Directory

- `docker-compose.yml`: PostgreSQL database configuration
- `README.md`: This file
