# DKRHUB: Docker Hub Publishing for Maproom MCP

**Project Slug**: DKRHUB
**Status**: Ready for Implementation
**Created**: 2025-10-29
**Priority**: Critical (P0)

## Overview

This project implements automated Docker Hub publishing for Maproom MCP to fix the broken v1.1.9 deployment and achieve a production-ready, fully deployable solution.

## Problem Statement

**v1.1.9 is broken**: Users cannot start the service because docker-compose.yml tries to build from a source context (`../../..`) that doesn't exist when installed via npm. The package fails with "lstat /packages: no such file or directory".

**Impact**: All users who installed v1.1.9 are blocked. The published npm package is unusable.

## Solution

Implement Docker Hub publishing with multi-platform images (AMD64, ARM64) via GitHub Actions, and update docker-compose.yml to pull pre-built images instead of building from source.

## Project Goals

1. Fix immediate deployment failure (critical)
2. Publish pre-built Docker images to Docker Hub
3. Support multi-platform (AMD64, ARM64 for Apple Silicon)
4. Automate via GitHub Actions on version tags
5. Maintain backward compatibility with development workflow
6. Ensure production-ready quality and security

## Documents

### 1. [DKRHUB_ANALYSIS.md](./DKRHUB_ANALYSIS.md)
**Purpose**: Deep analysis of the Docker distribution problem

**Key Contents**:
- Current broken state analysis (v1.1.9 deployment failure)
- Why Phase 1 (build from source) fails in production
- Target state: Docker Hub pre-built images
- npm package distribution constraints
- Multi-platform considerations
- Docker Hub vs alternatives comparison
- Risk analysis and success criteria

**Key Finding**: The docker-compose.yml build context assumption (`../../..`) breaks outside the development monorepo. Pre-built images from Docker Hub are the only viable solution for npm distribution.

### 2. [DKRHUB_ARCHITECTURE.md](./DKRHUB_ARCHITECTURE.md)
**Purpose**: Technical design and implementation details

**Key Contents**:
- GitHub Actions workflow specification
- Multi-platform build configuration (Buildx, QEMU)
- Docker image tagging strategy (full, minor, major, latest)
- Updated docker-compose.yml (using `image:` not `build:`)
- Development override pattern (docker-compose.override.yml)
- Dockerfile metadata labels
- Version management strategy
- Build performance optimization

**Key Decisions**:
- Use Docker Buildx for multi-platform builds
- Tag strategy: 1.1.10, 1.1, 1, latest
- Hybrid approach: production uses images, development can build locally
- GitHub Actions cache for faster builds

### 3. [DKRHUB_QUALITY_STRATEGY.md](./DKRHUB_QUALITY_STRATEGY.md)
**Purpose**: Testing strategy and validation approach

**Key Contents**:
- Testing pyramid (unit, integration, E2E)
- Image validation tests (build, metadata, size, security)
- Container integration tests (startup, communication, resources)
- End-to-end testing on multiple platforms
- Multi-platform validation matrix
- Automated security scanning (Trivy)
- Performance benchmarks
- Quality gates (must-pass, should-pass, nice-to-have)

**Key Requirements**:
- Test on Linux AMD64 and macOS ARM64 (must)
- Zero critical vulnerabilities (must)
- Image size <500MB (must)
- Startup time <30s warm (must)

### 4. [DKRHUB_SECURITY_REVIEW.md](./DKRHUB_SECURITY_REVIEW.md)
**Purpose**: Security analysis and threat mitigation

**Key Contents**:
- Threat model (attack vectors, threat actors)
- Credential management (Docker Hub tokens, GitHub Secrets)
- Supply chain security (base images, dependencies, SBOM)
- Image security (container hardening, vulnerability scanning)
- Access control (GitHub branch protection, release authorization)
- Runtime security (isolation, secrets management)
- Incident response plan
- Security roadmap

**Risk Level**: Medium (acceptable with mitigations)

