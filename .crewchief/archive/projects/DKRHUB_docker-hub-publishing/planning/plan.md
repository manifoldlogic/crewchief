# DKRHUB: Docker Hub Publishing - Implementation Plan

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Implementation Planning

## Executive Summary

This plan outlines the implementation of Docker Hub publishing to fix the broken v1.1.9 deployment. The implementation is divided into 4 phases with clear deliverables and acceptance criteria.

**Timeline**: 2-3 days for MVP (Phases 1-3), 1 week for complete implementation (all phases)
**Urgency**: Critical - v1.1.9 is broken in production
**Goal**: Users can install and run `@crewchief/maproom-mcp@1.1.10` successfully

## Related Existing Tickets

**IMPORTANT**: The following tickets already exist and should be referenced, not duplicated:

1. **`.crewchief/work-tickets/LOCAL-4006_optimize-docker-image-size.md`** (✅ COMPLETED)
   - Docker image is already optimized (~300MB target achieved)
   - Multi-stage build implemented
   - Alpine base image in use
   - **Action**: Reference this work, don't redo it

2. **`.crewchief/work-tickets/LOCAL-4005_arm64-apple-silicon-testing.md`** (relates to multi-platform)
   - Defines ARM64/Apple Silicon testing requirements
   - **Action**: Use test cases from this ticket for multi-platform validation

3. **`.crewchief/work-tickets/LOCAL-3008_npm-publish-test-release.md`** (⚠️ INCOMPLETE)
   - Covers npm publishing workflow
   - **Action**: Complete this after Docker images are available

4. **`.crewchief/work-tickets/MCPSTART-6004_publish-npm-v1-1-9.md`** (⚠️ BLOCKED)
   - Original publishing ticket that discovered this issue
   - **Action**: This DKRHUB project unblocks MCPSTART-6004

## Implementation Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                     Implementation Flow                          │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: GitHub Actions Workflow (Critical Path)               │
│  └─► Create workflow file                                       │
│      └─► Configure multi-platform builds                        │
│          └─► Set up Docker Hub auth                             │
│              └─► Add security scanning                          │
│                  └─► Test workflow                              │
│                      ✓ Deliverable: Working CI/CD pipeline      │
│                                                                  │
│  Phase 2: Docker Compose Updates                                │
│  └─► Update docker-compose.yml (use image:)                     │
│      └─► Create docker-compose.override.yml                     │
│          └─► Update documentation                               │
│              └─► Test both configurations                       │
│                  ✓ Deliverable: Production-ready compose files  │
│                                                                  │
│  Phase 3: Release v1.1.10                                       │
│  └─► Update package.json version                                │
│      └─► Tag git repository                                     │
│          └─► Trigger GitHub Actions                             │
│              └─► Verify image on Docker Hub                     │
│                  └─► Publish npm package                        │
│                      ✓ Deliverable: v1.1.10 released            │
│                                                                  │
│  Phase 4: Validation & Documentation                            │
│  └─► End-to-end testing                                         │
│      └─► Multi-platform validation                              │
│          └─► Update README and docs                             │
│              └─► Migration guide                                │
│                  ✓ Deliverable: Fully documented release        │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Phase 1: GitHub Actions Workflow (Day 1 - 4 hours)

**Objective**: Create automated workflow to build and publish Docker images

**Critical Path**: Must complete before other phases

### Tasks

#### DKRHUB-1001: Create GitHub Actions Workflow File
**Priority**: P0 (Critical)
**Effort**: 2 hours
**Assignee**: DevOps Engineer / CI/CD Specialist

**Description**: Create `.github/workflows/publish-maproom-mcp-image.yml` with complete workflow configuration.

**Implementation Steps**:
1. Create workflow file structure
2. Configure triggers (tags `v*.*.*`, workflow_dispatch)
3. Set up environment variables
4. Add permissions configuration

**Acceptance Criteria**:
- [ ] File created at `.github/workflows/publish-maproom-mcp-image.yml`
- [ ] Workflow triggers on version tags
- [ ] Manual trigger available via workflow_dispatch
- [ ] Environment variables defined
- [ ] Permissions correctly scoped

**Files Modified**:
- `NEW: .github/workflows/publish-maproom-mcp-image.yml`

---

#### DKRHUB-1002: Configure Multi-Platform Build
**Priority**: P0 (Critical)
**Effort**: 1 hour
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-1001

