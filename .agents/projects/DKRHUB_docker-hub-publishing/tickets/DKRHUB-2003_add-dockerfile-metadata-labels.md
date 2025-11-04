# Ticket: DKRHUB-2003: Add Dockerfile Metadata Labels

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add build arguments (ARG) and OCI-compliant labels (LABEL) to Dockerfile.mcp-server for version tracking, traceability, and metadata standardization.

## Background
Docker images should contain metadata about their build:
- Version: Which release this is (1.1.10)
- Revision: Git commit SHA for traceability
- Created: Build timestamp
- Title, Vendor: Descriptive information

These labels enable tools to query image metadata and help with debugging and support.

Reference: DKRHUB_PLAN.md Phase 2, Task DKRHUB-2003 (lines 404-442)

## Acceptance Criteria
- [x] ARG declarations added at top of Dockerfile (after first FROM): VERSION, COMMIT_SHA, BUILD_DATE
- [x] LABEL directives added before ENTRYPOINT in final stage (runtime stage)
- [x] Labels follow OCI image spec: `org.opencontainers.image.*`
- [x] Required labels: version, revision, created, title, vendor
- [x] Default values provided for ARGs (e.g., "unknown") for local builds
- [x] Build succeeds with and without build arguments provided

## Technical Requirements
**File**: `packages/maproom-mcp/config/Dockerfile.mcp-server`

**Changes**:

1. **Add ARGs after first FROM** (in builder stage):
```dockerfile
# ========================================
# Stage 1: Build Stage
# ========================================
FROM node:20-alpine AS builder

# Build arguments for metadata (passed from GitHub Actions)
ARG VERSION=unknown
ARG COMMIT_SHA=unknown
ARG BUILD_DATE=unknown

# ... rest of builder stage ...
```

2. **Add LABELs before ENTRYPOINT** (in runtime stage, after USER node):
```dockerfile
# ========================================
# Stage 2: Runtime Stage
# ========================================
FROM node:20-alpine

# ... install dependencies, copy files ...

# Switch to non-root user
USER node

# OCI-compliant image metadata labels
LABEL org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${COMMIT_SHA}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.title="Maproom MCP Server" \
      org.opencontainers.image.description="Semantic code search MCP server with local LLM embeddings" \
      org.opencontainers.image.vendor="CrewChief" \
      org.opencontainers.image.source="https://github.com/danielbushman/crewchief" \
      org.opencontainers.image.licenses="MIT"

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD pg_isready -h postgres -U maproom || exit 1

# Entrypoint
ENTRYPOINT ["node", "/app/dist/index.js"]
```

**Testing**:
```bash
# Build with arguments (as GitHub Actions does)
docker build \
  --build-arg VERSION=1.1.10 \
  --build-arg COMMIT_SHA=$(git rev-parse HEAD) \
  --build-arg BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ") \
  -f packages/maproom-mcp/config/Dockerfile.mcp-server \
  -t test-labels \
  packages/maproom-mcp

# Inspect labels
docker inspect test-labels --format='{{json .Config.Labels}}' | jq

# Build without arguments (local dev)
docker build \
  -f packages/maproom-mcp/config/Dockerfile.mcp-server \
  -t test-no-args \
  packages/maproom-mcp

# Should show "unknown" for VERSION, COMMIT_SHA, BUILD_DATE
docker inspect test-no-args --format='{{json .Config.Labels}}' | jq
```

## Implementation Notes
**OCI Image Spec**:
The Open Container Initiative (OCI) defines standard labels for container images. Using these ensures compatibility with container registries, security scanners, and management tools.

Standard labels:
- `org.opencontainers.image.version`: Semantic version (1.1.10)
- `org.opencontainers.image.revision`: VCS commit (git SHA)
- `org.opencontainers.image.created`: ISO 8601 timestamp
- `org.opencontainers.image.title`: Human-readable title
- `org.opencontainers.image.description`: Longer description
- `org.opencontainers.image.vendor`: Organization name
- `org.opencontainers.image.source`: Repository URL
- `org.opencontainers.image.licenses`: SPDX license identifier

**Build Argument Flow**:
1. GitHub Actions passes: `--build-arg VERSION=1.1.10`
2. Dockerfile receives: `ARG VERSION=unknown` (default if not provided)
3. Label uses: `${VERSION}` (interpolated at build time)
4. Result in image: `"org.opencontainers.image.version": "1.1.10"`

**Why Default Values**:
Local developers building without CI/CD don't provide build arguments. Defaults prevent build failures and show "unknown" in labels.

Reference DKRHUB_ARCHITECTURE.md lines 453-542 for complete Dockerfile specification.

## Dependencies
- None (can be implemented independently)
- Works with DKRHUB-1005 which passes build arguments from workflow

## Risk Assessment
- **Risk**: Labels increase image size
  - **Mitigation**: Labels add <1KB, negligible impact
- **Risk**: Missing ARGs cause build failure
  - **Mitigation**: Default values ensure builds always succeed
- **Risk**: Incorrect label format
  - **Mitigation**: Follow OCI spec exactly, test with `docker inspect`

## Files/Packages Affected
- `packages/maproom-mcp/config/Dockerfile.mcp-server` (add ARGs and LABELs)

## Implementation Notes

### Changes Made
1. Added ARG declarations after first FROM in builder stage (lines 7-9):
   - `ARG VERSION=unknown`
   - `ARG COMMIT_SHA=unknown`
   - `ARG BUILD_DATE=unknown`

2. Re-declared ARGs in runtime stage (lines 42-44) to make them available for LABEL directives

3. Added OCI-compliant LABEL directives in runtime stage after USER node, before HEALTHCHECK (lines 77-85):
   - All 8 required labels following org.opencontainers.image.* format
   - Proper interpolation using ${VERSION}, ${COMMIT_SHA}, ${BUILD_DATE}

### Testing Performed
1. Build without arguments (local dev scenario):
   ```bash
   docker build -f packages/maproom-mcp/config/Dockerfile.mcp-server -t test-no-args packages/maproom-mcp
   docker inspect test-no-args --format='{{json .Config.Labels}}' | jq
   ```
   Result: All labels present with "unknown" default values

2. Build with arguments (CI/CD scenario):
   ```bash
   docker build \
     --build-arg VERSION=1.1.10 \
     --build-arg COMMIT_SHA=$(git rev-parse HEAD) \
     --build-arg BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ") \
     -f packages/maproom-mcp/config/Dockerfile.mcp-server \
     -t test-with-args packages/maproom-mcp
   docker inspect test-with-args --format='{{json .Config.Labels}}' | jq
   ```
   Result: All labels present with actual values (VERSION=1.1.10, COMMIT_SHA=48e98e2e..., BUILD_DATE=2025-10-30T01:47:43Z)

### Verification Steps
To verify this implementation:
1. Build the Dockerfile without arguments - should succeed
2. Build the Dockerfile with arguments - should succeed
3. Inspect labels with `docker inspect <image> --format='{{json .Config.Labels}}' | jq`
4. Verify all 8 OCI labels are present:
   - org.opencontainers.image.version
   - org.opencontainers.image.revision
   - org.opencontainers.image.created
   - org.opencontainers.image.title
   - org.opencontainers.image.description
   - org.opencontainers.image.vendor
   - org.opencontainers.image.source
   - org.opencontainers.image.licenses

All acceptance criteria have been met.
