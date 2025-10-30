# DKRHUB: Docker Hub Publishing - Ticket Index (UPDATED)

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Updated**: 2025-10-29 (Post-Review)
**Status**: ⚠️  BLOCKED - Awaiting DKRHUB-1000 and ticket updates

## ⚠️ CRITICAL UPDATE - Review Findings

**Status**: All tickets require updates before implementation can proceed.

**Blocking Issues Found**:
1. 🔴 Wrong Dockerfile referenced (all tickets use `Dockerfile.mcp-server`, should be `Dockerfile.combined`)
2. 🔴 Wrong BUILD_CONTEXT (should be workspace root `.`, not `packages/maproom-mcp`)
3. 🔴 Missing prerequisite ticket (DKRHUB-1000: Create Combined Dockerfile)
4. 🟡 Missing testing/validation tickets

**Review Documents**:
- **Full Review**: `DKRHUB_TICKETS_REVIEW_REPORT.md` (9 issues identified)
- **Update Plan**: `DKRHUB_TICKETS_UPDATE_PLAN.md` (detailed changes for all 27 tickets)

---

## Project Overview

This project fixes the critical v1.1.9 deployment failure by implementing automated Docker image publishing to Docker Hub via GitHub Actions. The npm package will pull pre-built images instead of attempting to build from source.

**Problem**: v1.1.9 docker-compose.yml tries to build from `context: ../../..` which doesn't exist in deployed npm packages.

**Root Cause** (Discovered): Neither existing Dockerfile works for the MCP server architecture:
- `Dockerfile.maproom`: Only Rust binary (missing Node.js)
- `Dockerfile.mcp-server`: Only Node.js (missing Rust binary)
- **Required**: Both Rust binary AND Node.js runtime in single image

**Solution**:
1. Create `Dockerfile.combined` that builds both components
2. Publish multi-platform images to Docker Hub via GitHub Actions
3. Update docker-compose.yml to use `image:` instead of `build:`

**Documentation**:
- Analysis: `.agents/projects/DKRHUB-Docker_Hub_Publishing/DKRHUB_ANALYSIS.md`
- Architecture: `.agents/projects/DKRHUB-Docker_Hub_Publishing/DKRHUB_ARCHITECTURE.md`
- Security: `.agents/projects/DKRHUB-Docker_Hub_Publishing/DKRHUB_SECURITY_REVIEW.md`
- Quality: `.agents/projects/DKRHUB-Docker_Hub_Publishing/DKRHUB_QUALITY_STRATEGY.md`
- Plan: `.agents/projects/DKRHUB-Docker_Hub_Publishing/DKRHUB_PLAN.md`
- **Review**: `DKRHUB_TICKETS_REVIEW_REPORT.md` [NEW]
- **Update Plan**: `DKRHUB_TICKETS_UPDATE_PLAN.md` [NEW]

---

## Phase 0: Prerequisites (P0 - CRITICAL BLOCKERS) [NEW]

**Objective**: Create and test combined Dockerfile before any workflow implementation

**Duration**: Day 1 - 6-9 hours
**Agent**: docker-engineer

| Ticket | Title | Effort | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| DKRHUB-1000 | Create Combined Dockerfile | 4-6h | None | ⬜ Not Started |
| DKRHUB-1007 | Test Dockerfile Locally | 2-3h | 1000 | ⬜ Not Started |

**Phase 0 Deliverable**: Working Dockerfile.combined that builds both Rust and Node.js components, validated locally

**BLOCKS**: All other DKRHUB tickets (Phase 1-4)

---

## Phase 1: GitHub Actions Workflow (P0 - Critical Path)

**Objective**: Create automated workflow to build and publish Docker images

**Duration**: Day 2 - 7.5 hours
**Agent**: docker-engineer