**Key Controls**:
- Docker Hub access tokens (not passwords)
- 2FA on all accounts
- Non-root user in container
- Trivy security scanning on every build
- GitHub Secrets for credentials
- Multi-stage builds (minimal attack surface)

### 5. [DKRHUB_PLAN.md](./DKRHUB_PLAN.md)
**Purpose**: Phased implementation roadmap

**Key Contents**:
- 4-phase implementation plan
- 23 discrete tasks with effort estimates
- Critical path identification
- Timeline: 2-3 days for MVP, 1 week complete
- Task dependencies and prerequisites
- Acceptance criteria for each task
- Risk mitigation strategies
- Success criteria and deliverables

**Phases**:
1. **Phase 1**: GitHub Actions Workflow (4 hours) - Critical path
2. **Phase 2**: Docker Compose Updates (2 hours)
3. **Phase 3**: Release v1.1.10 (2 hours)
4. **Phase 4**: Validation & Documentation (4 hours)

## Quick Reference

### Solution Architecture

```
┌────────────────────────────────────────────────────────────┐
│              GitHub Actions Workflow                       │
│  (Triggered on tag push: v1.1.10)                          │
│                                                            │
│  1. Checkout code                                          │
│  2. Set up Buildx (multi-platform)                         │
│  3. Login to Docker Hub                                    │
│  4. Build AMD64 + ARM64 images                             │
│  5. Tag: 1.1.10, 1.1, 1, latest                            │
│  6. Push to Docker Hub                                     │
│  7. Security scan (Trivy)                                  │
└──────────────────┬─────────────────────────────────────────┘
                   │
                   ▼
          ┌────────────────┐
          │  Docker Hub    │
          │  crewchief/    │
          │  maproom-mcp   │
          │  (multi-arch)  │
          └────────┬───────┘
                   │
                   ▼
        ┌─────────────────────────┐
        │   User's System         │
        │                         │
        │   $ npm install -g      │
        │     @crewchief/         │
        │     maproom-mcp@1.1.10  │
        │                         │
        │   $ npx @crewchief/     │
        │     maproom-mcp start   │
        │                         │
        │   Docker pulls image    │
        │   from Docker Hub       │
        │   (no build needed!)    │
        └─────────────────────────┘
```

### Key Changes

**Before (v1.1.9 - BROKEN)**:
```yaml
# docker-compose.yml
maproom-mcp:
  build:
    context: ../../..  # Fails in npm deployment!
    dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
```

**After (v1.1.10 - FIXED)**:
```yaml
# docker-compose.yml
maproom-mcp:
  image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
  # No build section - pulls from Docker Hub
```

### Technology Stack

**CI/CD**:
- GitHub Actions (workflow automation)
- Docker Buildx (multi-platform builds)
- QEMU (ARM64 emulation on AMD64)

**Container Registry**:
- Docker Hub (public image hosting)
- Multi-arch manifest (AMD64 + ARM64)

**Security**:
- Trivy (vulnerability scanning)
- GitHub Secrets (credential management)
- Docker Content Trust (future: image signing)

**Testing**:
- GitHub Actions (automated testing)
- Linux AMD64 + macOS ARM64 (manual testing)

## Prerequisites

**Already Complete**:
- Docker Hub account created (crewchief)
- GitHub Secrets configured (DOCKERHUB_USERNAME, DOCKERHUB_TOKEN)
- Dockerfile.mcp-server exists (production-ready)
- npm package structure validated

**Required for Implementation**:
- GitHub repository write access
- Ability to create tags and workflows
- Test environments (Linux AMD64, macOS ARM64)

## Implementation Checklist

### Phase 1: GitHub Actions Workflow
- [ ] DKRHUB-1001: Create workflow file
- [ ] DKRHUB-1002: Configure multi-platform build
- [ ] DKRHUB-1003: Docker Hub authentication
- [ ] DKRHUB-1004: Version extraction and tagging
- [ ] DKRHUB-1005: Image build and push
- [ ] DKRHUB-1006: Security scanning (Trivy)
- [ ] DKRHUB-1007: Test with pre-release tag