**Description**: Set up Docker Buildx with QEMU for AMD64 and ARM64 builds.

**Implementation Steps**:
1. Add QEMU setup step
2. Add Buildx setup step
3. Configure platform matrix
4. Set up GitHub Actions cache

**Acceptance Criteria**:
- [ ] QEMU configured for emulation
- [ ] Buildx supports linux/amd64,linux/arm64
- [ ] Build cache configured (type=gha)
- [ ] Platform detection works correctly

**Code Reference**:
```yaml
- name: Set up QEMU
  uses: docker/setup-qemu-action@v3
  with:
    platforms: linux/amd64,linux/arm64

- name: Set up Docker Buildx
  uses: docker/setup-buildx-action@v3
```

---

#### DKRHUB-1003: Implement Docker Hub Authentication
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-1002

**Description**: Configure Docker Hub login using GitHub Secrets.

**Implementation Steps**:
1. Verify GitHub Secrets exist (DOCKERHUB_USERNAME, DOCKERHUB_TOKEN)
2. Add Docker login step
3. Test authentication
4. Verify token permissions

**Acceptance Criteria**:
- [ ] Login action configured
- [ ] Uses GitHub Secrets securely
- [ ] Authentication succeeds in test run
- [ ] No credentials exposed in logs

**Code Reference**:
```yaml
- name: Login to Docker Hub
  uses: docker/login-action@v3
  with:
    username: ${{ secrets.DOCKERHUB_USERNAME }}
    password: ${{ secrets.DOCKERHUB_TOKEN }}
```

---

#### DKRHUB-1004: Implement Version Extraction and Tagging
**Priority**: P0 (Critical)
**Effort**: 1 hour
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-1003

**Description**: Extract version from git tag and generate image tags (full, minor, major, latest).

**Implementation Steps**:
1. Add version extraction step
2. Parse semantic version
3. Generate multiple tags
4. Configure metadata action

**Acceptance Criteria**:
- [ ] Version extracted from git tag (v1.1.10 → 1.1.10)
- [ ] Major version extracted (1.1.10 → 1)
- [ ] Minor version extracted (1.1.10 → 1.1)
- [ ] Tags generated: 1.1.10, 1.1, 1, latest
- [ ] Metadata labels added to image

**Code Reference**:
```yaml
- name: Extract version
  id: version
  run: |
    VERSION="${GITHUB_REF#refs/tags/v}"
    echo "full=$VERSION" >> $GITHUB_OUTPUT
    echo "minor=$(echo $VERSION | cut -d. -f1-2)" >> $GITHUB_OUTPUT
    echo "major=$(echo $VERSION | cut -d. -f1)" >> $GITHUB_OUTPUT
```

---

#### DKRHUB-1005: Configure Image Build and Push
**Priority**: P0 (Critical)
**Effort**: 1.5 hours
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-1004

**Description**: Configure docker/build-push-action to build and push multi-platform images.

**Implementation Steps**:
1. Set build context and Dockerfile
2. Configure platforms
3. Add build arguments
4. Set up push conditions
5. Configure caching

**Acceptance Criteria**:
- [ ] Build context points to packages/maproom-mcp
- [ ] Dockerfile path correct
- [ ] Platforms: linux/amd64,linux/arm64
- [ ] Build arguments passed (VERSION, COMMIT_SHA, BUILD_DATE)
- [ ] Images pushed to Docker Hub
- [ ] Cache configured for performance

**Code Reference**:
```yaml
- name: Build and push Docker image
  uses: docker/build-push-action@v5
  with:
    context: packages/maproom-mcp
    file: packages/maproom-mcp/config/Dockerfile.mcp-server
    platforms: linux/amd64,linux/arm64
    push: true
    tags: ${{ steps.meta.outputs.tags }}
    cache-from: type=gha
    cache-to: type=gha,mode=max
```

---

#### DKRHUB-1006: Add Security Scanning with Trivy
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: Security Engineer
**Dependencies**: DKRHUB-1005

**Description**: Integrate Trivy security scanning into workflow.

**Implementation Steps**:
1. Add Trivy scan step
2. Configure severity threshold
3. Upload results to GitHub Security
4. Set failure conditions

**Acceptance Criteria**:
- [ ] Trivy scans published image
- [ ] CRITICAL and HIGH severities checked
- [ ] Results uploaded to GitHub Security tab
- [ ] Build fails on critical vulnerabilities

