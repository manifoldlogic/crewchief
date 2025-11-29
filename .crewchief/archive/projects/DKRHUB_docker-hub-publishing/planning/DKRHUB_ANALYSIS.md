# DKRHUB: Docker Hub Publishing - Problem Analysis

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Problem Analysis

## Executive Summary

The Maproom MCP v1.1.9 release is broken in production. While the npm package was successfully published, users cannot start the service because the docker-compose.yml attempts to build the `maproom-mcp` container from a source context (`../../..`) that only exists in the development workspace. When deployed to `~/.maproom-mcp/` via npm install, the build fails with "lstat /packages: no such file or directory".

**Root Cause**: The current architecture is stuck in Phase 1 (build from source) when it needs to be in Phase 2 (pre-built images from registry).

**Solution**: Implement automated Docker image publishing to Docker Hub via GitHub Actions, and update docker-compose.yml to use pre-built images instead of local builds.

**Impact**: Without this fix, v1.1.9 is unusable for all users. The published npm package cannot be started.

## Problem Space Analysis

### Current State: Broken Production Deployment

#### What Exists Today

1. **npm Package Structure** (`@crewchief/maproom-mcp@1.1.9`):
   ```
   ~/.maproom-mcp/
   ├── package.json (version 1.1.9)
   ├── bin/cli.cjs (executable wrapper)
   ├── config/
   │   ├── docker-compose.yml (BROKEN - tries to build from source)
   │   ├── Dockerfile.mcp-server (build instructions)
   │   └── init.sql (database schema)
   ├── dist/ (compiled TypeScript)
   └── src/ (TypeScript source)
   ```

2. **docker-compose.yml Configuration** (lines 87-91):
   ```yaml
   maproom-mcp:
     build:
       context: ../../..  # PROBLEM: This path doesn't exist in deployment!
       dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
   ```

3. **User Installation Flow**:
   ```bash
   # User installs the package
   npm install -g @crewchief/maproom-mcp

   # Package is extracted to ~/.maproom-mcp/

   # User tries to start (via npx or direct command)
   npx @crewchief/maproom-mcp start

   # CLI runs: docker-compose -f config/docker-compose.yml up -d

   # Docker Compose fails:
   # Error: lstat /packages: no such file or directory
   # Cannot build maproom-mcp service
   ```

#### Why It Fails

The build context `../../..` makes these assumptions:
1. The package is installed in a monorepo at `packages/maproom-mcp/`
2. The parent directory structure exists: `workspace/packages/maproom-mcp/config/`
3. The Dockerfile can access other packages via relative paths

None of these are true in production deployment:
- npm installs to `~/.maproom-mcp/` (or `node_modules/@crewchief/maproom-mcp/`)
- There is no `packages/` directory three levels up
- The Dockerfile cannot find its build context

#### Why Phase 1 Was Implemented

From LOCAL_ARCHITECTURE.md (the document that was referenced but never fully implemented):

**Phase 1: Build from Source (MVP)**
- Quick to implement for development/testing
- Avoided Docker Hub account setup
- Allowed iteration without image versioning complexity
- Worked fine in monorepo development environment

**Phase 2: Pre-built Images (Production)** [NOT YET IMPLEMENTED]
- Required for npm package distribution
- Necessary for users without monorepo structure
- Enables faster startup (no build time)
- Better user experience

### Target State: Docker Hub Distribution

#### What Users Should Experience

1. **Installation**:
   ```bash
   npm install -g @crewchief/maproom-mcp
   # Downloads npm package (~50KB source + config)
   ```

2. **First Start**:
   ```bash
   npx @crewchief/maproom-mcp start

   # Docker pulls pre-built images:
   # - crewchief/maproom-mcp:1.1.10 (~200MB)
   # - pgvector/pgvector:pg16 (~150MB, official)
   # - ollama/ollama:latest (~700MB, official)

   # Services start in ~30 seconds
   ```

3. **docker-compose.yml** (what it should be):
   ```yaml
   maproom-mcp:
     image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
     # No build section - just pull from Docker Hub
     container_name: maproom-mcp
     depends_on:
       postgres:
         condition: service_healthy
     environment:
       DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
       # ... other env vars
   ```