### Phase 2: Docker Compose Updates
- [ ] DKRHUB-2001: Update docker-compose.yml (use image:)
- [ ] DKRHUB-2002: Create docker-compose.override.yml
- [ ] DKRHUB-2003: Add Dockerfile metadata labels
- [ ] DKRHUB-2004: Test production config
- [ ] DKRHUB-2005: Test development config

### Phase 3: Release v1.1.10
- [ ] DKRHUB-3001: Update package.json version
- [ ] DKRHUB-3002: Create and push git tag
- [ ] DKRHUB-3003: Monitor GitHub Actions workflow
- [ ] DKRHUB-3004: Verify images on Docker Hub
- [ ] DKRHUB-3005: Publish npm package

### Phase 4: Validation & Documentation
- [ ] DKRHUB-4001: E2E testing (Linux AMD64)
- [ ] DKRHUB-4002: E2E testing (macOS ARM64)
- [ ] DKRHUB-4003: Version pinning tests
- [ ] DKRHUB-4004: Update README.md
- [ ] DKRHUB-4005: Create migration guide
- [ ] DKRHUB-4006: Update CHANGELOG and announcement

## Success Metrics

### Must-Have (Blocking Release)
- GitHub Actions workflow builds images successfully
- Multi-platform images published to Docker Hub (AMD64, ARM64)
- docker-compose.yml uses pre-built images
- v1.1.10 npm package published
- End-to-end tests pass on Linux AMD64
- End-to-end tests pass on macOS ARM64
- Zero critical security vulnerabilities

### Should-Have (High Priority)
- Image size <500MB
- Build time <15 minutes
- Startup time <30 seconds (warm)
- Security scanning integrated
- Documentation updated
- Migration guide created

### Nice-to-Have (Future)
- Image signing (Cosign)
- SBOM generation
- Windows WSL2 testing
- Performance benchmarks

## Timeline

**Critical Path (MVP)**: 8-10 hours elapsed time

| Phase | Duration | Status |
|-------|----------|--------|
| Phase 1: Workflow | 4 hours | Pending |
| Phase 2: Compose | 2 hours | Pending |
| Phase 3: Release | 2 hours | Pending |
| Phase 4: Validation | 4 hours | Pending |

**Target**: v1.1.10 released within 2-3 days

## Risk Management

### Critical Risks

**R1: v1.1.9 Remains Broken**
- Impact: Users blocked
- Mitigation: Prioritize v1.1.10 release
- Timeline: Fix within 2-3 days

**R2: Multi-Platform Build Failures**
- Impact: Apple Silicon users can't use images
- Mitigation: Test with pre-release tag first
- Fallback: Release AMD64 first, ARM64 later

**R3: Security Vulnerabilities Found**
- Impact: Cannot release
- Mitigation: Trivy scanning, base image updates
- Timeline: Fix before release

### Medium Risks

**R4: Docker Hub Rate Limits**
- Impact: Users can't pull images
- Mitigation: Authenticated pulls, monitoring
- Likelihood: Low

**R5: Large Image Size**
- Impact: Slow downloads
- Mitigation: Multi-stage builds (already implemented)
- Likelihood: Low

## Dependencies

### External Dependencies
- Docker Hub availability
- GitHub Actions availability
- npm registry availability
- Base images (node:20-alpine, pgvector, ollama)

### Internal Dependencies
- GitHub repository access
- Docker Hub credentials
- Test environments

### Blocked By
- None (all prerequisites met)

### Blocks
- v1.1.10 release (this project blocks the release)
- User adoption (users blocked until fixed)

## Next Steps

1. **Review all project documents**
   - Read DKRHUB_ANALYSIS.md for problem context
   - Study DKRHUB_ARCHITECTURE.md for technical design
   - Review DKRHUB_SECURITY_REVIEW.md for security requirements
   - Review DKRHUB_QUALITY_STRATEGY.md for testing approach
   - Study DKRHUB_PLAN.md for implementation tasks

2. **Set up environment**
   - Verify GitHub access and permissions
   - Confirm Docker Hub credentials work
   - Prepare test environments (Linux, macOS)