### Implementation Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-1001 | Create GitHub Actions Workflow File | 2h | 1000, 1007 | ⬜ Not Started | ✅ Partial (Dockerfile path) |
| DKRHUB-1002 | Configure Multi-Platform Build | 1h | 1000, 1001 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-1003 | Implement Docker Hub Authentication | 0.5h | 1000, 1002 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-1004 | Implement Version Extraction and Tagging | 1h | 1000, 1003 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-1005 | Configure Image Build and Push | 1.5h | 1000, 1007, 1004 | ⬜ Not Started | ⚠️ Update Dockerfile ref, deps |
| DKRHUB-1006 | Add Security Scanning with Trivy | 0.5h | 1000, 1005 | ⬜ Not Started | ⚠️ Add deps |

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-1901 | Test Workflow with Pre-Release Tag | 1h | 1000, 1007, 1001-1006 | ⬜ Not Started | ⚠️ Add ARM64 testing, deps |

**Phase 1 Deliverable**: Working GitHub Actions workflow that builds and publishes multi-platform images to Docker Hub

---

## Phase 2: Docker Compose Updates (P0 - Critical Path)

**Objective**: Update docker-compose configuration to use pre-built images

**Duration**: Day 2-3 - 5 hours
**Agent**: docker-engineer, integration-tester

### Implementation Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-2001 | Update docker-compose.yml to Use Images | 0.5h | 1000, 1007, 1901 | ⬜ Not Started | ⚠️ Update comments, deps |
| DKRHUB-2002 | Create docker-compose.override.yml for Development | 0.5h | 1000, 2001 | ⬜ Not Started | ⚠️ Update Dockerfile ref |
| DKRHUB-2003 | Add Dockerfile Metadata Labels | 0.5h | 1000, 2001 | ⬜ Not Started | ⚠️ Fix file path, deps |
| DKRHUB-2004 | Create Test Docker Compose Config | 1h | 1000, 1007 | ⬜ Not Started | ✅ NEW TICKET |

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-2902 | Test Production Configuration (Image Pull) | 0.5h | 1000, 2001, 1901 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-2903 | Test Development Configuration (Local Build) | 0.5h | 1000, 2002 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-2904 | Validate Pre-Release Images | 2h | 1000, 1901, 2001 | ⬜ Not Started | ✅ NEW TICKET |

**Phase 2 Deliverable**: Production-ready docker-compose.yml that pulls images; development override for local builds; validated pre-release images

---

## Phase 3: Release v1.1.10 (P0 - Critical Path)

**Objective**: Publish fixed version to npm and Docker Hub

**Duration**: Day 3 - 3.25 hours
**Agent**: docker-engineer

### Release Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-3001 | Update Package Version to v1.1.10 | 0.5h | 1000, 2001-2004, 2902-2904 | ⬜ Not Started | ⚠️ Add package audit, deps |
| DKRHUB-3002 | Create and Push Git Tag v1.1.10 | 0.25h | 1000, 3001 | ⬜ Not Started | ⚠️ Add deps |
| DKRHUB-3003 | Monitor GitHub Actions Workflow Execution | 0.5h | 1000, 3002 | ⬜ Not Started | ⚠️ Add monitoring points |
| DKRHUB-3004 | Verify Images on Docker Hub | 0.25h | 1000, 3003 | ⬜ Not Started | ⚠️ Add verification steps |
| DKRHUB-3005 | Publish npm Package v1.1.10 | 0.5h | 1000, 2904, 3004, 3006 | ⬜ Not Started | ⚠️ Add pre-publish checklist |
| DKRHUB-3006 | Create Rollback Procedure | 1.5h | N/A | ⬜ Not Started | ✅ NEW TICKET |

**Phase 3 Deliverable**: v1.1.10 published to npm and Docker Hub, ready for users, with rollback procedure documented

---

## Phase 4: Validation & Documentation (P1 - High Priority)

**Objective**: Verify release works end-to-end and update documentation

**Duration**: Day 3-4 - 4 hours
**Agents**: integration-tester, docker-engineer

