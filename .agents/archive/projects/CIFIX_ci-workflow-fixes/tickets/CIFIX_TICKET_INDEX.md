# CIFIX Ticket Index

**Project**: CI Workflow Fixes
**Total Tickets**: 10 (8 required + 2 optional)
**Status**: All tickets created, ready for implementation

---

## Phase 1: Test Workflow Fix (2 tickets)

### CIFIX-1001: Remove explicit pnpm version from test.yml
- **Status**: Ready
- **Agent**: github-actions-specialist
- **Estimated Time**: 5-10 minutes
- **Priority**: High
- **Dependencies**: None
- **File**: CIFIX-1001_remove-pnpm-version-from-test-workflow.md
- **Summary**: Remove `version: 10` from test.yml to eliminate pnpm version conflict error
- **Plan Reference**: plan.md lines 17-69

### CIFIX-1002: Update workflow documentation
- **Status**: Ready
- **Agent**: github-actions-specialist
- **Estimated Time**: 5 minutes
- **Priority**: Medium
- **Dependencies**: CIFIX-1001
- **File**: CIFIX-1002_update-workflow-documentation.md
- **Summary**: Add troubleshooting docs to `.github/CLAUDE.md` for pnpm auto-detection
- **Plan Reference**: plan.md lines 38-42

---

## Phase 2: Docker Build Fix (5 tickets)

### CIFIX-2005: Update release workflow with pnpm build step ⚠️ CRITICAL
- **Status**: Ready
- **Agent**: github-actions-specialist
- **Estimated Time**: 10 minutes
- **Priority**: CRITICAL - MUST BE FIRST IN PHASE 2
- **Dependencies**: None (blocks all other Phase 2 tickets)
- **File**: CIFIX-2005_update-release-workflow-pnpm-build.md
- **Summary**: Add pnpm setup and build steps to release workflow BEFORE Docker build
- **Plan Reference**: plan.md lines 85-132
- **Why Critical**: Docker build will fail 100% without daemon-client dist/ directory

### CIFIX-2001: Add pnpm to Docker builder stage
- **Status**: Ready
- **Agent**: docker-engineer
- **Estimated Time**: 10 minutes
- **Priority**: High
- **Dependencies**: CIFIX-2005
- **File**: CIFIX-2001_add-pnpm-to-docker-builder.md
- **Summary**: Install pnpm@10.12.1 in Dockerfile Stage 2 for workspace resolution
- **Plan Reference**: plan.md lines 135-156

### CIFIX-2002: Update Dockerfile for workspace dependencies
- **Status**: Ready
- **Agent**: docker-engineer
- **Estimated Time**: 20 minutes
- **Priority**: High
- **Dependencies**: CIFIX-2005, CIFIX-2001
- **File**: CIFIX-2002_update-dockerfile-workspace-dependencies.md
- **Summary**: Modify Dockerfile to copy workspace configs and use pnpm install --filter
- **Plan Reference**: plan.md lines 158-228
- **Reference**: architecture.md lines 131-220 (precise diff)

### CIFIX-2003: Test multi-platform Docker build
- **Status**: Ready
- **Agent**: docker-engineer
- **Estimated Time**: 25 minutes
- **Priority**: High
- **Dependencies**: CIFIX-2005, CIFIX-2001, CIFIX-2002
- **File**: CIFIX-2003_test-multi-platform-docker-build.md
- **Summary**: Validate Docker build locally with daemon-client dist/ and pnpm version checks
- **Plan Reference**: plan.md lines 230-308

### CIFIX-2004: Update Docker build documentation
- **Status**: Ready
- **Agent**: docker-engineer
- **Estimated Time**: 10 minutes
- **Priority**: Medium
- **Dependencies**: CIFIX-2001, CIFIX-2002, CIFIX-2003
- **File**: CIFIX-2004_update-docker-build-documentation.md
- **Summary**: Add inline comments to Dockerfile and Docker Build section to CLAUDE.md
- **Plan Reference**: plan.md lines 310-313

---

## Phase 3: Documentation and Monitoring (3 tickets)

### CIFIX-3001: Update project documentation
- **Status**: Ready
- **Agent**: General implementation agent
- **Estimated Time**: 15 minutes
- **Priority**: Medium
- **Dependencies**: CIFIX-1002, CIFIX-2004
- **File**: CIFIX-3001_update-project-documentation.md
- **Summary**: Add comprehensive troubleshooting section to `.github/CLAUDE.md`
- **Plan Reference**: plan.md lines 196-250

### CIFIX-3002: Add troubleshooting guides
- **Status**: Ready
- **Agent**: General implementation agent
- **Estimated Time**: 10 minutes
- **Priority**: Medium
- **Dependencies**: CIFIX-3001
- **File**: CIFIX-3002_add-troubleshooting-guides.md
- **Summary**: Add step-by-step debugging procedures and rollback guides
- **Plan Reference**: plan.md lines 226-258