#### Benefits of Pre-built Images

**For Users**:
- No build errors - image is guaranteed to work
- Faster startup - no compilation needed
- Smaller npm package - no build dependencies
- Cross-platform - multi-arch images (AMD64, ARM64)
- Offline-capable - image can be cached

**For Maintainers**:
- Controlled releases - images built from CI/CD
- Reproducible builds - same image everywhere
- Version management - clear image tags
- Rollback capability - previous versions available
- Security scanning - automated in CI pipeline

**For Development**:
- Separation of concerns - development uses local builds, production uses images
- Faster testing - pull image instead of building
- Consistent environments - same image across team

### Technical Deep Dive

#### Docker Image Distribution Models

**Model 1: Build from Source** (Current - Broken)
```yaml
services:
  app:
    build:
      context: ../../../  # Relative path dependency
      dockerfile: path/to/Dockerfile
```
- Pros: Simple for development, no registry needed
- Cons: Requires source context, slow, fails in deployment
- Use Case: Development only

**Model 2: Pre-built Registry Images** (Target)
```yaml
services:
  app:
    image: registry/repo:tag  # Pull from registry
```
- Pros: Fast, portable, production-ready
- Cons: Requires registry setup, CI/CD pipeline
- Use Case: Production deployment

**Model 3: Hybrid with Override** (Best Practice)
```yaml
# docker-compose.yml (production)
services:
  app:
    image: registry/repo:${VERSION:-latest}

# docker-compose.override.yml (development)
services:
  app:
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
```
- Pros: Best of both worlds - production uses images, dev can build locally
- Cons: Requires understanding of compose override mechanism
- Use Case: Recommended approach

#### npm Package Distribution Constraints

When distributing via npm, several constraints apply:

1. **Package Size Limits**:
   - npm has soft limit of 10MB, hard limit of 100MB
   - Including Docker images in npm package is not feasible
   - Current package (~50KB source + config) is appropriate

2. **Dependency on External Resources**:
   - Docker images must come from external registry (Docker Hub)
   - PostgreSQL and Ollama images are already external
   - Maproom MCP image should be external too for consistency

3. **Installation Context**:
   - npm can install globally or locally
   - Global: `~/.npm-packages/` or `/usr/local/lib/node_modules/`
   - Local: `<project>/node_modules/@crewchief/maproom-mcp/`
   - Neither context has access to monorepo structure

4. **User Environment Variability**:
   - Users may not have build tools (gcc, make, python)
   - Docker may be Docker Desktop, Docker Engine, or Rancher Desktop
   - Network constraints may prevent downloading build dependencies
   - Pre-built images work everywhere Docker works

#### Multi-Platform Considerations

Modern deployments require multi-platform support:

1. **AMD64 (x86_64)**:
   - Linux servers
   - Intel Macs
   - Windows (WSL2)
   - Most common platform

2. **ARM64 (aarch64)**:
   - Apple Silicon Macs (M1/M2/M3)
   - AWS Graviton instances
   - Raspberry Pi (development/edge)
   - Growing market share

3. **Build Strategy**:
   - Use `docker buildx` with multiple platforms
   - Create manifest list (multi-arch image)
   - Docker automatically pulls correct platform
   - Example: `docker buildx build --platform linux/amd64,linux/arm64 -t crewchief/maproom-mcp:1.1.10 --push .`

#### Docker Hub vs Alternatives

**Docker Hub** (Recommended):
- Free public repositories (unlimited pulls)
- Official registry for Docker ecosystem
- Excellent caching and CDN
- Integrated with Docker CLI
- Well-documented
- GitHub Actions has first-class support

**GitHub Container Registry (ghcr.io)**:
- Free for public repositories
- Tied to GitHub account
- Good GitHub Actions integration
- Less universal than Docker Hub
- Requires GitHub token for pulls (even public)

**Quay.io**:
- RedHat-owned registry
- Good security scanning
- Less common in community
- Overkill for this use case

**AWS ECR / GCR / Azure CR**:
- Cloud-specific registries
- Not suitable for open-source distribution
- Require cloud account for pulls
- Expensive for public distribution

