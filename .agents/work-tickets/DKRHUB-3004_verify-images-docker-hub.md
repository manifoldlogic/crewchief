# Ticket: DKRHUB-3004: Verify Images on Docker Hub

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Verify that Docker images were successfully published to Docker Hub with correct tags, platforms, metadata, and that they can be pulled and run.

## Background
After the GitHub Actions workflow completes (DKRHUB-3003), we must verify the end result:
1. Images exist on Docker Hub
2. All version tags present (1.1.10, 1.1, 1, latest)
3. Multi-platform manifests include both AMD64 and ARM64
4. Metadata labels are correct
5. Images are publicly accessible and pullable

This confirms the entire CI/CD pipeline worked correctly.

Reference: DKRHUB_PLAN.md Phase 3, Task DKRHUB-3004 (lines 646-681)

## Acceptance Criteria
- [ ] Docker Hub repository visible at https://hub.docker.com/r/crewchief/maproom-mcp
- [ ] All four tags present: 1.1.10, 1.1, 1, latest
- [ ] Each tag has multi-arch manifest with linux/amd64 and linux/arm64
- [ ] Image metadata includes version, commit SHA, build date labels
- [ ] Total image size ~300MB (uncompressed), ~120MB (compressed)
- [ ] Images can be pulled: `docker pull crewchief/maproom-mcp:1.1.10`
- [ ] Pulled image runs successfully
- [ ] Docker Hub description and README updated (if configured in workflow)

## Technical Requirements
**Verification Steps**:

**1. Check Docker Hub Web UI**:
- URL: https://hub.docker.com/r/crewchief/maproom-mcp
- Navigate to "Tags" tab
- Verify tags: 1.1.10, 1.1, 1, latest
- Check "Last pushed" timestamp (should be recent)
- Verify architecture: Multi-architecture (2 images)

**2. Inspect Manifest** (shows platforms):
```bash
# View multi-arch manifest
docker manifest inspect crewchief/maproom-mcp:1.1.10

# Pretty print platforms
docker manifest inspect crewchief/maproom-mcp:1.1.10 | \
  jq '.manifests[] | {platform: .platform, size: .size}'

# Expected output:
# {
#   "platform": {
#     "architecture": "amd64",
#     "os": "linux"
#   },
#   "size": 120000000
# }
# {
#   "platform": {
#     "architecture": "arm64",
#     "os": "linux"
#   },
#   "size": 125000000
# }
```

**3. Pull and Inspect Image**:
```bash
# Pull specific version
docker pull crewchief/maproom-mcp:1.1.10

# Verify size
docker images crewchief/maproom-mcp:1.1.10
# Should show ~300MB

# Inspect metadata labels
docker inspect crewchief/maproom-mcp:1.1.10 \
  --format='{{json .Config.Labels}}' | jq

# Verify labels:
# - org.opencontainers.image.version: "1.1.10"
# - org.opencontainers.image.revision: "<git-sha>"
# - org.opencontainers.image.created: "2025-10-29T..."
# - org.opencontainers.image.title: "Maproom MCP Server"
# - org.opencontainers.image.vendor: "CrewChief"

# Check architecture
docker inspect crewchief/maproom-mcp:1.1.10 \
  --format='{{.Architecture}}'
# Should match host architecture (amd64 or arm64)
```

**4. Test Run**:
```bash
# Quick run test (should show help or start)
docker run --rm crewchief/maproom-mcp:1.1.10 node --version
# Should output Node.js version (v20.x.x)

# Test with environment variables
docker run --rm \
  -e DATABASE_URL=postgresql://test:test@localhost/test \
  crewchief/maproom-mcp:1.1.10 \
  node -e "console.log('OK')"
# Should output: OK
```

**5. Verify All Tags**:
```bash
# Pull each tag and verify they point to correct image
for tag in 1.1.10 1.1 1 latest; do
  echo "Pulling crewchief/maproom-mcp:$tag"
  docker pull crewchief/maproom-mcp:$tag

  echo "Image ID:"
  docker images crewchief/maproom-mcp:$tag --format "{{.ID}}"

  echo "Version label:"
  docker inspect crewchief/maproom-mcp:$tag \
    --format='{{index .Config.Labels "org.opencontainers.image.version"}}'

  echo "---"
done

# Verify tag relationships:
# - 1.1.10: Specific version
# - 1.1: Should point to latest 1.1.x (currently 1.1.10)
# - 1: Should point to latest 1.x.x (currently 1.1.10)
# - latest: Should point to newest release (currently 1.1.10)
```

## Implementation Notes
**Tag Verification**:
- All tags (1.1.10, 1.1, 1, latest) should currently point to the same image digest
- Future patch releases (1.1.11) will update 1.1 and 1 tags
- Future minor releases (1.2.0) will update 1 tag
- latest always points to most recent release

**Multi-Arch Verification**:
- `docker manifest inspect` shows all platforms
- Docker automatically selects platform matching host
- Test on multiple platforms if possible (AMD64 and ARM64)

**Image Size Expectations** (from DKRHUB_ARCHITECTURE.md):
- Compressed (download): ~120MB
- Uncompressed (on disk): ~300MB
- Base image (node:20-alpine): ~180MB
- Added layers: ~40MB (app code, dependencies)

**Common Issues**:
1. **Tag not found**: Workflow may still be running or failed
2. **Wrong architecture**: Manifest missing platform; check buildx config
3. **Huge image size**: Multi-stage build failed; check Dockerfile
4. **Missing labels**: Build args not passed; check workflow

Reference DKRHUB_QUALITY_STRATEGY.md lines 117-146 for image size validation criteria.

## Dependencies
- DKRHUB-3003: Workflow must complete successfully
- DKRHUB-3002: Tag must be pushed to trigger workflow

## Risk Assessment
- **Risk**: Images not published despite workflow success
  - **Mitigation**: Check workflow logs for push confirmation
- **Risk**: Only one platform published (AMD64 or ARM64 missing)
  - **Mitigation**: Verify QEMU and buildx configuration in workflow
- **Risk**: Wrong tags or metadata
  - **Mitigation**: Check version extraction logic in workflow

## Files/Packages Affected
- None (verification only, no code changes)