### CIFIX-3003: Set up monitoring (OPTIONAL)
- **Status**: Ready (Optional - recommend defer)
- **Agent**: github-actions-specialist
- **Estimated Time**: 2 minutes (defer) or 20 minutes (implement)
- **Priority**: Optional - Not required for MVP
- **Dependencies**: CIFIX-3001, CIFIX-3002
- **File**: CIFIX-3003_setup-monitoring.md
- **Summary**: Optional Slack/Discord webhook monitoring for CI failures
- **Plan Reference**: plan.md lines 262-266
- **Recommendation**: DEFER - Mark as future enhancement

---

## Execution Order

### Required Sequence (8 tickets):
1. **CIFIX-1001** → CIFIX-1002 (Phase 1: Test workflow - can run in parallel with Phase 2)
2. **CIFIX-2005** ⚠️ MUST BE FIRST IN PHASE 2
3. **CIFIX-2001** (requires CIFIX-2005)
4. **CIFIX-2002** (requires CIFIX-2005, CIFIX-2001)
5. **CIFIX-2003** (requires CIFIX-2005, CIFIX-2001, CIFIX-2002)
6. **CIFIX-2004** (requires CIFIX-2001, CIFIX-2002, CIFIX-2003)
7. **CIFIX-3001** (requires CIFIX-1002, CIFIX-2004)
8. **CIFIX-3002** (requires CIFIX-3001)

### Optional Tickets:
9. **CIFIX-3003** - DEFER to future work (not blocking)

---

## Critical Path

**BLOCKER**: CIFIX-2005 must complete before any other Phase 2 ticket

**Why**: The release workflow must run `pnpm build` to create daemon-client dist/ before Docker build. Without this, all Docker changes in CIFIX-2001, 2002, 2003 will fail because Dockerfile copies daemon-client/dist/ which won't exist.

**Critical Dependencies**:
- CIFIX-2005 → (blocks) → CIFIX-2001, CIFIX-2002, CIFIX-2003
- CIFIX-1001 → (context for) → CIFIX-1002
- CIFIX-2001 + CIFIX-2002 → (required for) → CIFIX-2003
- CIFIX-1002 + CIFIX-2004 → (context for) → CIFIX-3001
- CIFIX-3001 → (extends) → CIFIX-3002

---

## Quick Reference

### By Agent Assignment

**github-actions-specialist**:
- CIFIX-1001 (Phase 1)
- CIFIX-1002 (Phase 1)
- CIFIX-2005 (Phase 2 - CRITICAL FIRST)
- CIFIX-3003 (Phase 3 - Optional)

**docker-engineer**:
- CIFIX-2001 (Phase 2)
- CIFIX-2002 (Phase 2)
- CIFIX-2003 (Phase 2)
- CIFIX-2004 (Phase 2)

**General implementation agent**:
- CIFIX-3001 (Phase 3)
- CIFIX-3002 (Phase 3)

### By Estimated Time

**Quick wins** (<10 min):
- CIFIX-1001: 5-10 min
- CIFIX-1002: 5 min
- CIFIX-2005: 10 min
- CIFIX-2001: 10 min
- CIFIX-2004: 10 min
- CIFIX-3002: 10 min

**Medium effort** (10-20 min):
- CIFIX-2002: 20 min
- CIFIX-3001: 15 min

**Longer tasks** (>20 min):
- CIFIX-2003: 25 min (includes validation execution time)
- CIFIX-3003: 20 min (if implemented)

**Total Time**: ~2 hours for required tickets (excluding CIFIX-3003)

---

## Success Criteria

**Phase 1 Complete When**:
- ✅ Test workflow runs without "Multiple versions of pnpm" error
- ✅ pnpm version auto-detected from package.json
- ✅ Documentation explains auto-detection behavior

**Phase 2 Complete When**:
- ✅ Release workflow builds daemon-client before Docker
- ✅ Docker build completes without workspace: protocol errors
- ✅ Local amd64 build succeeds with validation
- ✅ Image size ~220MB (±10MB)
- ✅ Documentation explains pnpm workspace strategy

**Phase 3 Complete When**:
- ✅ Comprehensive troubleshooting guide in `.github/CLAUDE.md`
- ✅ Step-by-step debugging procedures documented
- ✅ Rollback procedures documented
- ⚠️ Monitoring setup (optional - can defer)

**Project Complete When**:
- ✅ All 8 required tickets closed
- ✅ Test workflow passing consistently
- ✅ Docker builds completing successfully
- ✅ CI green for 3+ consecutive runs
- ✅ No manual interventions needed

---

## Notes

### Phase Execution Strategy
- Phase 1 can run in parallel with Phase 2 (independent fixes)
- Within Phase 2: CIFIX-2005 MUST complete first (critical blocker)
- Phase 3 requires completion of Phases 1 and 2 (documentation context)

### MVP vs Future Work
- **MVP**: All Phase 1, Phase 2, and CIFIX-3001, CIFIX-3002
- **Future**: CIFIX-3003 (monitoring) - defer unless specific need identified