**Decision**: Docker Hub is the clear choice for open-source npm package distribution.

### Risk Analysis

#### High-Risk Issues

**R1: v1.1.9 is Completely Broken**
- Severity: Critical
- Impact: All users who installed v1.1.9 cannot use the package
- Mitigation: Immediate hotfix release (v1.1.10) with Docker Hub images
- Timeline: Must fix within 1-2 days

**R2: Multi-Platform Build Complexity**
- Severity: High
- Impact: ARM64 users (Apple Silicon) may have issues
- Mitigation: Use GitHub Actions with QEMU for cross-platform builds
- Timeline: Include in initial implementation

**R3: Docker Hub Rate Limits**
- Severity: Medium
- Impact: Anonymous pulls limited to 100 per 6 hours per IP
- Mitigation: Authenticated pulls (200 per 6 hours), document login for power users
- Timeline: Monitor usage, add docs in v1.1.10

#### Medium-Risk Issues

**R4: Image Size Growth**
- Severity: Medium
- Impact: Large images slow downloads, increase storage costs
- Current: Dockerfile.mcp-server uses node:20-alpine (good)
- Mitigation: Multi-stage builds (already implemented), regular optimization
- Timeline: Ongoing monitoring

**R5: Breaking Existing Development Workflows**
- Severity: Medium
- Impact: Contributors need to adapt to new docker-compose structure
- Mitigation: Use docker-compose.override.yml for local development
- Timeline: Document in CONTRIBUTING.md, update CI/CD

**R6: GitHub Actions Costs**
- Severity: Low
- Impact: Multi-platform builds consume GitHub Actions minutes
- Mitigation: Build only on version tags (not every commit), use caching
- Timeline: Monitor usage after implementation

#### Low-Risk Issues

**R7: Docker Hub Account Security**
- Severity: Low (with proper setup)
- Impact: Compromised credentials could push malicious images
- Mitigation: Use access tokens (not password), limit permissions, enable 2FA
- Timeline: Already configured (DOCKERHUB_USERNAME, DOCKERHUB_TOKEN in GitHub Secrets)

**R8: Tag Management Complexity**
- Severity: Low
- Impact: Confusion about which tag to use (latest, 1.1.10, 1.1, 1)
- Mitigation: Clear tagging strategy (document in README)
- Timeline: Define in ARCHITECTURE document

### Success Criteria

#### Must-Have (P0) - MVP

1. **Functional Deployment**
   - [ ] Users can install v1.1.10 via npm
   - [ ] `npx @crewchief/maproom-mcp start` works without errors
   - [ ] All three services start (maproom-mcp, postgres, ollama)
   - [ ] MCP server responds to requests

2. **Multi-Platform Support**
   - [ ] Image builds for linux/amd64
   - [ ] Image builds for linux/arm64
   - [ ] Docker automatically selects correct platform

3. **Automated Publishing**
   - [ ] GitHub Actions workflow triggers on version tags (v*)
   - [ ] Workflow builds and pushes to Docker Hub
   - [ ] Images are publicly accessible

#### Should-Have (P1) - Production Quality

4. **Performance**
   - [ ] Image size < 500MB (Dockerfile.mcp-server uses alpine - should be ~200MB)
   - [ ] Build time < 10 minutes in CI
   - [ ] Startup time < 30 seconds (cold pull + start)

5. **Developer Experience**
   - [ ] Development workflow preserved (local builds work)
   - [ ] Clear documentation for contributors
   - [ ] docker-compose.override.yml pattern documented

6. **Reliability**
   - [ ] Health checks pass after startup
   - [ ] Integration tests pass with published image
   - [ ] No regression in functionality

#### Nice-to-Have (P2) - Polish

7. **Observability**
   - [ ] GitHub Actions build logs are clear
   - [ ] Image metadata includes git commit SHA
   - [ ] Docker Hub description auto-updated

8. **Security**
   - [ ] Automated vulnerability scanning (Trivy in CI)
   - [ ] Base images pinned to specific digests
   - [ ] Non-root user in container (already implemented)

### Constraints and Dependencies

#### Technical Constraints

