# Ticket: DKRHUB-4005: Create Migration Guide v1.1.9 to v1.1.10

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive migration guide documenting the upgrade path from broken v1.1.9 to fixed v1.1.10, including what changed, how to upgrade, rollback procedures, and common issues.

## Background
Users who attempted to install v1.1.9 experienced deployment failures. This guide helps them:
1. Understand what went wrong
2. Upgrade safely to v1.1.10
3. Verify the fix worked
4. Rollback if needed
5. Avoid common pitfalls

Clear migration documentation reduces support burden and builds user confidence.

Reference: DKRHUB_PLAN.md Phase 4, Task DKRHUB-4005 (lines 965-1033)

## Acceptance Criteria
- [ ] Migration guide created at `packages/maproom-mcp/docs/MIGRATION_v1.1.10.md`
- [ ] Documents what changed between v1.1.9 and v1.1.10
- [ ] Clear upgrade steps provided
- [ ] Rollback procedure documented
- [ ] Common issues and solutions included
- [ ] Breaking changes section (none expected, but documented)
- [ ] For users section (what they see) and for developers section (what changed under the hood)
- [ ] Guide linked from main README

## Technical Requirements
**File**: `packages/maproom-mcp/docs/MIGRATION_v1.1.10.md`

**Content Structure**:

```markdown
# Migration Guide: v1.1.9 → v1.1.10

**TL;DR**: v1.1.9 is broken. Upgrade to v1.1.10 immediately.

```bash
npm install -g @crewchief/maproom-mcp@latest
npx @crewchief/maproom-mcp start
```

---

## Overview

### What Went Wrong with v1.1.9

v1.1.9 introduced a critical deployment failure:

**Problem**: docker-compose.yml attempted to build the `maproom-mcp` Docker image from source using a build context (`../../..`) that only exists in the development workspace.

**Impact**: When users installed v1.1.9 via npm (globally or locally), the package was extracted to `~/.npm-packages/` or `node_modules/`. The docker-compose.yml tried to build from `../../../packages/maproom-mcp/` which didn't exist.

**Error**: `lstat /packages: no such file or directory`

**Result**: Services failed to start. Package was completely unusable.

### What Changed in v1.1.10

**Fix**: docker-compose.yml now pulls pre-built images from Docker Hub instead of building from source.

**Benefits**:
- ✅ Works from any installation location (npm global, npm local, npx)
- ✅ Faster startup (~30s vs ~10min for build)
- ✅ Multi-platform support (AMD64, ARM64)
- ✅ Version pinning for production stability
- ✅ Reproducible deployments (same image everywhere)

---

## Upgrade Steps

### For Users Who Attempted v1.1.9

#### Step 1: Stop Broken Services (if running)

```bash
# Attempt to stop services (may fail if broken)
npx @crewchief/maproom-mcp stop 2>/dev/null || true

# If that fails, manually stop Docker containers
docker stop maproom-mcp maproom-postgres maproom-ollama 2>/dev/null || true
docker rm maproom-mcp maproom-postgres maproom-ollama 2>/dev/null || true
```

#### Step 2: Update Package

```bash
# Uninstall broken version
npm uninstall -g @crewchief/maproom-mcp

# Install fixed version
npm install -g @crewchief/maproom-mcp@latest

# Verify version
maproom-mcp --version
# Should show: 1.1.10 (or higher)
```

#### Step 3: Start Services

```bash
# Clean slate (optional but recommended)
docker volume prune -f  # Removes old data

# Start services
npx @crewchief/maproom-mcp start

# Wait for services to start (1-2 minutes first time)
# Docker will pull images from Docker Hub
```

#### Step 4: Verify

```bash
# Check all services are running
docker ps

# Expected output:
# - maproom-postgres (Up, healthy)
# - maproom-ollama (Up)
# - maproom-mcp (Up)

# Check logs
docker logs maproom-mcp
# Should show successful startup, no "lstat /packages" errors

# Verify image source
docker inspect maproom-mcp --format='{{.Config.Image}}'
# Should show: crewchief/maproom-mcp:latest
```

### For Users Upgrading from v1.1.8 or Earlier

```bash
# Stop existing services
npx @crewchief/maproom-mcp stop

# Update package
npm install -g @crewchief/maproom-mcp@latest

# Start services (now with Docker Hub images)
npx @crewchief/maproom-mcp start
```

**Note**: Data persists across upgrades (stored in Docker volumes).

---

## Breaking Changes

### None for End Users

- API: Unchanged
- Environment variables: Unchanged
- CLI commands: Unchanged
- Data storage: Unchanged
- Functionality: Unchanged

### For Contributors (Development Workflow)

**Changed**: Local development now requires docker-compose.override.yml

**Before (v1.1.9)**:
```bash
cd packages/maproom-mcp/config
docker-compose up -d  # Built from source automatically
```

**After (v1.1.10)**:
```bash
cd packages/maproom-mcp/config
# Create override file for development
cat > docker-compose.override.yml <<EOF
services:
  maproom-mcp:
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
EOF

