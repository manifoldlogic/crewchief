# Ticket: LOCAL-1005: Configure health checks for all services

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- monitoring-observability-engineer
- manual-testing (docker compose verification)
- verify-ticket
- commit-ticket

## Summary
Configure comprehensive health checks for PostgreSQL, Ollama, and Maproom MCP services in docker-compose.yml to ensure proper startup ordering, service health monitoring, and automatic recovery.

## Background
Health checks are critical for reliable service orchestration in Docker Compose. Without proper health checks, dependent services may attempt to connect before their dependencies are ready, causing startup failures. This ticket implements health checks according to LOCAL_ARCHITECTURE.md specifications, enabling:

1. **Proper startup ordering** - Maproom waits for PostgreSQL and Ollama to be healthy
2. **Automatic recovery** - Unhealthy services trigger restarts via `restart: unless-stopped`
3. **Service visibility** - `docker compose ps` shows health status for monitoring
4. **Initialization grace periods** - Start periods account for model downloads and database initialization

This work depends on LOCAL-1003 (docker-compose.yml creation) and is part of Phase 1 - Core Infrastructure (Week 1) from LOCAL_PLAN.md.

## Acceptance Criteria
- [ ] PostgreSQL has health check configured: `pg_isready -U maproom -d maproom` (10s interval, 5s timeout, 5 retries, 30s start period)
- [ ] Ollama has health check configured: `curl -f http://localhost:11434/api/tags` (30s interval, 10s timeout, 3 retries, 120s start period)
- [ ] Maproom MCP has health check configured: `curl -f http://localhost:3000/health` (30s interval, 10s timeout, 3 retries, 60s start period)
- [ ] All services show "healthy" status in `docker compose ps` after successful startup
- [ ] Maproom service uses `depends_on` with `condition: service_healthy` for both PostgreSQL and Ollama
- [ ] All services have `restart: unless-stopped` policy configured
- [ ] Manual testing confirms unhealthy services trigger restart behavior
- [ ] Health check endpoints return appropriate HTTP status codes (200 for healthy)

## Technical Requirements

### PostgreSQL Health Check
```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U maproom -d maproom"]
  interval: 10s
  timeout: 5s
  retries: 5
  start_period: 30s
```

### Ollama Health Check
```yaml
healthcheck:
  test: ["CMD-SHELL", "curl -f http://localhost:11434/api/tags || exit 1"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 120s  # Allows time for model download on first startup
```

### Maproom MCP Health Check
```yaml
healthcheck:
  test: ["CMD-SHELL", "curl -f http://localhost:3000/health || exit 1"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 60s  # Allows time for initial indexing
```

### Service Dependencies
- Maproom service must use:
  ```yaml
  depends_on:
    postgres:
      condition: service_healthy
    ollama:
      condition: service_healthy
  ```

### Restart Policy
- All three services must have: `restart: unless-stopped`

## Implementation Notes

### Health Check Design Principles
1. **Intervals**: PostgreSQL uses shorter interval (10s) as it's a critical dependency; Ollama and Maproom use 30s as they're stable once running
2. **Start Periods**:
   - PostgreSQL: 30s for database initialization
   - Ollama: 120s to allow for initial model download (e.g., nomic-embed-text)
   - Maproom: 60s to allow for initial repository indexing
3. **Retries**: PostgreSQL gets 5 retries (core dependency), others get 3 retries
4. **Timeouts**: 5s for PostgreSQL (fast response), 10s for HTTP endpoints (network overhead)

### Health Endpoint Requirements
- Maproom MCP must implement a `/health` endpoint that:
  - Returns HTTP 200 when service is operational
  - Checks database connectivity
  - Returns HTTP 503 if database is unreachable
  - Responds within 10s timeout

### Testing Strategy
1. **Successful startup**: Run `docker compose up` and verify all services reach "healthy" status
2. **Forced failure**: Kill PostgreSQL and verify Maproom detects unhealthy state and restarts
3. **Dependency ordering**: Start from clean state and verify Maproom waits for dependencies
4. **Recovery**: Restart failed services and verify they return to healthy state

### Reference Documentation
- Docker Compose healthcheck: https://docs.docker.com/compose/compose-file/05-services/#healthcheck
- Healthcheck best practices: https://docs.docker.com/engine/reference/builder/#healthcheck
- LOCAL_ARCHITECTURE.md lines 583-588 (PostgreSQL), 601-606 (Ollama), 652-657 (Maproom)

## Dependencies
- **LOCAL-1003**: docker-compose.yml must exist (prerequisite)
- **External**: curl must be available in Ollama and Maproom containers
- **External**: pg_isready must be available in PostgreSQL container (included in official postgres image)

## Risk Assessment
- **Risk**: Ollama's 120s start period may be insufficient for slow networks or large model downloads
  - **Mitigation**: Start period is configurable; can be increased if needed. Health check will retry 3 times before marking unhealthy.

- **Risk**: Health check commands may fail if curl is not installed in containers
  - **Mitigation**: Verify Dockerfile includes curl installation. PostgreSQL uses pg_isready (built-in).

- **Risk**: Maproom /health endpoint may not exist yet
  - **Mitigation**: If endpoint doesn't exist, create minimal implementation that checks database connection and returns HTTP 200/503.

- **Risk**: False negatives during initial startup (services marked unhealthy too quickly)
  - **Mitigation**: Generous start_period values account for initialization time. Can be tuned based on testing.

## Files/Packages Affected
- `docker-compose.yml` - Add healthcheck configurations to all three services
- `docker-compose.yml` - Update Maproom depends_on to use service_healthy condition
- `crates/maproom/src/main.rs` (potentially) - Add /health endpoint if not present
- `crates/maproom/src/http/` (potentially) - Health check handler implementation
