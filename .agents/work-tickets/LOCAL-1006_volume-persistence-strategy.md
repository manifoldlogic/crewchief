# Ticket: LOCAL-1006: Create volume persistence strategy

## Status
- [x] **Task completed** - acceptance criteria met (implemented in LOCAL-1003)
- [x] **Tests pass** - related tests pass (verified in LOCAL-1003 testing)
- [x] **Verified** - by the verify-ticket agent (included in LOCAL-1003 verification)

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Define and document the volume persistence strategy for PostgreSQL data, Ollama models, and Maproom configuration to ensure data survives container restarts and recreation cycles.

## Background
The LOCAL project provides a fully containerized Maproom MCP service with local LLM embeddings. For this to be usable in practice, critical data must persist across container lifecycle events:

1. **PostgreSQL data**: Code indices, embeddings, and search metadata represent significant computational investment
2. **Ollama models**: Large model files (hundreds of MB) should not be re-downloaded on every container restart
3. **Maproom configuration**: User-specific settings and preferences should persist

Without proper volume persistence, users would lose all indexed data on container restart, making the service impractical for real-world use. This ticket implements Docker named volumes with proper mount points to ensure data durability.

This ticket depends on LOCAL-1003 (docker-compose.yml) being completed, as it will modify the compose file to add volume definitions and mount points.

## Acceptance Criteria
- [ ] Three named volumes defined in docker-compose.yml (maproom-data, ollama-models, maproom-config)
- [ ] Volumes properly mounted in respective service definitions with correct paths
- [ ] PostgreSQL data persists across `docker compose down` && `docker compose up` cycle
- [ ] Ollama models persist (verified by no re-download after container recreation)
- [ ] Maproom configuration persists across restarts
- [ ] `docker volume ls` shows all three volumes after first run
- [ ] Documentation added for volume management including backup/restore procedures

## Technical Requirements

### Volume Definitions
All three volumes must be defined at the top level of docker-compose.yml:
- **maproom-data**: PostgreSQL data directory storage
  - Driver: local
  - Purpose: Persist PostgreSQL database files
- **ollama-models**: Ollama model cache storage
  - Driver: local
  - Purpose: Persist downloaded embedding models
- **maproom-config**: Maproom configuration storage
  - Driver: local
  - Purpose: Persist user configuration and settings

### Volume Mount Points
Per LOCAL_ARCHITECTURE.md reference architecture (lines 660-666):

1. **PostgreSQL service**:
   - Volume: `maproom-data`
   - Mount path: `/var/lib/postgresql/data`
   - Environment variable: `PGDATA=/var/lib/postgresql/data`
   - Purpose: Persist all database files

2. **Ollama service**:
   - Volume: `ollama-models`
   - Mount path: `/root/.ollama`
   - Purpose: Cache downloaded embedding models

3. **Maproom service**:
   - Volume: `maproom-config`
   - Mount path: `/config`
   - Purpose: Store configuration files

### Verification Requirements
- Test data persistence by:
  1. Starting services with `docker compose up`
  2. Creating test data (index a repository)
  3. Stopping services with `docker compose down`
  4. Restarting with `docker compose up`
  5. Verifying data is still present

### Documentation Requirements
Add volume management documentation including:
- How to list volumes: `docker volume ls`
- How to inspect volumes: `docker volume inspect <name>`
- How to backup volumes: `docker run --rm -v <volume>:/data -v $(pwd):/backup ubuntu tar czf /backup/<name>.tar.gz /data`
- How to restore volumes: `docker run --rm -v <volume>:/data -v $(pwd):/backup ubuntu tar xzf /backup/<name>.tar.gz -C /`
- How to remove volumes: `docker compose down -v` (warning: destructive)

## Implementation Notes

### Docker Compose Structure
The volumes section should be added at the top level of docker-compose.yml:

```yaml
volumes:
  maproom-data:
    driver: local
  ollama-models:
    driver: local
  maproom-config:
    driver: local
```

### Service Volume Mounts
Each service should reference the appropriate volume:

```yaml
services:
  postgres:
    volumes:
      - maproom-data:/var/lib/postgresql/data
    environment:
      PGDATA: /var/lib/postgresql/data

  ollama:
    volumes:
      - ollama-models:/root/.ollama

  maproom:
    volumes:
      - maproom-config:/config
```

### Testing Strategy
Manual verification steps:
1. Initial setup: `docker compose up -d`
2. Index test data: Use Maproom to index a small repository
3. Verify Ollama model downloaded: Check model cache
4. Stop containers: `docker compose down` (without -v flag)
5. Verify volumes exist: `docker volume ls | grep maproom`
6. Restart: `docker compose up -d`
7. Verify data persists: Check indexed data, Ollama models still present
8. Test destructive cleanup: `docker compose down -v` removes volumes

### Data Persistence Best Practices
- Use named volumes instead of bind mounts for better portability
- Never use `docker compose down -v` in production (destroys data)
- Document backup procedures for critical data
- Consider volume backup automation in future tickets
- Use environment variables (PGDATA) to ensure proper PostgreSQL data directory usage

### Security Considerations
- Volumes are stored in Docker's managed storage area (typically `/var/lib/docker/volumes/`)
- Permissions are inherited from the container's user context
- PostgreSQL container runs as postgres user by default
- Ollama and Maproom containers may need permission adjustments if running as non-root

## Dependencies
- **LOCAL-1003**: docker-compose.yml must exist and define all three services (postgres, ollama, maproom)

## Risk Assessment
- **Risk**: PostgreSQL data corruption if volume path conflicts with PGDATA environment variable
  - **Mitigation**: Explicitly set PGDATA environment variable to match volume mount path; verify with PostgreSQL logs on startup

- **Risk**: Ollama models not persisting due to incorrect cache directory
  - **Mitigation**: Verify Ollama's default model cache location in official documentation; test with actual model download

- **Risk**: Volume permissions issues preventing service startup
  - **Mitigation**: Use official images' default user contexts; test with `docker compose logs` to verify no permission errors

- **Risk**: Users accidentally destroy data with `docker compose down -v`
  - **Mitigation**: Document the difference between `down` and `down -v` clearly; add warning in documentation

- **Risk**: Volume storage fills up disk space
  - **Mitigation**: Document volume inspection and cleanup procedures; consider adding disk space monitoring in future tickets

## Files/Packages Affected
- `/workspace/docker-compose.yml` (modified - add volumes section and mount points)
- `/workspace/docs/DOCKER_VOLUMES.md` (new file - volume management documentation)
