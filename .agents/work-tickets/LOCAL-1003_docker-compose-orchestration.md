# Ticket: LOCAL-1003: Create docker-compose.yml with all services

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- manual-tester (docker compose validation)
- verify-ticket
- commit-ticket

## Summary
Create a production-ready docker-compose.yml that orchestrates PostgreSQL (with pgvector), Ollama (with nomic-embed-text model), and Maproom MCP service into a coordinated multi-service stack. This is the orchestration heart of the LOCAL project and will be embedded in the npm package for zero-config deployment.

## Background
The LOCAL project aims to provide a fully containerized Maproom MCP service with local LLM embeddings, eliminating the need for external API keys or cloud services. The docker-compose.yml is the critical orchestration layer that brings together:

1. **PostgreSQL** - Database with pgvector extension for hybrid search
2. **Ollama** - Local LLM runtime with nomic-embed-text model
3. **Maproom MCP** - The semantic code search service

This file must handle complex service dependencies, health checks, automatic model provisioning, and data persistence. It will be the primary deployment artifact for end users via the npm package wrapper.

**Reference Documents**:
- LOCAL_PLAN.md: Phase 1, Task LOCAL-1003 (lines 46)
- LOCAL_ARCHITECTURE.md: Complete docker-compose.yml specification (lines 563-671)

## Acceptance Criteria
- [ ] docker-compose.yml file created in /workspace/config/ directory
- [ ] All three services (postgres, ollama, maproom) defined with correct images/build contexts
- [ ] postgres service uses pgvector/pgvector:pg16 image with proper initialization
- [ ] ollama service includes automatic nomic-embed-text model download on startup
- [ ] maproom service builds from Dockerfile.maproom with proper environment variables
- [ ] Health checks configured for all services (pg_isready, curl checks)
- [ ] Service dependency ordering (maproom depends on postgres + ollama with health check conditions)
- [ ] Three volumes defined (maproom-data, ollama-models, maproom-config) with local driver
- [ ] maproom-network defined with bridge driver
- [ ] Stack starts successfully with `docker compose up -d`
- [ ] All services reach "healthy" status within expected timeframes
- [ ] Services can communicate on internal network (postgres and ollama accessible to maproom)

## Technical Requirements

### Postgres Service
- Image: `pgvector/pgvector:pg16`
- Container name: `maproom-postgres`
- Environment variables:
  - `POSTGRES_DB=maproom`
  - `POSTGRES_USER=maproom`
  - `POSTGRES_PASSWORD=maproom`
  - `PGDATA=/var/lib/postgresql/data/pgdata`
- Volumes:
  - `maproom-data:/var/lib/postgresql/data` (persistence)
  - `./init.sql:/docker-entrypoint-initdb.d/init.sql:ro` (schema init)
  - `./postgresql.conf:/etc/postgresql/postgresql.conf:ro` (config)
- Health check: `pg_isready -U maproom -d maproom` every 10s
- Network: maproom-network (internal only, no port exposure)
- Restart policy: unless-stopped

### Ollama Service
- Image: `ollama/ollama:latest`
- Container name: `maproom-ollama`
- Volumes:
  - `ollama-models:/root/.ollama` (model cache)
  - `./init-ollama.sh:/usr/local/bin/init-ollama.sh:ro` (init script)
- Port: `${OLLAMA_PORT:-11434}:11434` (configurable via env var)
- Health check: `curl -f http://localhost:11434/api/tags` every 30s, start_period 120s
- Command: Multi-line shell script that:
  1. Starts ollama server in background
  2. Waits for server to be ready
  3. Pulls nomic-embed-text model
  4. Keeps server running
- Network: maproom-network
- Restart policy: unless-stopped
- Optional GPU support (commented out)

### Maproom Service
- Build context: `.` (current directory)
- Dockerfile: `Dockerfile.maproom`
- Container name: `maproom-mcp`
- Depends on: postgres (service_healthy), ollama (service_healthy)
- Environment variables:
  - `DATABASE_URL=postgresql://maproom:maproom@postgres:5432/maproom`
  - `EMBEDDING_PROVIDER=ollama`
  - `EMBEDDING_MODEL=nomic-embed-text`
  - `EMBEDDING_DIMENSION=768`
  - `EMBEDDING_API_ENDPOINT=http://ollama:11434`
  - `RUST_LOG=${RUST_LOG:-info}`
