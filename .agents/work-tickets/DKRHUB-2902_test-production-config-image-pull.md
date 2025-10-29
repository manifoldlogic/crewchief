# Ticket: DKRHUB-2902: Test Production Configuration (Image Pull)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Test that the updated docker-compose.yml successfully pulls pre-built images from Docker Hub and starts all services without attempting local builds.

## Background
After updating docker-compose.yml to use `image:` instead of `build:`, we must verify:
1. Images pull from Docker Hub (not built locally)
2. Services start correctly
3. Health checks pass
4. No build errors occur

This validates the production deployment configuration that will be published in the npm package.

Reference: DKRHUB_PLAN.md Phase 2, Task DKRHUB-2004 (lines 444-485)

## Acceptance Criteria
- [ ] Clean Docker environment (no cached images or volumes)
- [ ] docker-compose up successfully pulls images from Docker Hub
- [ ] All three services start: maproom-mcp, maproom-postgres, maproom-ollama
- [ ] Health checks pass for all services within 60 seconds
- [ ] No build errors in docker-compose output (confirms no build attempted)
- [ ] Logs show successful service startup with no errors
- [ ] `docker images` shows images from crewchief/maproom-mcp (not local builds)

## Technical Requirements
**Test Environment**:
- Location: `packages/maproom-mcp/config/`
- Files present: docker-compose.yml (updated), NO docker-compose.override.yml
- Docker version: 24.0+
- Internet connection: Required (for image pull)

**Test Commands**:
```bash
# 1. Clean environment
docker-compose -f packages/maproom-mcp/config/docker-compose.yml down -v
docker system prune -af
docker volume prune -f

# 2. Verify no maproom images cached
docker images | grep maproom
# Should be empty

# 3. Start services (should pull, not build)
cd packages/maproom-mcp/config
time docker-compose up -d

# 4. Monitor pull progress
docker-compose logs -f

# 5. Wait for services to stabilize
sleep 60

# 6. Verify all containers running
docker ps --filter "name=maproom"
# Should show 3 containers: maproom-mcp, maproom-postgres, maproom-ollama

# 7. Check health status
docker inspect maproom-mcp --format='{{.State.Health.Status}}'
docker inspect maproom-postgres --format='{{.State.Health.Status}}'
# Should both be "healthy"

# 8. Verify image source
docker inspect maproom-mcp --format='{{.Config.Image}}'
# Should be: crewchief/maproom-mcp:latest (or specific version)

# 9. Check logs for errors
docker logs maproom-mcp 2>&1 | grep -i error
docker logs maproom-postgres 2>&1 | grep -i error
docker logs maproom-ollama 2>&1 | grep -i error
# Should be minimal/expected errors only

# 10. Cleanup
docker-compose down -v
```

## Implementation Notes
**Success Indicators**:
- docker-compose output shows "Pulling maproom-mcp" (not "Building maproom-mcp")
- Image pull completes in <3 minutes (depending on connection speed)
- All services reach "running" state
- Health checks report "healthy"
- No errors like "lstat /packages: no such file or directory"

**Failure Scenarios**:
1. **Image not found**: Indicates workflow hasn't published images yet
   - Fix: Run DKRHUB-1901 to publish test images first
2. **Build attempted**: docker-compose.yml still has `build:` directive
   - Fix: Verify DKRHUB-2001 changes applied correctly
3. **Services don't start**: Configuration issue
   - Fix: Check environment variables, port conflicts

**Performance Expectations** (from DKRHUB_QUALITY_STRATEGY.md):
- Image pull: <3 minutes
- Container start: <15 seconds
- Health check pass: <30 seconds
- Total startup: <4 minutes

Reference DKRHUB_QUALITY_STRATEGY.md lines 230-272 for detailed service communication tests.

## Dependencies
- DKRHUB-2001: docker-compose.yml must be updated to use `image:`
- DKRHUB-1901: Images must be published to Docker Hub (test images acceptable)

## Risk Assessment
- **Risk**: Docker Hub rate limit during pull
  - **Mitigation**: Authenticated pulls have higher limits; test during low-traffic times
- **Risk**: Network issues during pull
  - **Mitigation**: Retry pull if fails; Docker resumable downloads
- **Risk**: Wrong image version pulled
  - **Mitigation**: Verify image tag with `docker inspect`

## Files/Packages Affected
- None (testing only, no code changes)