3. **Begin Phase 1**
   - Assign DevOps Engineer to workflow tasks
   - Create `.github/workflows/publish-maproom-mcp-image.yml`
   - Test workflow with pre-release tag (v1.1.10-rc1)

4. **Track progress**
   - Use task IDs from DKRHUB_PLAN.md
   - Update checklist in this README
   - Report status daily

## Support and Contact

**Project Lead**: TBD
**DevOps Lead**: TBD
**QA Lead**: TBD

**Communication**:
- GitHub Issues for bugs and questions
- Pull Requests for code review
- Project board for task tracking

## Resources

### Docker Hub
- Repository: https://hub.docker.com/r/crewchief/maproom-mcp (will be public after first push)
- Documentation: https://docs.docker.com/docker-hub/

### GitHub
- Repository: https://github.com/danielbushman/crewchief
- Actions: https://github.com/danielbushman/crewchief/actions
- Secrets: https://github.com/danielbushman/crewchief/settings/secrets/actions

### Documentation
- Docker Buildx: https://docs.docker.com/buildx/
- GitHub Actions: https://docs.github.com/en/actions
- Multi-platform images: https://docs.docker.com/build/building/multi-platform/

## Files Created

This project creates/modifies:

1. **Project Documentation**:
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/README.md` (this file)
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_ANALYSIS.md`
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_ARCHITECTURE.md`
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_QUALITY_STRATEGY.md`
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_SECURITY_REVIEW.md`
   - `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_PLAN.md`

2. **Implementation Files** (to be created):
   - `.github/workflows/publish-maproom-mcp-image.yml` (NEW)
   - `packages/maproom-mcp/config/docker-compose.yml` (MODIFIED)
   - `packages/maproom-mcp/config/docker-compose.override.yml` (NEW)
   - `packages/maproom-mcp/config/Dockerfile.mcp-server` (MODIFIED - add labels)
   - `packages/maproom-mcp/package.json` (MODIFIED - version bump)
   - `packages/maproom-mcp/README.md` (MODIFIED - add Docker Hub info)
   - `packages/maproom-mcp/MIGRATION_v1.1.10.md` (NEW)
   - `CHANGELOG.md` (MODIFIED - add v1.1.10 entry)

3. **Related Existing Tickets** (reference, don't duplicate):
   - `.agents/work-tickets/LOCAL-4006_optimize-docker-image-size.md` (COMPLETED - image already optimized)
   - `.agents/work-tickets/LOCAL-3008_npm-publish-test-release.md` (INCOMPLETE - relates to npm publishing)
   - `.agents/work-tickets/LOCAL-4005_arm64-apple-silicon-testing.md` (relates to multi-platform support)
   - `.agents/work-tickets/MCPSTART-6004_publish-npm-v1-1-9.md` (INCOMPLETE - blocked by this project)

## FAQs

**Q: Why Docker Hub and not GitHub Container Registry?**
A: Docker Hub is the standard for open-source Docker images. It has better discoverability, no authentication required for public pulls, and is more familiar to users.

**Q: Why multi-platform builds?**
A: Apple Silicon Macs (M1/M2/M3) require ARM64 images. Without multi-platform support, these users would experience slow emulation or incompatibility.

**Q: What happens to v1.1.9?**
A: It remains published but broken. We'll add a deprecation warning and encourage users to upgrade to v1.1.10.

**Q: Can developers still build locally?**
A: Yes! The docker-compose.override.yml pattern preserves the local build workflow for development.

**Q: How long will builds take?**
A: First build: ~15 minutes. Subsequent builds: ~5 minutes (with caching).

**Q: What's the image size?**
A: Target: ~300MB uncompressed, ~120MB compressed download (thanks to alpine base).

**Q: Is this secure?**
A: Yes. We use access tokens (not passwords), scan for vulnerabilities with Trivy, run containers as non-root, and follow Docker security best practices.

---

**Ready for Implementation**: All planning documents are complete. Begin Phase 1 to start implementation.