- Volumes:
  - `maproom-config:/config` (config persistence)
  - `${HOST_WORKSPACE:-/workspace}:/workspace:ro` (workspace mount, read-only)
- Port: `${MAPROOM_PORT:-3000}:3000` (configurable via env var)
- Health check: `curl -f http://localhost:3000/health` every 30s, start_period 60s
- Network: maproom-network
- Restart policy: unless-stopped

### Networks
- `maproom-network`: bridge driver (default bridge mode)

### Volumes
- `maproom-data`: local driver (PostgreSQL data persistence)
- `ollama-models`: local driver (Ollama model cache)
- `maproom-config`: local driver (Maproom configuration)

## Implementation Notes

### Critical Considerations
1. **Service Startup Order**: The `depends_on` with health check conditions is critical. Maproom must wait for both postgres AND ollama to be healthy before starting. This prevents connection errors during startup.

2. **Ollama Model Provisioning**: The ollama service uses a custom command that:
   - Starts the ollama server in the background
   - Polls the health endpoint until ready
   - Pulls the nomic-embed-text model (can take several minutes on first run)
   - Keeps the server running via `wait $OLLAMA_PID`

3. **Health Check Timing**:
   - postgres: 10s interval, 30s start_period (fast database startup)
   - ollama: 30s interval, 120s start_period (allows time for model download)
   - maproom: 30s interval, 60s start_period (depends on postgres + ollama being ready)

4. **Environment Variable Defaults**: Use `${VAR:-default}` syntax to allow user overrides while providing sensible defaults.

5. **Volume Mounts**:
   - PostgreSQL and Ollama volumes must persist data between restarts
   - Workspace mount is read-only (`:ro`) for security
   - Init scripts are mounted read-only

6. **Network Security**: Only necessary ports exposed to host (ollama:11434, maproom:3000). PostgreSQL stays internal to container network.

### Reference Implementation
The complete docker-compose.yml specification is provided in LOCAL_ARCHITECTURE.md lines 565-671. Follow this specification exactly, including:
- Exact service names (maproom-postgres, maproom-ollama, maproom-mcp)
- Version string: `'3.8'`
- All environment variables as specified
- Health check parameters (interval, timeout, retries, start_period)
- Volume mount paths and flags

### Testing Strategy
After creation, validate with:
```bash
# Validate syntax
docker compose config

# Start services
docker compose up -d

# Check health status
docker compose ps

# Verify logs
docker compose logs -f

# Test connectivity
docker compose exec maproom curl http://postgres:5432
docker compose exec maproom curl http://ollama:11434/api/tags
```

## Dependencies
- **LOCAL-1001** (REQUIRED): Dockerfile.maproom must exist for the maproom service build context
- **LOCAL-1002** (OPTIONAL): init.sql for PostgreSQL initialization (can be created later)
- init-ollama.sh script (if using separate init script instead of inline command)

## Risk Assessment

- **Risk**: Ollama model download fails on first startup
  - **Mitigation**: Health check has 120s start_period to allow download time. Add retry logic in init command. Document manual model pull procedure.

- **Risk**: Service dependency deadlock if health checks fail
  - **Mitigation**: Proper health check configuration with retries. Clear error messages in logs. Document manual startup procedure.

- **Risk**: Volume permissions issues on different platforms (Linux vs macOS)
  - **Mitigation**: Use named volumes (not bind mounts) for data persistence. Test on both platforms. Document known platform quirks.

- **Risk**: Port conflicts with existing services (3000, 11434, 5432)
  - **Mitigation**: Make ports configurable via environment variables with defaults. Document how to override ports.

- **Risk**: Docker Compose version compatibility (v1 vs v2)
  - **Mitigation**: Use version: '3.8' which is compatible with both. Document requirement for Docker Compose v2+ in README.

## Files/Packages Affected
- `/workspace/config/docker-compose.yml` (NEW)
- Optionally `/workspace/config/init-ollama.sh` (NEW - if using separate script)
- Validation in Phase 3 task LOCAL-3001