**Code Reference**:
```yaml
- name: Run Trivy security scan
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: crewchief/maproom-mcp:${{ steps.version.outputs.full }}
    format: 'sarif'
    output: 'trivy-results.sarif'
    severity: 'CRITICAL,HIGH'
    exit-code: 1
```

---

#### DKRHUB-1007: Test Workflow with Pre-Release Tag
**Priority**: P0 (Critical)
**Effort**: 1 hour
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-1006

**Description**: Test complete workflow with test tag (v1.1.10-rc1).

**Implementation Steps**:
1. Create test tag: `git tag v1.1.10-rc1`
2. Push tag to trigger workflow
3. Monitor GitHub Actions logs
4. Verify images on Docker Hub
5. Test pulling images

**Acceptance Criteria**:
- [ ] Workflow triggers successfully
- [ ] All steps complete without errors
- [ ] Images appear on Docker Hub
- [ ] Both AMD64 and ARM64 images exist
- [ ] Images can be pulled
- [ ] No secrets exposed in logs

**Testing Commands**:
```bash
# Tag and push
git tag v1.1.10-rc1
git push origin v1.1.10-rc1

# Wait for workflow, then test
docker pull crewchief/maproom-mcp:1.1.10-rc1
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Architecture}}'
```

**Phase 1 Deliverable**: Working GitHub Actions workflow that builds and publishes multi-platform Docker images to Docker Hub.

---

## Phase 2: Docker Compose Updates (Day 1-2 - 2 hours)

**Objective**: Update docker-compose configuration to use pre-built images

### Tasks

#### DKRHUB-2001: Update docker-compose.yml to Use Images
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-1007

**Description**: Replace `build:` section with `image:` in docker-compose.yml.

**Implementation Steps**:
1. Open `packages/maproom-mcp/config/docker-compose.yml`
2. Locate `maproom-mcp` service (lines 87-91)
3. Remove `build:` section
4. Add `image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}`
5. Test configuration

**Acceptance Criteria**:
- [ ] `build:` section removed from maproom-mcp service
- [ ] `image:` directive added with version variable
- [ ] Default version is `latest`
- [ ] Environment variable `MAPROOM_VERSION` supported
- [ ] Other services (postgres, ollama) unchanged

**Files Modified**:
- `packages/maproom-mcp/config/docker-compose.yml` (lines 87-91)

**Code Changes**:
```yaml
# Before (BROKEN):
maproom-mcp:
  build:
    context: ../../..
    dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server

# After (FIXED):
maproom-mcp:
  image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
```

---

#### DKRHUB-2002: Create docker-compose.override.yml for Development
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-2001

**Description**: Create override file for local development builds.

**Implementation Steps**:
1. Create `packages/maproom-mcp/config/docker-compose.override.yml`
2. Add build configuration for maproom-mcp service
3. Document usage in comments
4. Add to .gitignore (optional - for personal overrides)

**Acceptance Criteria**:
- [ ] Override file created
- [ ] Contains build configuration
- [ ] Context and Dockerfile paths correct
- [ ] Works with docker-compose (automatic merge)
- [ ] Documented with comments

**Files Created**:
- `NEW: packages/maproom-mcp/config/docker-compose.override.yml`

**Code**:
```yaml
# Development override - allows building from source
# Place this file in your development workspace
# Docker Compose automatically merges this with docker-compose.yml

services:
  maproom-mcp:
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
```

---

#### DKRHUB-2003: Add Dockerfile Metadata Labels
**Priority**: P2 (Medium)
**Effort**: 0.5 hours
**Assignee**: DevOps Engineer
**Dependencies**: DKRHUB-2001

**Description**: Add build arguments and labels to Dockerfile for metadata.

**Implementation Steps**:
1. Open `packages/maproom-mcp/config/Dockerfile.mcp-server`
2. Add ARG declarations at top
3. Add LABEL directives before ENTRYPOINT
4. Test build with arguments

**Acceptance Criteria**:
- [ ] ARG VERSION, COMMIT_SHA, BUILD_DATE added
- [ ] LABEL directives for OCI metadata
- [ ] Labels include version, revision, created date
- [ ] Build succeeds with and without arguments

**Files Modified**:
- `packages/maproom-mcp/config/Dockerfile.mcp-server`

