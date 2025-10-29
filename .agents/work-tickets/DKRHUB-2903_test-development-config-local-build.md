# Ticket: DKRHUB-2903: Test Development Configuration (Local Build)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Test that docker-compose.override.yml allows local builds, preserving the development workflow for contributors building from source.

## Background
After creating docker-compose.override.yml, we must verify:
1. Override file is detected and merged with docker-compose.yml
2. Local build works (using `build:` from override)
3. Built image is used instead of pulling from Docker Hub
4. Development workflow is preserved

This ensures contributors can continue developing locally without Docker Hub dependency.

Reference: DKRHUB_PLAN.md Phase 2, Task DKRHUB-2005 (lines 487-527)

## Acceptance Criteria
- [ ] docker-compose.override.yml present in config/ directory
- [ ] `docker-compose build` succeeds and builds image from source
- [ ] Local image created with recognizable tag (not crewchief/maproom-mcp)
- [ ] `docker-compose up` uses locally built image, not Docker Hub image
- [ ] Override merges correctly with base compose file
- [ ] Build time reasonable (<15 minutes)
- [ ] No conflicts with production configuration
- [ ] Cleanup: Remove override, verify production mode still works

## Technical Requirements
**Test Environment**:
- Location: `packages/maproom-mcp/config/`
- Files present: docker-compose.yml, docker-compose.override.yml
- Docker version: 24.0+
- Build tools: Not required (already in Docker image)

**Test Commands**:
```bash
cd packages/maproom-mcp/config

# 1. Verify override file exists
ls -la docker-compose.override.yml
cat docker-compose.override.yml

# 2. Check compose file merge
docker-compose config | grep -A5 maproom-mcp
# Should show build: section from override

# 3. Build locally (from source)
time docker-compose build maproom-mcp

# 4. Verify local image created
docker images | grep maproom-mcp
# Should show locally built image (tag contains directory or "latest")

# 5. Start services with local build
docker-compose up -d

# 6. Verify local image used (not Docker Hub)
docker inspect maproom-mcp --format='{{.Config.Image}}'
# Should NOT be "crewchief/maproom-mcp:latest"
# Should be something like "config_maproom-mcp" or "maproom-mcp:latest"

# 7. Test functionality
docker ps
docker logs maproom-mcp
docker exec maproom-mcp node --version

# 8. Stop services
docker-compose down

# 9. Test production mode (without override)
mv docker-compose.override.yml docker-compose.override.yml.bak
docker-compose up -d
docker inspect maproom-mcp --format='{{.Config.Image}}'
# Should be "crewchief/maproom-mcp:latest" (pulled from Docker Hub)

# 10. Restore override for development
mv docker-compose.override.yml.bak docker-compose.override.yml
docker-compose down -v
```

## Implementation Notes
**Docker Compose Merge Behavior**:
When both files exist in the same directory:
1. docker-compose.yml is loaded first (base config)
2. docker-compose.override.yml is loaded second (overrides)
3. Merge rules:
   - Scalar values (strings, numbers): Override replaces base
   - Lists/arrays: Override appends to base
   - Maps/objects: Merged recursively

In our case:
- Base has: `image: crewchief/maproom-mcp:latest`
- Override has: `build: {context: ..., dockerfile: ...}`
- Result: `build:` takes precedence, image is built (not pulled)

**Local Image Naming**:
When building with docker-compose, image name depends on:
- Project name (derived from directory name or set via -p flag)
- Service name (maproom-mcp)
- Default format: `{project}_{service}` or `{project}-{service}`
- Example: `config_maproom-mcp` or `maproom-mcp-config_maproom-mcp`

**Build Time Expectations**:
- First build: 8-12 minutes (no cache)
- Subsequent builds: 2-5 minutes (with layer cache)
- Multi-stage build reduces final image size

Reference DKRHUB_QUALITY_STRATEGY.md lines 536-584 for regression testing details.

## Dependencies
- DKRHUB-2002: docker-compose.override.yml must be created
- DKRHUB-2001: docker-compose.yml must be updated (to verify override takes precedence)

## Risk Assessment
- **Risk**: Override doesn't merge correctly
  - **Mitigation**: Test with `docker-compose config` to preview merge
- **Risk**: Build context path incorrect
  - **Mitigation**: Path is relative to override file, tested in DKRHUB-2002
- **Risk**: Contributors confused by two modes
  - **Mitigation**: Document in override file comments and README

## Files/Packages Affected
- None (testing only, no code changes)