# Now build and run
docker-compose build
docker-compose up -d
```

**Rationale**: Production uses Docker Hub images (fast, reliable), development can still build from source (for testing changes).

---

## Rollback

If issues occur with v1.1.10 (unlikely):

### Rollback to v1.1.8

v1.1.9 is broken, so rollback to v1.1.8:

```bash
# Stop v1.1.10
npx @crewchief/maproom-mcp stop

# Install v1.1.8
npm install -g @crewchief/maproom-mcp@1.1.8

# Start services
npx @crewchief/maproom-mcp start
```

**Note**: v1.1.8 still builds from source, so requires:
- Git repository cloned
- Source code available
- Slower startup

---

## Common Issues

### Issue: "manifest unknown" Error

**Symptom**:
```
Error response from daemon: manifest for crewchief/maproom-mcp:latest not found
```

**Cause**: Docker Hub image not yet published or Docker Hub connectivity issue

**Solution**:
1. Verify image exists: https://hub.docker.com/r/crewchief/maproom-mcp
2. Check Docker Hub status: https://status.docker.com
3. Try manual pull: `docker pull crewchief/maproom-mcp:latest`
4. Wait 5 minutes and retry (images may still be publishing)

### Issue: Old Image Cached

**Symptom**: Still seeing v1.1.9 behavior after upgrade

**Cause**: Docker cached old image

**Solution**:
```bash
# Force pull latest image
docker pull crewchief/maproom-mcp:latest

# Or remove all old images
docker rmi crewchief/maproom-mcp:latest
npx @crewchief/maproom-mcp start  # Will pull fresh
```

### Issue: Port Already in Use

**Symptom**: `port is already allocated`

**Cause**: Old v1.1.9 containers still running

**Solution**:
```bash
# Stop all maproom containers
docker stop $(docker ps -q --filter "name=maproom")
docker rm $(docker ps -aq --filter "name=maproom")

# Start fresh
npx @crewchief/maproom-mcp start
```

---

## Version Pinning

For production stability, pin to specific version:

```bash
# Pin to exact version (recommended)
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start

# Or set in .env file
echo "MAPROOM_VERSION=1.1.10" >> ~/.maproom-mcp/.env
```

See [README](../README.md#version-pinning) for more version pinning options.

---

## FAQ

**Q: Will I lose my data upgrading from v1.1.9 to v1.1.10?**
A: No. Data is stored in Docker volumes which persist across upgrades.

**Q: Do I need to reindex my code?**
A: No. The database schema hasn't changed.

**Q: Can I stay on v1.1.8?**
A: Yes, but you'll miss bug fixes, performance improvements, and new features. v1.1.10 is recommended.

**Q: Why wasn't v1.1.9 tested before release?**
A: v1.1.9 worked in development (monorepo structure) but failed in deployment (npm package structure). v1.1.10 includes deployment testing.

**Q: How do I know if I'm using Docker Hub images?**
A: Check with: `docker inspect maproom-mcp --format='{{.Config.Image}}'`
Should show `crewchief/maproom-mcp:latest` (not a local image name).

---

## Support

If you encounter issues not covered in this guide:

1. Check logs: `docker logs maproom-mcp`
2. Search issues: https://github.com/danielbushman/crewchief/issues
3. Report bug: https://github.com/danielbushman/crewchief/issues/new

---

**Last Updated**: 2025-10-29
**Migration Path**: v1.1.9 → v1.1.10
**Status**: v1.1.9 broken, v1.1.10 fixes deployment
```

**Add to README.md**:
```markdown
## Migration

Upgrading from v1.1.9? See [Migration Guide](docs/MIGRATION_v1.1.10.md).
```

## Implementation Notes
**Guide Purpose**:
- Explain what went wrong (builds trust)
- Provide clear steps (reduces confusion)
- Address common issues (reduces support)
- Document for developers too (not just users)

**Tone**:
- Clear and direct ("v1.1.9 is broken")
- Apologetic but solution-focused
- Technical but accessible
- Confidence-building ("this fixes it")

**Audience**:
- Primary: Users who hit v1.1.9 bug
- Secondary: Users on older versions
- Tertiary: Contributors (development workflow change)

Reference DKRHUB_PLAN.md lines 988-1031 for migration guide outline.

## Dependencies
- DKRHUB-4004: README should reference this migration guide
- DKRHUB-4001, DKRHUB-4002: E2E tests validate migration steps work

## Risk Assessment
- **Risk**: Migration steps don't work
  - **Mitigation**: Test all commands on fresh system before documenting
- **Risk**: Guide too technical or confusing
  - **Mitigation**: Use simple language, step-by-step format, clear examples

## Files/Packages Affected
- NEW: `packages/maproom-mcp/docs/MIGRATION_v1.1.10.md`
- UPDATE: `packages/maproom-mcp/README.md` (add link to migration guide)