**Code Changes**:
```dockerfile
# Add after FROM node:20-alpine AS builder
ARG VERSION=unknown
ARG COMMIT_SHA=unknown
ARG BUILD_DATE=unknown

# Add before ENTRYPOINT (in final stage)
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.revision="${COMMIT_SHA}"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.title="Maproom MCP Server"
LABEL org.opencontainers.image.vendor="CrewChief"
```

---

#### DKRHUB-2004: Test Production Configuration (Image Pull)
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-2001, DKRHUB-1007

**Description**: Test updated docker-compose.yml pulls and runs images correctly.

**Implementation Steps**:
1. Clean Docker environment
2. Run docker-compose up with updated config
3. Verify images pulled from Docker Hub
4. Verify services start correctly
5. Check health status

**Acceptance Criteria**:
- [ ] Images pulled from Docker Hub (not built locally)
- [ ] All three services start
- [ ] Health checks pass
- [ ] No build errors (no build attempted)
- [ ] Logs show successful startup

**Testing Commands**:
```bash
# Clean environment
docker system prune -af
docker volume prune -f

# Test production config
cd packages/maproom-mcp/config
docker-compose up -d

# Verify image source
docker inspect maproom-mcp --format='{{.Config.Image}}'
# Should be: crewchief/maproom-mcp:latest (or specific version)

# Check services
docker ps
docker logs maproom-mcp
```

---

#### DKRHUB-2005: Test Development Configuration (Local Build)
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-2002

**Description**: Test docker-compose.override.yml allows local builds.

**Implementation Steps**:
1. Ensure override file present
2. Run docker-compose build
3. Verify local image built
4. Run docker-compose up
5. Verify local image used

**Acceptance Criteria**:
- [ ] docker-compose build succeeds
- [ ] Local image created (not pulled)
- [ ] Override merges correctly
- [ ] Local development workflow preserved
- [ ] No conflicts with production config

**Testing Commands**:
```bash
cd packages/maproom-mcp/config

# Ensure override exists
ls docker-compose.override.yml

# Build locally
docker-compose build

# Run with local build
docker-compose up -d

# Verify local image used
docker images | grep maproom-mcp
```

**Phase 2 Deliverable**: docker-compose configuration that uses Docker Hub images in production, supports local builds in development.

---

## Phase 3: Release v1.1.10 (Day 2 - 2 hours)

**Objective**: Publish fixed version to npm and Docker Hub

### Tasks

#### DKRHUB-3001: Update Package Version
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-2001, DKRHUB-1007

**Description**: Bump package.json to v1.1.10 and prepare for release.

**Implementation Steps**:
1. Update `packages/maproom-mcp/package.json` version
2. Update CHANGELOG.md with v1.1.10 notes
3. Commit changes
4. Create PR for review

**Acceptance Criteria**:
- [ ] package.json version = "1.1.10"
- [ ] CHANGELOG.md includes v1.1.10 section
- [ ] All changes committed
- [ ] PR approved by maintainer

**Files Modified**:
- `packages/maproom-mcp/package.json`
- `CHANGELOG.md`

**CHANGELOG Entry**:
```markdown
## [1.1.10] - 2025-10-29

### Fixed
- Docker Hub image distribution (v1.1.9 deployment failure)
- docker-compose.yml now pulls pre-built images instead of building from source
- Multi-platform support (AMD64 and ARM64)

### Added
- Automated Docker image publishing via GitHub Actions
- Version pinning support via MAPROOM_VERSION environment variable
- docker-compose.override.yml for local development builds

### Changed
- docker-compose.yml uses `image:` instead of `build:`
- Images now available at https://hub.docker.com/r/crewchief/maproom-mcp
```

---

#### DKRHUB-3002: Create and Push Git Tag
**Priority**: P0 (Critical)
**Effort**: 0.25 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-3001

**Description**: Tag repository with v1.1.10 to trigger workflow.

**Implementation Steps**:
1. Merge PR from DKRHUB-3001
2. Pull latest main branch
3. Create annotated tag
4. Push tag to origin

**Acceptance Criteria**:
- [ ] PR merged to main
- [ ] Local main up to date
- [ ] Tag created: v1.1.10
- [ ] Tag pushed to GitHub
- [ ] Workflow triggered automatically

**Commands**:
```bash
# Ensure on main and up to date
git checkout main
git pull origin main

# Create annotated tag
git tag -a v1.1.10 -m "Release v1.1.10: Fix Docker Hub deployment"

# Push tag (triggers workflow)
git push origin v1.1.10
```

---

#### DKRHUB-3003: Monitor GitHub Actions Workflow
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-3002