### Testing Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-4001 | End-to-End Testing on Linux AMD64 | 1h | 1000, 3005 | ⬜ Not Started | ⚠️ Add component verification |
| DKRHUB-4002 | End-to-End Testing on macOS ARM64 | 1h | 1000, 3005 | ⬜ Not Started | ⚠️ Add ARM64 specifics |
| DKRHUB-4003 | Test Version Pinning Functionality | 0.5h | 1000, 4001 | ⬜ Not Started | ⚠️ Add deps |

### Documentation Tickets

| Ticket | Title | Effort | Dependencies | Status | Updates Required |
|--------|-------|--------|--------------|--------|------------------|
| DKRHUB-4004 | Update README with Docker Hub Information | 1h | 1000, 4001, 4002 | ⬜ Not Started | ⚠️ Add architecture section |
| DKRHUB-4005 | Create Migration Guide v1.1.9 to v1.1.10 | 0.5h | 1000, 4004 | ⬜ Not Started | ⚠️ Add Dockerfile changes |

**Phase 4 Deliverable**: Fully tested and documented v1.1.10 release

---

## Ticket Summary

### By Phase
- **Phase 0 (NEW)**: 2 tickets (Prerequisites - BLOCKERS)
- **Phase 1**: 7 tickets (6 implementation + 1 test)
- **Phase 2**: 7 tickets (4 implementation + 3 test, includes 2 new)
- **Phase 3**: 6 tickets (5 release + 1 new rollback)
- **Phase 4**: 5 tickets (3 test + 2 documentation)
- **Total**: 27 tickets (was 22, added 5)

### By Priority
- **P0 (Critical - BLOCKERS)**: 2 tickets (Phase 0)
- **P0 (Critical)**: 17 tickets (Phases 1-3)
- **P1 (High)**: 5 tickets (Phase 4)
- **P2 (Nice-to-Have)**: 3 tickets (Future)

### By Effort
- **Original Estimate**: 17.5 hours (22 tickets)
- **Revised Estimate**: 26 hours (27 tickets)
- **Added Time**: +8.5 hours (Dockerfile creation, validation, rollback)

### By Agent
- **docker-engineer**: 19 tickets
- **integration-tester**: 6 tickets
- **verify-ticket**: All tickets (verification step)
- **commit-ticket**: All tickets (commit step)

### By Status
- **✅ Created (NEW)**: 5 tickets (1000, 1007, 2004, 2904, 3006)
- **⚠️ Needs Updates**: 22 tickets (all original tickets need Dockerfile refs fixed)
- **🔴 BLOCKED**: All 27 tickets (until DKRHUB-1000 complete)

---

## Critical Path (MVP)

**STOP**: Do NOT proceed with original Phase 1 until Prerequisites complete!

### Prerequisites (MUST complete first)
1. **Phase 0 - NEW**:
   - DKRHUB-1000: Create Dockerfile.combined (4-6 hours) [BLOCKER]
   - DKRHUB-1007: Test Dockerfile locally (2-3 hours) [BLOCKER]
   - **Update all 22 original tickets** with correct Dockerfile references

2. **Phase 1** (after 1000, 1007 complete) (7.5 hours):
   - DKRHUB-1001 through DKRHUB-1006: Build and publish workflow
   - DKRHUB-1901: Test workflow with pre-release tag

3. **Phase 2** (5 hours):
   - DKRHUB-2001: Update docker-compose.yml to use images
   - DKRHUB-2002: Create development override
   - DKRHUB-2003: Add Dockerfile metadata
   - DKRHUB-2004: Test Docker Compose config [NEW]
   - DKRHUB-2902, DKRHUB-2903: Test configurations
   - DKRHUB-2904: Validate pre-release images [NEW]

4. **Phase 3** (3.25 hours):
   - DKRHUB-3006: Rollback procedure [NEW]
   - DKRHUB-3001: Version bump + package audit
   - DKRHUB-3002: Tag and trigger workflow
   - DKRHUB-3003: Monitor workflow
   - DKRHUB-3004: Verify Docker Hub images
   - DKRHUB-3005: Publish npm package

**Total Critical Path Time**: 6-9 hours (Phase 0) + 15.75 hours (Phases 1-3) = **21.75-24.75 hours**

