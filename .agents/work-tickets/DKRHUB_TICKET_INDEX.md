# DKRHUB: Docker Hub Publishing - Ticket Index

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Ready for Implementation

## Project Overview

This project fixes the critical v1.1.9 deployment failure by implementing automated Docker image publishing to Docker Hub via GitHub Actions. The npm package will pull pre-built images instead of attempting to build from source.

**Problem**: v1.1.9 docker-compose.yml tries to build from `context: ../../..` which doesn't exist in deployed npm packages.

**Solution**: Publish multi-platform images to Docker Hub and update docker-compose.yml to use `image:` instead of `build:`.

**Documentation**:
- Analysis: `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_ANALYSIS.md`
- Architecture: `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_ARCHITECTURE.md`
- Security: `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_SECURITY_REVIEW.md`
- Quality: `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_QUALITY_STRATEGY.md`
- Plan: `crewchief_context/maproom/DKRHUB-Docker_Hub_Publishing/DKRHUB_PLAN.md`

---

## Phase 1: GitHub Actions Workflow (P0 - Critical Path)

**Objective**: Create automated workflow to build and publish Docker images

**Duration**: Day 1 - 4 hours
**Agent**: docker-engineer

### Implementation Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-1001 | Create GitHub Actions Workflow File | 2h | None | ⬜ Not Started |
| DKRHUB-1002 | Configure Multi-Platform Build | 1h | 1001 | ⬜ Not Started |
| DKRHUB-1003 | Implement Docker Hub Authentication | 0.5h | 1002 | ⬜ Not Started |
| DKRHUB-1004 | Implement Version Extraction and Tagging | 1h | 1003 | ⬜ Not Started |
| DKRHUB-1005 | Configure Image Build and Push | 1.5h | 1004 | ⬜ Not Started |
| DKRHUB-1006 | Add Security Scanning with Trivy | 0.5h | 1005 | ⬜ Not Started |

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-1901 | Test Workflow with Pre-Release Tag | 1h | 1001-1006 | ⬜ Not Started |

**Phase 1 Deliverable**: Working GitHub Actions workflow that builds and publishes multi-platform images to Docker Hub

---

## Phase 2: Docker Compose Updates (P0 - Critical Path)

**Objective**: Update docker-compose configuration to use pre-built images

**Duration**: Day 1-2 - 2 hours
**Agent**: docker-engineer

### Implementation Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-2001 | Update docker-compose.yml to Use Images | 0.5h | 1901 | ⬜ Not Started |
| DKRHUB-2002 | Create docker-compose.override.yml for Development | 0.5h | 2001 | ⬜ Not Started |
| DKRHUB-2003 | Add Dockerfile Metadata Labels | 0.5h | 2001 | ⬜ Not Started |

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-2902 | Test Production Configuration (Image Pull) | 0.5h | 2001, 1901 | ⬜ Not Started |
| DKRHUB-2903 | Test Development Configuration (Local Build) | 0.5h | 2002 | ⬜ Not Started |

**Phase 2 Deliverable**: Production-ready docker-compose.yml that pulls images; development override for local builds

---

## Phase 3: Release v1.1.10 (P0 - Critical Path)

**Objective**: Publish fixed version to npm and Docker Hub

**Duration**: Day 2 - 2 hours
**Agent**: docker-engineer

### Release Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-3001 | Update Package Version to v1.1.10 | 0.5h | 2001-2003, 2902-2903 | ⬜ Not Started |
| DKRHUB-3002 | Create and Push Git Tag v1.1.10 | 0.25h | 3001 | ⬜ Not Started |
| DKRHUB-3003 | Monitor GitHub Actions Workflow Execution | 0.5h | 3002 | ⬜ Not Started |
| DKRHUB-3004 | Verify Images on Docker Hub | 0.25h | 3003 | ⬜ Not Started |
| DKRHUB-3005 | Publish npm Package v1.1.10 | 0.5h | 3004 | ⬜ Not Started |

**Phase 3 Deliverable**: v1.1.10 published to npm and Docker Hub, ready for users

---

## Phase 4: Validation & Documentation (P1 - High Priority)

**Objective**: Verify release works end-to-end and update documentation

**Duration**: Day 2-3 - 4 hours
**Agents**: integration-tester, docker-engineer

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-4001 | End-to-End Testing on Linux AMD64 | 1h | 3005 | ⬜ Not Started |
| DKRHUB-4002 | End-to-End Testing on macOS ARM64 | 1h | 3005 | ⬜ Not Started |
| DKRHUB-4003 | Test Version Pinning Functionality | 0.5h | 4001 | ⬜ Not Started |