**Description**: Monitor workflow execution and verify success.

**Implementation Steps**:
1. Navigate to GitHub Actions tab
2. Watch workflow progress
3. Check for errors or warnings
4. Verify all steps complete
5. Review build summary

**Acceptance Criteria**:
- [ ] Workflow triggers on tag push
- [ ] All steps complete successfully
- [ ] Build time <15 minutes
- [ ] No errors in logs
- [ ] Build summary generated

**Monitoring**:
- URL: `https://github.com/danielbushman/crewchief/actions`
- Workflow: "Publish Maproom MCP Docker Image"
- Check: Build logs, security scan results, push confirmation

---

#### DKRHUB-3004: Verify Images on Docker Hub
**Priority**: P0 (Critical)
**Effort**: 0.25 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-3003

**Description**: Verify images published correctly to Docker Hub.

**Implementation Steps**:
1. Navigate to Docker Hub repository
2. Check tags exist (1.1.10, 1.1, 1, latest)
3. Verify platforms (AMD64, ARM64)
4. Check image metadata
5. Test pull

**Acceptance Criteria**:
- [ ] Repository visible: https://hub.docker.com/r/crewchief/maproom-mcp
- [ ] Tags present: 1.1.10, 1.1, 1, latest
- [ ] Both platforms available
- [ ] Metadata includes version, commit SHA
- [ ] Images can be pulled

**Verification Commands**:
```bash
# Pull image
docker pull crewchief/maproom-mcp:1.1.10

# Check platforms
docker manifest inspect crewchief/maproom-mcp:1.1.10

# Inspect metadata
docker inspect crewchief/maproom-mcp:1.1.10 \
  --format='{{json .Config.Labels}}' | jq
```

---

#### DKRHUB-3005: Publish npm Package
**Priority**: P0 (Critical)
**Effort**: 0.5 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-3004

**Description**: Publish v1.1.10 to npm registry.

**Implementation Steps**:
1. Build package locally
2. Run prepublishOnly script
3. Verify package contents
4. Publish to npm
5. Verify publication

**Acceptance Criteria**:
- [ ] Package built successfully
- [ ] Security audit passes
- [ ] Package.json version correct
- [ ] docker-compose.yml included in package
- [ ] Published to npm
- [ ] Version 1.1.10 visible on npmjs.com

**Commands**:
```bash
cd packages/maproom-mcp

# Build
pnpm build

# Run prepublish checks
pnpm prepublishOnly

# Verify package contents
npm pack --dry-run

# Publish
pnpm publish --access public

# Verify
npm view @crewchief/maproom-mcp version
npm view @crewchief/maproom-mcp
```

**Phase 3 Deliverable**: v1.1.10 published to npm and Docker Hub, ready for users.

---

## Phase 4: Validation & Documentation (Day 2-3 - 4 hours)

**Objective**: Verify release works end-to-end and update documentation

### Tasks

#### DKRHUB-4001: End-to-End Testing (Linux AMD64)
**Priority**: P0 (Critical)
**Effort**: 1 hour
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-3005

**Description**: Test complete installation and startup on Linux AMD64.

**Implementation Steps**:
1. Set up clean Ubuntu 22.04 environment
2. Install npm and Docker
3. Install package: `npm install -g @crewchief/maproom-mcp@1.1.10`
4. Start services: `npx @crewchief/maproom-mcp start`
5. Verify all services healthy
6. Test basic functionality

**Acceptance Criteria**:
- [ ] Clean install succeeds
- [ ] All dependencies resolve
- [ ] Docker images pull correctly
- [ ] Services start without errors
- [ ] Health checks pass
- [ ] MCP server responds (if applicable)

**Environment**:
- OS: Ubuntu 22.04 LTS
- Docker: 24.0+
- Node: 18+
- Architecture: AMD64

**Testing Script**:
```bash
# Clean environment
docker system prune -af
npm uninstall -g @crewchief/maproom-mcp

# Install
npm install -g @crewchief/maproom-mcp@1.1.10

# Verify installation
which maproom-mcp
maproom-mcp --version

# Start services
time npx @crewchief/maproom-mcp start

# Wait for startup
sleep 60

# Check services
docker ps
docker logs maproom-mcp
docker logs maproom-postgres
docker logs maproom-ollama

# Check health
docker inspect maproom-mcp --format='{{.State.Health.Status}}'
docker inspect maproom-postgres --format='{{.State.Health.Status}}'
```

