# Ticket: TESTENV-2001: Add daemon service to Docker Compose

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Verify daemon container builds and starts with health check.

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add a `maproom-daemon` service to the Docker Compose configuration, using the existing `Dockerfile.maproom` from the repository root. The daemon should be activated via an `e2e` profile for optional E2E testing.

## Background
The Dockerfile for the maproom daemon already exists at `/workspace/Dockerfile.maproom` with all required features (multi-stage build, non-root user, health check). This ticket adds the service definition to Docker Compose to enable containerized daemon usage for E2E tests.

Reference: [plan.md](../planning/plan.md) - Phase 2, Deliverable 1: "Docker Compose Updates"
Reference: [architecture.md](../planning/architecture.md) - "Existing Dockerfile" section

## Acceptance Criteria
- [ ] `maproom-daemon` service added to `docker-compose.yml`
- [ ] Service uses existing `Dockerfile.maproom` (no new Dockerfile)
- [ ] Service is gated by `e2e` profile (not started by default)
- [ ] Service depends on `postgres-test` with health check condition
- [ ] Service connects to test database via container networking
- [ ] Container builds successfully: `docker compose --profile e2e build maproom-daemon`
- [ ] Container starts and responds to health check
- [ ] Container logs show successful database connection

## Technical Requirements

### Docker Compose Service Definition
Add to `packages/vscode-maproom/config/docker-compose.yml`:

```yaml
services:
  # ... existing postgres and postgres-test services ...

  maproom-daemon:
    container_name: maproom-daemon
    profiles:
      - e2e
    build:
      context: ../../..                    # Repository root
      dockerfile: Dockerfile.maproom       # Use existing Dockerfile
    environment:
      MAPROOM_DATABASE_URL: postgresql://maproom:maproom@postgres-test:5432/maproom_test
      MAPROOM_EMBEDDING_PROVIDER: ollama
      OLLAMA_HOST: http://host.docker.internal:11434
      RUST_LOG: info
    depends_on:
      postgres-test:
        condition: service_healthy
    networks:
      maproom-network:
        aliases:
          - maproom-daemon
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s
    ports:
      - "3000:3000"                        # Expose for local testing
```

### Existing Dockerfile Features
The existing `Dockerfile.maproom` includes:
- Multi-stage build (rust:1.82-slim → debian:bookworm-slim)
- Non-root user (`maproom`, uid 1000)
- Health check on port 3000
- Stripped binary for minimal image size
- `ENTRYPOINT ["/usr/local/bin/crewchief-maproom"]`
- `CMD ["serve", "--host", "0.0.0.0", "--port", "3000"]`

### Verification Commands
```bash
# Build the daemon image
docker compose -p crewchief-dev-env --profile e2e build maproom-daemon

# Start with e2e profile
docker compose -p crewchief-dev-env --profile e2e up -d

# Check health
docker inspect --format='{{.State.Health.Status}}' maproom-daemon

# View logs
docker logs maproom-daemon

# Test health endpoint
curl http://localhost:3000/health

# Stop
docker compose -p crewchief-dev-env --profile e2e down
```

## Implementation Notes

1. **Use existing Dockerfile** - Do NOT create a new Dockerfile. Reference `Dockerfile.maproom` from repo root.

2. **Profile gating** - The `e2e` profile ensures daemon doesn't start during normal development or CI fixture tests.

3. **Network configuration** - Use `maproom-network` for container-to-container communication.

4. **Health check** - The existing Dockerfile has a health check, but we override it to use `curl` for simpler debugging.

5. **Port exposure** - Port 3000 is exposed for local development; in CI, tests connect via container network.

6. **Database URL** - Uses `postgres-test:5432` (container name) for internal networking, not `host.docker.internal`.

7. **Embedding provider** - Defaults to Ollama; can be overridden for tests that don't need embeddings.

## Dependencies
- TESTENV-1006 (Phase 1 complete - fixtures working)
- Existing `Dockerfile.maproom` at repository root

## Risk Assessment
- **Risk**: Long build time for daemon image
  - **Mitigation**: Build is cached after first run; ~2-5 minutes initial build
- **Risk**: Daemon can't connect to database
  - **Mitigation**: `depends_on` with health check ensures postgres-test is ready
- **Risk**: Port 3000 conflict
  - **Mitigation**: Port is only exposed when e2e profile is active

## Files/Packages Affected
- `packages/vscode-maproom/config/docker-compose.yml` (MODIFY)
