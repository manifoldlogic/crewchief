# Ticket: DKRHUB-4004: Update README with Docker Hub Information

## Status
- [x] **Task completed** - acceptance criteria met (README updated in production)
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Update packages/maproom-mcp/README.md with comprehensive documentation about Docker Hub images, version pinning, multi-platform support, troubleshooting, and migration from v1.1.9.

## Background
Users need clear documentation on:
1. How Docker Hub images work (no longer building from source)
2. Version pinning options for production stability
3. Multi-platform support (AMD64, ARM64)
4. Troubleshooting common issues
5. Migration from broken v1.1.9

Good documentation reduces support burden and improves user experience.

Reference: DKRHUB_PLAN.md Phase 4, Task DKRHUB-4004 (lines 891-963)

## Acceptance Criteria
- [ ] Docker Hub section added with badge and repository link
- [ ] Supported platforms documented (linux/amd64, linux/arm64)
- [ ] Version pinning section with examples (exact, minor, major, latest)
- [ ] Installation instructions updated
- [ ] Troubleshooting section added with common issues and solutions
- [ ] Migration guide for v1.1.9 users
- [ ] Architecture diagram updated (if present) to show Docker Hub
- [ ] All examples tested and verified working
- [ ] README renders correctly on npmjs.com and GitHub

## Technical Requirements
**File**: `packages/maproom-mcp/README.md`

**Sections to Add/Update**:

### 1. Docker Hub Badge (at top)
```markdown
# Maproom MCP

[![npm version](https://badge.fury.io/js/@crewchief%2Fmaproom-mcp.svg)](https://www.npmjs.com/package/@crewchief/maproom-mcp)
[![Docker Hub](https://img.shields.io/docker/pulls/crewchief/maproom-mcp)](https://hub.docker.com/r/crewchief/maproom-mcp)

Semantic code search MCP server with local LLM embeddings - zero configuration required.
```

### 2. Docker Images Section (after Features)
```markdown
## Docker Images

Pre-built multi-platform Docker images are available on Docker Hub:

🐳 **Repository**: [crewchief/maproom-mcp](https://hub.docker.com/r/crewchief/maproom-mcp)

### Supported Platforms

- **linux/amd64** (x86_64) - Linux servers, Intel Macs, Windows WSL2
- **linux/arm64** (ARM64) - Apple Silicon Macs (M1/M2/M3), AWS Graviton

Docker automatically selects the correct platform for your system.

### Image Tags

| Tag | Description | Use Case |
|-----|-------------|----------|
| `latest` | Most recent release | Development, testing |
| `1` | Latest 1.x.x release | Production (auto-updates to 1.x.x) |
| `1.1` | Latest 1.1.x patch | Production (auto-updates to 1.1.x) |
| `1.1.10` | Specific version | Production (pinned, no updates) |
```

### 3. Version Pinning Section (after Installation)
```markdown
## Version Pinning

By default, Maproom MCP uses the `latest` image tag. For production deployments, pin to a specific version:

### Pin to Exact Version (Recommended for Production)
```bash
# Pin to 1.1.10 - image never changes
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start
```

### Pin to Minor Version
```bash
# Pin to 1.1.x - get patch updates automatically
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start
```

### Pin to Major Version
```bash
# Pin to 1.x.x - get minor and patch updates
MAPROOM_VERSION=1 npx @crewchief/maproom-mcp start
```

### Use Latest (Default)
```bash
# Always use newest release
npx @crewchief/maproom-mcp start
# or explicitly:
MAPROOM_VERSION=latest npx @crewchief/maproom-mcp start
```

**Configuration File**: Set `MAPROOM_VERSION` in your `.env` file or shell profile for persistent pinning.
```