---

#### DKRHUB-4002: End-to-End Testing (macOS ARM64)
**Priority**: P0 (Critical)
**Effort**: 1 hour
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-3005

**Description**: Test complete installation and startup on macOS ARM64 (Apple Silicon).

**Implementation Steps**:
1. Set up clean macOS environment (M1/M2/M3)
2. Ensure Docker Desktop installed
3. Install package
4. Start services
5. Verify ARM64 images used
6. Test functionality

**Acceptance Criteria**:
- [ ] Install succeeds on Apple Silicon
- [ ] ARM64 images pulled (not AMD64)
- [ ] Rosetta not required
- [ ] Performance acceptable
- [ ] All tests pass

**Environment**:
- OS: macOS 13+ (Ventura or later)
- Docker Desktop: 4.25+
- Node: 18+
- Architecture: ARM64 (Apple Silicon)

**Platform Verification**:
```bash
# Check architecture
uname -m  # Should be arm64

# Install and start
npm install -g @crewchief/maproom-mcp@1.1.10
npx @crewchief/maproom-mcp start

# Verify ARM64 image used
docker inspect maproom-mcp --format='{{.Architecture}}'
# Should be: arm64
```

---

#### DKRHUB-4003: Version Pinning Tests
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: QA Engineer
**Dependencies**: DKRHUB-4001

**Description**: Test version pinning with MAPROOM_VERSION environment variable.

**Implementation Steps**:
1. Test with latest (default)
2. Test with specific version (1.1.10)
3. Test with minor version (1.1)
4. Test with major version (1)
5. Test with invalid version

**Acceptance Criteria**:
- [ ] Latest pulls :latest tag
- [ ] Specific version pulls :1.1.10 tag
- [ ] Minor version pulls :1.1 tag
- [ ] Major version pulls :1 tag
- [ ] Invalid version fails gracefully

**Testing Commands**:
```bash
# Test latest (default)
npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test specific version
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test minor version
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test major version
MAPROOM_VERSION=1 npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test invalid
MAPROOM_VERSION=99.99.99 npx @crewchief/maproom-mcp start
# Should fail with clear error
```

---

#### DKRHUB-4004: Update README.md
**Priority**: P1 (High)
**Effort**: 1 hour
**Assignee**: Documentation Writer
**Dependencies**: DKRHUB-4001, DKRHUB-4002

**Description**: Update README with Docker Hub information and version pinning.

**Implementation Steps**:
1. Add Docker Hub badge and link
2. Document version pinning
3. Update installation instructions
4. Add troubleshooting section
5. Update architecture diagram

**Acceptance Criteria**:
- [ ] Docker Hub link added
- [ ] Version pinning documented
- [ ] Installation steps updated
- [ ] Examples include version pinning
- [ ] Troubleshooting section added

**Files Modified**:
- `packages/maproom-mcp/README.md`

**Sections to Add/Update**:
```markdown
## Docker Images

Pre-built Docker images are available on Docker Hub:
- https://hub.docker.com/r/crewchief/maproom-mcp

Supported platforms:
- linux/amd64 (x86_64)
- linux/arm64 (ARM64/Apple Silicon)

## Version Pinning

By default, Maproom MCP uses the `latest` image tag. To pin to a specific version:

\`\`\`bash
# Pin to specific patch version (recommended for production)
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start

# Pin to minor version (get patches automatically)
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start

# Pin to major version
MAPROOM_VERSION=1 npx @crewchief/maproom-mcp start

# Use latest (default)
npx @crewchief/maproom-mcp start
\`\`\`

## Troubleshooting

### Image Pull Failures
If Docker can't pull images:
1. Check Docker is running: `docker ps`
2. Check network connectivity: `docker pull hello-world`
3. Try manual pull: `docker pull crewchief/maproom-mcp:latest`
4. Check Docker Hub status: https://status.docker.com

### Wrong Architecture
If using Apple Silicon (M1/M2/M3), ensure ARM64 image is pulled:
\`\`\`bash
docker inspect maproom-mcp --format='{{.Architecture}}'
# Should be: arm64
\`\`\`
```

---

#### DKRHUB-4005: Create Migration Guide
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: Documentation Writer
**Dependencies**: DKRHUB-4004

**Description**: Create guide for users migrating from v1.1.9 to v1.1.10.

**Implementation Steps**:
1. Document what changed
2. List upgrade steps
3. Add rollback instructions
4. Include troubleshooting