1. **Docker Hub Account**: Already created and configured
2. **GitHub Secrets**: DOCKERHUB_USERNAME and DOCKERHUB_TOKEN already stored
3. **Dockerfile**: Dockerfile.mcp-server exists and is production-ready
4. **npm Package**: Published to npm as @crewchief/maproom-mcp

#### External Dependencies

1. **Base Images**:
   - `node:20-alpine` (official Node.js image)
   - `pgvector/pgvector:pg16` (official pgvector image)
   - `ollama/ollama:latest` (official Ollama image)

2. **GitHub Actions**:
   - Free tier: 2000 minutes/month for private repos
   - Unlimited for public repos
   - Sufficient for our use case

3. **Docker Hub**:
   - Free tier: Unlimited public repositories
   - Pull rate limits: 100 pulls/6hrs (anonymous), 200 pulls/6hrs (authenticated)
   - Sufficient for moderate usage

#### Timeline Constraints

1. **Urgency**: v1.1.9 is broken - need fix ASAP
2. **Release Window**: Can publish v1.1.10 as soon as workflow is ready
3. **Testing Time**: Must test on both AMD64 and ARM64 before release
4. **Documentation**: Must update README with new deployment instructions

### Comparison with Alternatives

#### Alternative 1: Stay with Build from Source (Status Quo)

**Approach**: Revert to requiring users to clone repository

**Pros**:
- No new infrastructure needed
- No Docker Hub setup

**Cons**:
- Terrible user experience (defeats purpose of npm package)
- Users need git, source code, build tools
- Slower startup (build time)
- Not viable for production use case

**Verdict**: Not acceptable - defeats the purpose of npm distribution

#### Alternative 2: Bundle Image in npm Package

**Approach**: Export Docker image as tar, include in npm package

**Pros**:
- No Docker Hub dependency
- All-in-one package

**Cons**:
- npm package would be ~200MB (vs 50KB)
- Violates npm package size guidelines
- Can't leverage Docker caching
- Multi-platform becomes impossible
- Would still need registry for postgres/ollama

**Verdict**: Not feasible - npm packages should not include Docker images

#### Alternative 3: GitHub Container Registry (ghcr.io)

**Approach**: Use ghcr.io instead of Docker Hub

**Pros**:
- Integrated with GitHub
- Free for public repositories
- Good GitHub Actions support

**Cons**:
- Less universal than Docker Hub
- Requires GitHub token for pulls (even public images)
- Less familiar to users
- Not the standard for open-source distribution

**Verdict**: Possible but Docker Hub is better for public distribution

#### Alternative 4: Hybrid with Optional Build (Recommended + Enhancement)

**Approach**: Default to Docker Hub images, support local builds via override

**Pros**:
- Best user experience (fast startup from registry)
- Preserves developer workflow (local builds)
- Standard Docker Compose pattern
- Allows advanced users to customize

**Cons**:
- Slightly more complex setup (two compose files)
- Need to document override mechanism

**Verdict**: Best approach - combines registry benefits with development flexibility

### Recommended Solution

**Implement Docker Hub publishing with hybrid compose configuration**:

1. **Primary**: docker-compose.yml uses `image: crewchief/maproom-mcp:${VERSION:-latest}`
2. **Development**: docker-compose.override.yml (not published to npm) uses `build:` for local dev
3. **CI/CD**: GitHub Actions builds and pushes multi-platform images on version tags
4. **Testing**: Automated integration tests pull published images and verify functionality
5. **Documentation**: Clear migration guide for v1.1.9 users

This approach:
- Fixes the immediate v1.1.9 deployment failure
- Provides production-ready solution
- Maintains development workflow
- Scales to future needs
- Follows Docker best practices

## Next Steps

1. **Review DKRHUB_ARCHITECTURE.md** for detailed technical design
2. **Review DKRHUB_SECURITY_REVIEW.md** for security considerations
3. **Review DKRHUB_QUALITY_STRATEGY.md** for testing approach
4. **Review DKRHUB_PLAN.md** for implementation roadmap
5. **Begin Phase 1 implementation** (GitHub Actions workflow)

---

**Status**: Analysis complete, ready for architecture design