---

## Success Criteria

### Must-Have (Blocking Release)
- [ ] **Dockerfile.combined created and tested** [NEW - BLOCKER]
- [ ] **All tickets updated with correct Dockerfile references** [NEW - BLOCKER]
- [ ] GitHub Actions workflow created
- [ ] Multi-platform builds working (AMD64, ARM64)
- [ ] Images pushed to Docker Hub
- [ ] docker-compose.yml updated to use `image:`
- [ ] **Pre-release images validated** [NEW]
- [ ] v1.1.10 tagged and built
- [ ] npm package published
- [ ] **Rollback procedure documented** [NEW]
- [ ] End-to-end tests pass on Linux AMD64
- [ ] End-to-end tests pass on macOS ARM64

### Should-Have (High Priority)
- [ ] docker-compose.override.yml created
- [ ] docker-compose.test.yml created [NEW]
- [ ] Dockerfile metadata labels added
- [ ] Security scanning passing
- [ ] README updated
- [ ] Migration guide created

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

## Revised Timeline

**Original Timeline**: 2-3 days (17.5 hours)
**Revised Timeline**: 3-4 days (26 hours)

### Day 1: Prerequisites
- Morning: DKRHUB-1000 (Create Dockerfile.combined) - 4-6 hours
- Afternoon: DKRHUB-1007 (Test locally) - 2-3 hours
- Evening: Update all 22 tickets with correct references - 1-2 hours

### Day 2: Phase 1 + Phase 2 Start
- Morning: DKRHUB-1001 through 1004 - 4.5 hours
- Afternoon: DKRHUB-1005, 1006, 1901 - 3 hours
- Evening: DKRHUB-2001, 2002, 2003, 2004 - 2 hours

### Day 3: Phase 2 Finish + Phase 3
- Morning: DKRHUB-2902, 2903, 2904 - 3 hours
- Afternoon: DKRHUB-3006, 3001, 3002, 3003 - 2.75 hours
- Evening: DKRHUB-3004, 3005 - 0.75 hours

### Day 4: Phase 4
- Morning: DKRHUB-4001, 4002 - 2 hours
- Afternoon: DKRHUB-4003, 4004, 4005 - 2 hours

---

## Next Steps (REVISED)

1. ✅ **Complete Review** - DONE
   - Review report: `DKRHUB_TICKETS_REVIEW_REPORT.md`
   - Update plan: `DKRHUB_TICKETS_UPDATE_PLAN.md`

2. ⏳ **User Approval** - PENDING
   - User reviews findings
   - User approves proceeding with updates

3. ⬜ **Apply Updates** - NOT STARTED
   - Update all 22 existing tickets per `DKRHUB_TICKETS_UPDATE_PLAN.md`
   - Verify all references to Dockerfile.combined and BUILD_CONTEXT: .
   - Add dependency on DKRHUB-1000 to all relevant tickets

4. ⬜ **Begin Implementation** - BLOCKED
   - Start with DKRHUB-1000 (Create Dockerfile.combined)
   - Then DKRHUB-1007 (Test locally)
   - Then proceed to original Phase 1

---

## Notes

- All ticket files located in: `.agents/work-tickets/DKRHUB-*.md`
- Ticket naming: `DKRHUB-{PHASE}{NUMBER}_{descriptive-name}.md`
- Test tickets: Use 900s range (e.g., 1901, 2902)
- NEW tickets: 1000, 1007, 2004, 2904, 3006
- Follow template at: `.agents/work-tickets/_WORK_TICKET_TEMPLATE.md`

**⚠️ IMPORTANT**: Do NOT proceed with Phase 1 until Phase 0 (Prerequisites) is complete and all tickets are updated!

---

**Created**: 2025-10-29
**Updated**: 2025-10-29 (Post-Review)
**Status**: ⚠️ BLOCKED - Awaiting updates and DKRHUB-1000
**Owner**: docker-engineer (primary)
**Blocker**: DKRHUB-1000 (Create Combined Dockerfile)