### Documentation Tickets

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-4004 | Update README with Docker Hub Information | 1h | 4001, 4002 | ⬜ Not Started |
| DKRHUB-4005 | Create Migration Guide v1.1.9 to v1.1.10 | 0.5h | 4004 | ⬜ Not Started |

**Phase 4 Deliverable**: Fully tested and documented v1.1.10 release

---

## Ticket Summary

### By Phase
- **Phase 1**: 7 tickets (6 implementation + 1 test)
- **Phase 2**: 5 tickets (3 implementation + 2 test)
- **Phase 3**: 5 tickets (release process)
- **Phase 4**: 5 tickets (3 test + 2 documentation)
- **Total**: 22 tickets

### By Priority
- **P0 (Critical)**: 17 tickets (Phases 1-3)
- **P1 (High)**: 5 tickets (Phase 4)

### By Effort
- **Total Effort**: ~17.5 hours
- **Elapsed Time**: ~12 hours (with parallel work)
- **Timeline**: 2-3 days

### By Agent
- **docker-engineer**: 14 tickets
- **integration-tester**: 6 tickets
- **verify-ticket**: All tickets (verification step)
- **commit-ticket**: All tickets (commit step)

---

## Critical Path (MVP)

**Must complete for v1.1.10 release**:

1. ✅ **Prerequisites** (Already Complete):
   - Docker Hub account created
   - GitHub Secrets configured (DOCKERHUB_USERNAME, DOCKERHUB_TOKEN)
   - Dockerfile.mcp-server exists and optimized

2. **Phase 1** (4-6 hours):
   - DKRHUB-1001 through DKRHUB-1006: Build and publish workflow
   - DKRHUB-1901: Test workflow with pre-release tag

3. **Phase 2** (2-3 hours):
   - DKRHUB-2001: Update docker-compose.yml to use images
   - DKRHUB-2002: Create development override
   - DKRHUB-2003: Add Dockerfile metadata
   - DKRHUB-2902, DKRHUB-2903: Test configurations

4. **Phase 3** (2 hours):
   - DKRHUB-3001: Version bump
   - DKRHUB-3002: Tag and trigger workflow
   - DKRHUB-3003: Monitor workflow
   - DKRHUB-3004: Verify Docker Hub images
   - DKRHUB-3005: Publish npm package

**Total MVP Time**: 8-11 hours elapsed, 13 hours total effort

---

## Success Criteria

### Must-Have (Blocking Release)
- [x] GitHub Actions workflow created
- [x] Multi-platform builds working (AMD64, ARM64)
- [x] Images pushed to Docker Hub
- [x] docker-compose.yml updated to use `image:`
- [x] v1.1.10 tagged and built
- [x] npm package published
- [x] End-to-end tests pass on Linux AMD64
- [x] End-to-end tests pass on macOS ARM64

### Should-Have (High Priority)
- [x] docker-compose.override.yml created
- [x] Dockerfile metadata labels added
- [x] Security scanning passing
- [x] README updated
- [x] Migration guide created

### Nice-to-Have (Future)
- [ ] Windows WSL2 testing
- [ ] Image signing (Cosign)
- [ ] SBOM generation
- [ ] Performance benchmarks

---

## Related Tickets (Reference Only)

**Do NOT duplicate** - these already exist:

- **LOCAL-4006**: Docker image optimization (✅ COMPLETED)
- **LOCAL-4005**: ARM64/Apple Silicon testing (relates to multi-platform)
- **LOCAL-3008**: npm publishing workflow (⚠️ INCOMPLETE, complete after DKRHUB)
- **MCPSTART-6004**: Publish npm v1.1.9 (⚠️ BLOCKED, unblocked by DKRHUB)

---

## Next Steps

1. **Begin Phase 1**: Assign DKRHUB-1001 to docker-engineer
2. **Work sequentially**: Complete each phase before starting next
3. **Test thoroughly**: Run all test tickets before proceeding
4. **Monitor workflow**: Verify each step succeeds
5. **Document issues**: Create tickets for any problems discovered

---

## Notes

- All ticket files located in: `.agents/work-tickets/DKRHUB-*.md`
- Ticket naming: `DKRHUB-{PHASE}{NUMBER}_{descriptive-name}.md`
- Test tickets: Use 900s range (e.g., 1901, 2902)
- Follow template at: `.agents/work-tickets/_WORK_TICKET_TEMPLATE.md`

---

**Created**: 2025-10-29
**Status**: Ready for Implementation
**Owner**: docker-engineer (primary)