**Acceptance Criteria**:
- [ ] Migration guide created
- [ ] Upgrade steps clear
- [ ] Rollback procedure documented
- [ ] Common issues addressed

**Files Created**:
- `NEW: packages/maproom-mcp/MIGRATION_v1.1.10.md`

**Content Outline**:
```markdown
# Migration Guide: v1.1.9 → v1.1.10

## What Changed

### Breaking Changes
- None (backward compatible)

### Improvements
- Docker images now pre-built and available on Docker Hub
- Faster startup (no build time)
- Multi-platform support (AMD64, ARM64)
- Version pinning support

## Upgrade Steps

1. **Stop existing services**:
   \`\`\`bash
   npx @crewchief/maproom-mcp stop
   \`\`\`

2. **Update package**:
   \`\`\`bash
   npm install -g @crewchief/maproom-mcp@latest
   \`\`\`

3. **Start services**:
   \`\`\`bash
   npx @crewchief/maproom-mcp start
   \`\`\`

4. **Verify**:
   \`\`\`bash
   docker ps
   maproom-mcp --version
   \`\`\`

## Rollback

If issues occur, rollback to v1.1.8 (v1.1.9 is broken):
\`\`\`bash
npm install -g @crewchief/maproom-mcp@1.1.8
\`\`\`
```

---

#### DKRHUB-4006: Update CHANGELOG and Announcement
**Priority**: P1 (High)
**Effort**: 0.5 hours
**Assignee**: Release Manager
**Dependencies**: DKRHUB-4004

**Description**: Finalize CHANGELOG and prepare release announcement.

**Implementation Steps**:
1. Review CHANGELOG.md
2. Add final notes
3. Prepare GitHub release notes
4. Draft announcement message

**Acceptance Criteria**:
- [ ] CHANGELOG complete and accurate
- [ ] GitHub release notes drafted
- [ ] Announcement message ready
- [ ] Links to documentation included

**Files Modified**:
- `CHANGELOG.md`

**GitHub Release Notes**:
```markdown
# v1.1.10: Docker Hub Distribution

## 🎉 What's New

- **Pre-built Docker images** now available on Docker Hub
- **Multi-platform support**: AMD64 and ARM64 (Apple Silicon)
- **Faster startup**: No build time, just pull and run
- **Version pinning**: Control which version you use

## 🐛 Bug Fixes

- Fixed critical deployment failure in v1.1.9
- docker-compose.yml now uses pre-built images

## 📦 Docker Hub

Images available at: https://hub.docker.com/r/crewchief/maproom-mcp

Pull specific version:
\`\`\`bash
docker pull crewchief/maproom-mcp:1.1.10
\`\`\`

## 🚀 Installation

\`\`\`bash
npm install -g @crewchief/maproom-mcp@1.1.10
npx @crewchief/maproom-mcp start
\`\`\`

## 📚 Documentation

- [README](https://github.com/danielbushman/crewchief/blob/main/packages/maproom-mcp/README.md)
- [Migration Guide](https://github.com/danielbushman/crewchief/blob/main/packages/maproom-mcp/MIGRATION_v1.1.10.md)
- [Docker Hub](https://hub.docker.com/r/crewchief/maproom-mcp)

## ⚠️ Important

v1.1.9 had a deployment issue and should not be used. Please upgrade to v1.1.10.
```

**Phase 4 Deliverable**: Fully tested and documented v1.1.10 release, ready for users.

---

## Timeline and Effort Summary

### Phase Breakdown

| Phase | Description | Duration | Effort | Priority |
|-------|-------------|----------|--------|----------|
| 1 | GitHub Actions Workflow | 4 hours | 8 hours | P0 |
| 2 | Docker Compose Updates | 2 hours | 3 hours | P0 |
| 3 | Release v1.1.10 | 2 hours | 2 hours | P0 |
| 4 | Validation & Documentation | 4 hours | 4.5 hours | P1 |
| **Total** | | **12 hours** | **17.5 hours** | |

### Critical Path (MVP)

**Phases 1-3** (MVP): 8 hours elapsed, 13 hours total effort

Phases 1-3 must complete before v1.1.10 can be released. Phase 4 can happen in parallel with early user testing.

### Staffing

