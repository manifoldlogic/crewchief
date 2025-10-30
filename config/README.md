# Maproom LOCAL - Docker Compose Configuration

This directory contains the Docker Compose orchestration for running Maproom with local LLM embeddings via Ollama.

## Quick Start

```bash
# Start all services (from the config directory)
docker compose up -d

# View logs
docker compose logs -f

# Check service health
docker compose ps

# Stop all services
docker compose down

# Stop and remove volumes (clean slate)
docker compose down -v
```

## Services

### PostgreSQL (maproom-postgres)
- **Image**: pgvector/pgvector:pg16
- **Purpose**: Database with pgvector extension for hybrid search
- **Port**: Internal only (5432 not exposed to host)
- **Database**: maproom
- **Credentials**: maproom/maproom (change in production!)
- **Initialization**: Automatically loads init.sql on first startup
- **Health Check**: pg_isready every 10s

### Ollama (maproom-ollama)
- **Image**: ollama/ollama:latest
- **Purpose**: Local LLM runtime for embeddings
- **Port**: 11434 (configurable via OLLAMA_PORT env var)
- **Model**: nomic-embed-text (768 dimensions)
- **First Startup**: Downloads model automatically (~300MB, can take 2-5 minutes)
- **Health Check**: API endpoint check every 30s, 120s start period

### Maproom MCP (maproom-mcp)
- **Build**: From Dockerfile.maproom
- **Purpose**: Semantic code search MCP service
- **Port**: 3000 (configurable via MAPROOM_PORT env var)
- **Dependencies**: Waits for postgres and ollama to be healthy before starting
- **Health Check**: /health endpoint every 30s
- **Workspace Mount**: ${HOST_WORKSPACE:-/workspace} mounted read-only

## Environment Variables

Create a `.env` file in this directory to customize:

```env
# Port Configuration
OLLAMA_PORT=11434
MAPROOM_PORT=3000

# Workspace Path
HOST_WORKSPACE=/path/to/your/code

# Logging Level
RUST_LOG=info  # debug, info, warn, error
```

## Data Persistence

Three named volumes store persistent data:

- **maproom-data**: PostgreSQL database files
- **ollama-models**: Downloaded LLM models (nomic-embed-text)
- **maproom-config**: Maproom configuration files

To reset all data:
```bash
docker compose down -v
```

## Health Checks

All services implement health checks for reliable orchestration:

| Service | Check | Interval | Start Period |
|---------|-------|----------|--------------|
| postgres | pg_isready | 10s | 30s |
| ollama | curl /api/tags | 30s | 120s |
| maproom | curl /health | 30s | 60s |

The `depends_on` configuration ensures maproom only starts after postgres and ollama are healthy.

## First Startup

On first startup:

1. **PostgreSQL** (~10-20s)
   - Creates database
   - Enables pgvector extension
   - Runs init.sql schema

2. **Ollama** (~2-5 minutes)
   - Starts server
   - Downloads nomic-embed-text model (~300MB)
   - Becomes healthy when model is ready

3. **Maproom** (~30-60s)
   - Waits for postgres + ollama health
   - Connects to database
   - Initializes MCP service

**Total first startup time**: ~3-6 minutes

Subsequent startups are much faster (~30-60s) since models are cached.

## Troubleshooting

### Services won't start
```bash
# Check service logs
docker compose logs postgres
docker compose logs ollama
docker compose logs maproom

# Validate configuration
docker compose config

# Restart a specific service
docker compose restart ollama
```

### Ollama model download fails
```bash
# Manual model pull
docker compose exec ollama ollama pull nomic-embed-text

# Verify model exists
docker compose exec ollama ollama list
```

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
If ports 3000 or 11434 are already in use:
```bash
# Use custom ports
MAPROOM_PORT=3001 OLLAMA_PORT=11435 docker compose up -d
```

## Architecture Notes

### Network Isolation
All services communicate on the `maproom-network` bridge. PostgreSQL is NOT exposed to the host for security - only accessible to maproom service.

### Service Dependencies
```
maproom (depends on)
  ├── postgres (healthy)
  └── ollama (healthy)
```

This ensures proper startup order without race conditions or sleep delays.

### Volume Strategy
Named volumes (not bind mounts) for data persistence ensure:
- Platform compatibility (Linux, macOS, Windows)
- Proper permissions handling
- Clean separation of data from configuration

## Optional Configuration

### PostgreSQL Tuning
Uncomment in docker-compose.yml:
```yaml
volumes:
  - ./postgresql.conf:/etc/postgresql/postgresql.conf:ro
```

Then create postgresql.conf with tuning parameters.

### GPU Support for Ollama
Uncomment in docker-compose.yml:
```yaml
deploy:
  resources:
    reservations:
      devices:
        - driver: nvidia
          count: 1
          capabilities: [gpu]
```

Requires: nvidia-docker2 installed on host.

## Production Considerations

For production use:

1. **Change Default Credentials**: Update POSTGRES_PASSWORD
2. **Enable TLS**: Configure PostgreSQL SSL and Ollama HTTPS
3. **Resource Limits**: Add memory/CPU limits to services
4. **Backup Strategy**: Implement volume backup for maproom-data
5. **Monitoring**: Add Prometheus metrics exporters
6. **Log Rotation**: Configure log driver with rotation

## Development vs Production

This configuration is optimized for development/testing. For production:

- Use secrets management (not environment variables)
- Enable TLS/SSL for all connections
- Configure firewall rules
- Implement backup and disaster recovery
- Add monitoring and alerting
- Use specific image versions (not :latest)

## Files in This Directory

- `docker-compose.yml`: Main orchestration configuration
- `init.sql`: PostgreSQL schema initialization (pgvector, indexes)
- `README.md`: This file
- `.env` (optional): Environment variable overrides

## Support

For issues, see:
- Maproom documentation: /workspace/crates/maproom/README.md
- Docker Compose logs: `docker compose logs -f`
- Health status: `docker compose ps`