### 4. Troubleshooting Section
```markdown
## Troubleshooting

### Image Pull Failures

**Symptom**: Error pulling images from Docker Hub

**Solutions**:
1. Check Docker is running: `docker ps`
2. Test Docker Hub connectivity: `docker pull hello-world`
3. Try manual pull: `docker pull crewchief/maproom-mcp:latest`
4. Check Docker Hub status: https://status.docker.com
5. If behind proxy, configure Docker proxy settings

### Wrong Architecture on Apple Silicon

**Symptom**: Performance issues or "This image requires Rosetta" warning

**Solution**: Verify ARM64 image is being used:
```bash
docker inspect maproom-mcp --format='{{.Architecture}}'
# Should output: arm64 (not amd64)
```

If showing `amd64`, pull the correct image:
```bash
docker pull --platform linux/arm64 crewchief/maproom-mcp:latest
```

### Port Conflicts

**Symptom**: Error "port is already allocated"

**Solutions**:
1. Check what's using the port:
   ```bash
   # PostgreSQL port
   lsof -i :5433

   # Ollama port
   lsof -i :11434
   ```
2. Stop conflicting service or configure different ports in docker-compose.yml

### Services Not Starting

**Symptom**: Containers exit immediately or health checks fail

**Solutions**:
1. Check logs: `docker logs maproom-mcp`
2. Check disk space: `df -h`
3. Check memory: `docker stats`
4. Clean up old containers: `docker system prune`
5. Restart Docker daemon

### Migration from v1.1.9

**Symptom**: v1.1.9 fails with "lstat /packages: no such file or directory"

**Solution**: Upgrade to v1.1.10:
```bash
# Stop broken v1.1.9 services (if running)
npx @crewchief/maproom-mcp stop 2>/dev/null || true

# Update to v1.1.10
npm install -g @crewchief/maproom-mcp@latest

# Start services (now pulls from Docker Hub)
npx @crewchief/maproom-mcp start
```

**Note**: v1.1.9 is broken and should not be used. Skip directly to v1.1.10.
```

### 5. Update Quick Start
```markdown
## Quick Start

### 1. Install

```bash
npm install -g @crewchief/maproom-mcp
```

### 2. Start Services

```bash
# Start all services (PostgreSQL, Ollama, MCP server)
npx @crewchief/maproom-mcp start

# Docker will automatically:
# - Pull pre-built images from Docker Hub (~200MB)
# - Start PostgreSQL with pgvector
# - Download and start Ollama with nomic-embed-text model
# - Launch MCP server

# First start takes 2-3 minutes (image download)
# Subsequent starts take <30 seconds (cached images)
```

### 3. Verify

```bash
# Check services are running
docker ps

# Expected output:
# - maproom-postgres (healthy)
# - maproom-ollama (healthy)
# - maproom-mcp (running)
```
```

### 6. Architecture Diagram Update (if exists)
Update any architecture diagrams to show:
- Docker Hub as image source
- No local build steps
- Multi-platform support

## Implementation Notes
**README Best Practices**:
- Use clear, concise language
- Include code examples with expected output
- Link to external resources (Docker Hub, Docker docs)
- Use badges for visual appeal
- Organize with clear headings
- Test all commands before documenting

**NPM Package Display**:
- npmjs.com renders README from package
- GitHub renders README from repository
- Ensure links work in both contexts (use absolute URLs for GitHub-specific links)

**Version-Specific Documentation**:
- Current examples show v1.1.10
- Future versions update examples accordingly
- Keep documentation up-to-date with each release

Reference DKRHUB_PLAN.md lines 915-960 for specific sections to add.

## Dependencies
- DKRHUB-3005: npm package should be published (so README changes are visible)
- DKRHUB-4001, DKRHUB-4002, DKRHUB-4003: Testing complete (examples verified)

## Risk Assessment
- **Risk**: Incorrect examples confuse users
  - **Mitigation**: Test all commands before documenting
- **Risk**: README too long and overwhelming
  - **Mitigation**: Use clear sections, table of contents, concise language
- **Risk**: Links break over time
  - **Mitigation**: Use stable URLs (Docker Hub, GitHub), periodic review

## Files/Packages Affected
- `packages/maproom-mcp/README.md` (comprehensive updates)