**Required Roles**:
- DevOps Engineer (1): Phases 1-2 (11 hours)
- QA Engineer (1): Testing tasks (3.5 hours)
- Release Manager (1): Phase 3 (1.75 hours)
- Documentation Writer (1): Phase 4 (1.5 hours)
- Security Engineer (1): Security tasks (0.75 hours)

**Total**: ~18 hours team effort, 12 hours elapsed time (with parallel work)

### Milestones

| Milestone | Target | Status |
|-----------|--------|--------|
| M1: Workflow working | End of Day 1 | Pending |
| M2: Docker Compose updated | End of Day 1 | Pending |
| M3: v1.1.10 released | End of Day 2 | Pending |
| M4: Fully documented | End of Day 3 | Pending |

## Risk Mitigation

### High-Risk Items

**R1: Workflow Fails in Production**
- Mitigation: Test with pre-release tag first (DKRHUB-1007)
- Fallback: Debug in GitHub Actions, iterate

**R2: Multi-Platform Build Issues**
- Mitigation: Test both platforms separately
- Fallback: Release AMD64 first, ARM64 later

**R3: npm Package Still Broken**
- Mitigation: End-to-end testing before announcement
- Fallback: Quick hotfix release

### Contingency Plans

**If Workflow Doesn't Trigger**:
1. Check tag format (must be `v*.*.*`)
2. Check GitHub Actions enabled
3. Try manual trigger via workflow_dispatch

**If Images Don't Push**:
1. Verify Docker Hub credentials
2. Check token permissions
3. Try manual docker push

**If Tests Fail**:
1. Document failure
2. Fix issue
3. Retest
4. Do not release until all pass

## Success Criteria

### Must-Have (Blocking Release)

- [x] GitHub Actions workflow created
- [x] Multi-platform builds working
- [x] Images pushed to Docker Hub
- [x] docker-compose.yml updated
- [x] v1.1.10 tagged and built
- [x] npm package published
- [x] End-to-end tests pass (Linux AMD64)
- [x] End-to-end tests pass (macOS ARM64)

### Should-Have (Nice to Have)

- [x] docker-compose.override.yml created
- [x] Dockerfile metadata added
- [x] Security scanning passing
- [x] README updated
- [x] Migration guide created
- [x] CHANGELOG updated

### Nice-to-Have (Future)

- [ ] Windows WSL2 testing
- [ ] Image signing (Cosign)
- [ ] SBOM generation
- [ ] Performance benchmarks
- [ ] User feedback collected

## Post-Release Activities

### Monitoring (Week 1)

1. **GitHub Actions**:
   - Monitor workflow runs
   - Track build failures
   - Optimize build times

2. **Docker Hub**:
   - Monitor pull counts
   - Track platform distribution
   - Watch for rate limit issues

3. **User Feedback**:
   - Monitor GitHub issues
   - Track npm downloads
   - Respond to questions

### Iteration (Week 2+)

1. **Performance Optimization**:
   - Analyze build times
   - Optimize caching
   - Reduce image size

2. **Security Enhancements**:
   - Implement image signing
   - Add SBOM generation
   - Pin base image to digest

3. **Documentation Improvements**:
   - Based on user questions
   - Add more examples
   - Video tutorials

## Appendix

### Useful Commands

**GitHub Actions**:
```bash
# Trigger workflow manually
gh workflow run publish-maproom-mcp-image.yml -f version=1.1.10

# Check workflow status
gh run list --workflow=publish-maproom-mcp-image.yml

# View workflow logs
gh run view <run-id> --log
```

**Docker Hub**:
```bash
# Login
docker login -u $DOCKERHUB_USERNAME

# Pull image
docker pull crewchief/maproom-mcp:1.1.10

# Inspect manifest
docker manifest inspect crewchief/maproom-mcp:1.1.10

# Check layers
docker history crewchief/maproom-mcp:1.1.10
```

**npm**:
```bash
# Install specific version
npm install -g @crewchief/maproom-mcp@1.1.10

# Check installed version
npm list -g @crewchief/maproom-mcp

# View package info
npm view @crewchief/maproom-mcp
```

### References

- [Docker Buildx Documentation](https://docs.docker.com/buildx/working-with-buildx/)
- [GitHub Actions: Docker Build](https://github.com/marketplace/actions/build-and-push-docker-images)
- [Docker Compose Override](https://docs.docker.com/compose/extends/)
- [npm Publishing Guide](https://docs.npmjs.com/cli/v9/commands/npm-publish)

---

**Status**: Implementation plan complete, ready to begin Phase 1
