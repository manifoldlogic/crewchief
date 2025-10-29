# Ticket: DKRHUB-2001: Update docker-compose.yml to Use Images

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Replace the `build:` section in docker-compose.yml with `image:` directive to pull pre-built images from Docker Hub instead of building from source. This fixes the broken v1.1.9 deployment.

## Background
The root cause of v1.1.9 failure: docker-compose.yml tries to build from source using `context: ../../..` which doesn't exist when the package is deployed via npm to `~/.maproom-mcp/`.

Solution: Change the maproom-mcp service to pull the pre-built image from Docker Hub.

This is the critical fix that makes the npm package functional in production.

Reference: DKRHUB_PLAN.md Phase 2, Task DKRHUB-2001 (lines 324-362)

## Acceptance Criteria
- [ ] `build:` section removed from maproom-mcp service in docker-compose.yml (lines 87-91)
- [ ] `image:` directive added: `crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}`
- [ ] Environment variable `MAPROOM_VERSION` supports version pinning with default `latest`
- [ ] Other services (postgres, ollama) remain unchanged
- [ ] File passes `docker-compose config` validation
- [ ] Comments document the change and version pinning option

## Technical Requirements
**File**: `packages/maproom-mcp/config/docker-compose.yml`

**Change at lines 87-91**:
```yaml
# BEFORE (BROKEN):
maproom-mcp:
  build:
    context: ../../..
    dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server

# AFTER (FIXED):
maproom-mcp:
  # Pull pre-built multi-platform image from Docker Hub
  # Set MAPROOM_VERSION environment variable to pin to specific version:
  #   MAPROOM_VERSION=1.1.10 (pin to exact version - recommended for production)
  #   MAPROOM_VERSION=1.1 (pin to minor, get patch updates)
  #   MAPROOM_VERSION=1 (pin to major, get minor updates)
  #   Default: latest (always newest release)
  image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
```

**Validation**:
```bash
# Validate compose file syntax
docker-compose -f packages/maproom-mcp/config/docker-compose.yml config

# Verify image reference resolves
docker-compose -f packages/maproom-mcp/config/docker-compose.yml pull maproom-mcp
```

## Implementation Notes
**Why This Fixes v1.1.9**:
- npm package contains docker-compose.yml in `config/` directory
- When user runs `npx @crewchief/maproom-mcp start`, CLI executes `docker-compose up`
- Old behavior: Tried to build from `context: ../../..` → fails (path doesn't exist)
- New behavior: Pulls `crewchief/maproom-mcp:latest` from Docker Hub → succeeds

**Version Pinning**:
Users can control which image version to use:
```bash
# Use latest (default)
npx @crewchief/maproom-mcp start

# Pin to specific version
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start
```

**Backwards Compatibility**:
- This is a breaking change for local development (no longer builds from source)
- Solution: docker-compose.override.yml (created in DKRHUB-2002)

Reference DKRHUB_ARCHITECTURE.md lines 286-411 for complete docker-compose specification.

## Dependencies
- DKRHUB-1901: Workflow must be tested and images published to Docker Hub
- Images must exist on Docker Hub before this change is released

## Risk Assessment
- **Risk**: Breaking local development workflows
  - **Mitigation**: DKRHUB-2002 creates override file for development builds
- **Risk**: Users pull wrong version
  - **Mitigation**: Document version pinning in README (DKRHUB-4004)
- **Risk**: Docker Hub unavailable during deployment
  - **Mitigation**: Docker automatically caches images locally; subsequent starts work offline

## Files/Packages Affected
- `packages/maproom-mcp/config/docker-compose.yml` (modify lines 87-91, add comments)
