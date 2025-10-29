# DKRHUB Tickets Update Plan

**Date**: 2025-10-29
**Purpose**: Document required updates to all DKRHUB tickets based on review findings

---

## Summary of Changes

### New Tickets Created (5)
1. ✅ **DKRHUB-1000**: Create Combined Dockerfile (BLOCKER - prerequisite for all)
2. ✅ **DKRHUB-1007**: Test Dockerfile Locally (validates before GitHub Actions)
3. ✅ **DKRHUB-2004**: Create Test Docker Compose Config (prevents test failures)
4. ✅ **DKRHUB-2904**: Validate Pre-Release Images (validates before production)
5. ✅ **DKRHUB-3006**: Create Rollback Procedure (safety net)

### Critical Global Changes Required

**All tickets must change**:
- ❌ OLD: `Dockerfile.mcp-server`
- ✅ NEW: `Dockerfile.combined`

**Build Context Change**:
- ❌ OLD: `BUILD_CONTEXT: packages/maproom-mcp`
- ✅ NEW: `BUILD_CONTEXT: .` (workspace root)

**Dependency Addition**:
- All Phase 1-4 tickets must add dependency on **DKRHUB-1000** and **DKRHUB-1007**

---

## Phase 1: GitHub Actions Workflow (Tickets 1001-1006, 1901)

### DKRHUB-1001 ✅ (COMPLETED)
- [x] Changed DOCKERFILE_PATH to `Dockerfile.combined`
- [x] Changed BUILD_CONTEXT to `.`
- [x] Added dependencies on DKRHUB-1000 and DKRHUB-1007
- [x] Updated acceptance criteria

### DKRHUB-1002
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1001**: Workflow file must exist before adding these steps

## Implementation Notes
Add note: "Multi-stage caching works with combined Dockerfile's Rust and Node.js stages"
```

### DKRHUB-1003
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1002**: Buildx must be configured before authentication
```

### DKRHUB-1004
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1003**: Authentication must be configured before version extraction
```

### DKRHUB-1005
**Required Changes**:
```markdown
## Technical Requirements
Change:
- file: ${{ env.DOCKERFILE_PATH }} (packages/maproom-mcp/config/Dockerfile.combined)
- context: ${{ env.BUILD_CONTEXT }} (workspace root: .)

## Implementation Notes
Add: "Combined Dockerfile builds both Rust and Node.js components. Build time: ~12-15 min (cold), ~5 min (warm with cache)."

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist and be tested
- **DKRHUB-1007**: Local testing must pass before GitHub Actions build
- **DKRHUB-1004**: Version extraction must be configured before build
```

### DKRHUB-1006
**Required Changes**:
```markdown
## Technical Requirements
Add: "Scans combined image containing both Rust binary and Node.js dependencies"

## Implementation Notes
Add: "Security scan covers: Rust runtime dependencies (libgcc, libssl3), Node.js dependencies (pg, pino, zod, execa), Alpine base packages"

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1005**: Image must be built before scanning
```

### DKRHUB-1901
**Required Changes**:
```markdown
## Acceptance Criteria (ADD):
- [ ] ARM64 image builds successfully (verify in GitHub Actions logs)
- [ ] AMD64 image builds successfully (verify in GitHub Actions logs)
- [ ] Docker manifest includes both platforms: `docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Pull and test on Linux AMD64: `docker pull --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Pull ARM64 image (at minimum verify pullable): `docker pull --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Verify both components exist in image: Node.js runtime + Rust binary

## Testing Requirements (ADD):
- [ ] Test Node.js runtime: `docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 node --version`
- [ ] Test Rust binary: `docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version`
- [ ] Verify image size reasonable (< 450MB)

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist and be tested locally
- **DKRHUB-1007**: Local testing must pass completely
- **DKRHUB-1001 through 1006**: Complete workflow must be implemented
```

---

## Phase 2: Docker Compose Updates (Tickets 2001-2003, 2902-2903)

### DKRHUB-2001
**Required Changes**:
```markdown
## Technical Requirements
Change comment:
"# Use Dockerfile.combined (contains both Rust binary and Node.js MCP server)"

## Implementation Notes (ADD):
"The image pulled from Docker Hub is built using Dockerfile.combined, which includes:
- Rust binary: /usr/local/bin/crewchief-maproom (for scan operations)
- Node.js runtime: node 20 (for MCP server)
- MCP server: /app/dist/index.js (spawns Rust binary as needed)"

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1007**: Local testing must pass
- **DKRHUB-1901**: Pre-release images must be published and tested
```

### DKRHUB-2002
**Required Changes**:
```markdown
## Technical Requirements
Change:
dockerfile: packages/maproom-mcp/config/Dockerfile.combined

Add comment in YAML example:
"# Builds from source using Dockerfile.combined (Rust + Node.js)"

## Implementation Notes (ADD):
"Dockerfile.combined builds both components. Expect 12-15 min build time on first run."

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2001**: Production docker-compose.yml must be updated first
```

### DKRHUB-2003
**Required Changes**:
```markdown
## Summary
Change: "Add metadata labels to Dockerfile.combined"

## Technical Requirements
File: packages/maproom-mcp/config/Dockerfile.combined

Labels section: Add to final runtime stage (Stage 3) of combined Dockerfile

## Implementation Notes (ADD):
"Labels must be added to the final runtime stage (Stage 3: Runtime Image) of Dockerfile.combined, not the builder stages."

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2001**: docker-compose.yml must be updated to use images
```

### DKRHUB-2902
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2001**: Production docker-compose.yml must use image directive
- **DKRHUB-1901**: Pre-release images must be published
```

### DKRHUB-2903
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2002**: docker-compose.override.yml must exist
```

---

## Phase 3: Release v1.1.10 (Tickets 3001-3005)

### DKRHUB-3001
**Required Changes**:
```markdown
## Acceptance Criteria (ADD):
- [ ] Audit npm package contents: Remove unused platform-specific binaries if Docker provides Rust binary
- [ ] Verify package.json "files" array includes only necessary files
- [ ] Measure package size reduction (if binaries removed)
- [ ] Update package.json to 1.1.10
- [ ] Update CHANGELOG.md with Docker Hub publishing changes

## Technical Requirements (ADD):
**Package Audit**:
Check if platform binaries in packages/cli/bin/<platform>/ are still needed:
- If Docker provides Rust binary, these may be redundant
- Consider removing to reduce npm package size
- Update package.json "files" array accordingly

## Implementation Notes (ADD):
"This release includes major architectural change: Docker images now built via GitHub Actions and pulled from Docker Hub instead of built from source. This fixes v1.1.9 deployment failure."

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2001-2003, 2902-2903**: All Phase 2 tickets must be complete
- **DKRHUB-2904**: Pre-release images must be validated
```

### DKRHUB-3002
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-3001**: Version bump must be complete
```

### DKRHUB-3003
**Required Changes**:
```markdown
## Monitoring Points (ADD):
- [ ] Verify workflow uses Dockerfile.combined (check build logs)
- [ ] Verify BUILD_CONTEXT is workspace root (check "context" in logs)
- [ ] Verify both Rust and Node.js build stages complete
- [ ] Check build time (expect ~12-15 min first build, ~5 min with cache)

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-3002**: Git tag must be pushed to trigger workflow
```

### DKRHUB-3004
**Required Changes**:
```markdown
## Verification Steps (ADD):
- [ ] Pull image: `docker pull crewchief/maproom-mcp:1.1.10`
- [ ] Check size: `docker images crewchief/maproom-mcp:1.1.10` (expect < 450MB)
- [ ] Verify Node.js: `docker run --rm crewchief/maproom-mcp:1.1.10 node --version`
- [ ] Verify Rust: `docker run --rm crewchief/maproom-mcp:1.1.10 crewchief-maproom --version`
- [ ] Check both platforms: `docker manifest inspect crewchief/maproom-mcp:1.1.10`

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must have been used in build
- **DKRHUB-3003**: Workflow must complete successfully
```

### DKRHUB-3005
**Required Changes**:
```markdown
## Pre-Publish Checklist (ADD):
- [ ] Docker images validated (DKRHUB-3004 complete)
- [ ] Both platforms work (AMD64, ARM64)
- [ ] Rollback procedure documented (DKRHUB-3006 complete)
- [ ] Package contents audited (platform binaries reviewed)

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-2904**: Pre-release validation must pass
- **DKRHUB-3004**: Docker Hub images must be verified
- **DKRHUB-3006**: Rollback procedure must be documented
```

---

## Phase 4: Validation & Documentation (Tickets 4001-4005)

### DKRHUB-4001
**Required Changes**:
```markdown
## Testing Requirements (ADD):
- [ ] Verify both components in pulled image: Node.js + Rust binary
- [ ] Test Rust binary spawning: Verify index.js can spawn crewchief-maproom
- [ ] Check image built from Dockerfile.combined (verify in Docker labels)

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must have been used
- **DKRHUB-3005**: npm package v1.1.10 must be published
```

### DKRHUB-4002
**Required Changes**:
```markdown
## Testing Requirements (ADD):
- [ ] Verify ARM64 image contains both components: Node.js + Rust binary
- [ ] Test on Apple Silicon if available (native performance)
- [ ] OR use QEMU emulation for ARM64 testing

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must have been used
- **DKRHUB-3005**: npm package v1.1.10 must be published
```

### DKRHUB-4003
**Required Changes**:
```markdown
## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-4001**: Linux AMD64 testing must pass first
```

### DKRHUB-4004
**Required Changes**:
```markdown
## Documentation Sections (ADD):
**Architecture Change**:
"v1.1.10 introduces Docker Hub publishing. Images are pre-built via GitHub Actions and contain both:
- Rust indexing binary (crewchief-maproom)
- Node.js MCP server (index.js)

Previously, images were built from source on user's machine, which failed in deployed npm packages."

**Dockerfile Information**:
"Images built from Dockerfile.combined (multi-stage build):
- Stage 1: Builds Rust binary
- Stage 2: Compiles TypeScript to JavaScript
- Stage 3: Combines both in Node.js Alpine runtime (~400MB)"

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-4001, 4002**: Testing must be complete
```

### DKRHUB-4005
**Required Changes**:
```markdown
## Migration Notes (ADD):
**What Changed**:
- v1.1.9: Built from source (context: ../../.., Dockerfile.maproom)
- v1.1.10: Pulls from Docker Hub (image: crewchief/maproom-mcp:1.1.10)
- Dockerfile.combined: New multi-stage build with Rust + Node.js

**Why It Failed**:
v1.1.9's build context (../../..) only worked in development workspace, not deployed npm packages

**Why v1.1.10 Works**:
Images pre-built via GitHub Actions, docker-compose.yml pulls ready-made images

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-4004**: README must be updated first
```

---

## Updated Ticket Dependencies Graph

```
DKRHUB-1000 (Combined Dockerfile) [CRITICAL BLOCKER]
    └─► DKRHUB-1007 (Test Dockerfile Locally)
        └─► DKRHUB-1001 (GitHub Actions Workflow)
            └─► DKRHUB-1002 (Multi-Platform Build)
                └─► DKRHUB-1003 (Docker Hub Auth)
                    └─► DKRHUB-1004 (Version Extraction)
                        └─► DKRHUB-1005 (Image Build & Push)
                            └─► DKRHUB-1006 (Security Scanning)
                                └─► DKRHUB-1901 (Test Workflow Pre-Release)
                                    ├─► DKRHUB-2001 (Update docker-compose.yml)
                                    │   ├─► DKRHUB-2002 (Development Override)
                                    │   ├─► DKRHUB-2003 (Dockerfile Metadata)
                                    │   ├─► DKRHUB-2004 (Test Docker Compose) [NEW]
                                    │   ├─► DKRHUB-2902 (Test Production Config)
                                    │   └─► DKRHUB-2903 (Test Dev Config)
                                    └─► DKRHUB-2904 (Validate Pre-Release) [NEW]
                                        └─► DKRHUB-3001 (Update Package Version)
                                            ├─► DKRHUB-3006 (Rollback Procedure) [NEW]
                                            └─► DKRHUB-3002 (Create Git Tag)
                                                └─► DKRHUB-3003 (Monitor Workflow)
                                                    └─► DKRHUB-3004 (Verify Docker Hub)
                                                        └─► DKRHUB-3005 (Publish npm)
                                                            ├─► DKRHUB-4001 (E2E Linux AMD64)
                                                            ├─► DKRHUB-4002 (E2E macOS ARM64)
                                                            ├─► DKRHUB-4003 (Test Version Pinning)
                                                            ├─► DKRHUB-4004 (Update README)
                                                            └─► DKRHUB-4005 (Migration Guide)
```

---

## Implementation Order (Revised)

### Must Complete First (Critical Path)
1. **DKRHUB-1000**: Create Dockerfile.combined (4-6 hours) [BLOCKER]
2. **DKRHUB-1007**: Test Dockerfile locally (2-3 hours) [BLOCKER]

### Phase 1: Workflow (After 1000, 1007 complete)
3. DKRHUB-1001: Create workflow file (2h)
4. DKRHUB-1002: Configure multi-platform (1h)
5. DKRHUB-1003: Docker Hub auth (0.5h)
6. DKRHUB-1004: Version extraction (1h)
7. DKRHUB-1005: Build and push (1.5h)
8. DKRHUB-1006: Security scanning (0.5h)
9. DKRHUB-1901: Test workflow pre-release (1h)

### Phase 2: Docker Compose
10. DKRHUB-2001: Update docker-compose.yml (0.5h)
11. DKRHUB-2002: Development override (0.5h)
12. DKRHUB-2003: Dockerfile metadata (0.5h)
13. DKRHUB-2004: Test Docker Compose (1h) [NEW]
14. DKRHUB-2902: Test production config (0.5h)
15. DKRHUB-2903: Test development config (0.5h)
16. DKRHUB-2904: Validate pre-release (2h) [NEW]

### Phase 3: Release
17. DKRHUB-3006: Rollback procedure (1.5h) [NEW]
18. DKRHUB-3001: Version bump + audit (0.5h)
19. DKRHUB-3002: Create git tag (0.25h)
20. DKRHUB-3003: Monitor workflow (0.5h)
21. DKRHUB-3004: Verify Docker Hub (0.25h)
22. DKRHUB-3005: Publish npm (0.5h)

### Phase 4: Validation
23. DKRHUB-4001: E2E Linux AMD64 (1h)
24. DKRHUB-4002: E2E macOS ARM64 (1h)
25. DKRHUB-4003: Version pinning (0.5h)
26. DKRHUB-4004: Update README (1h)
27. DKRHUB-4005: Migration guide (0.5h)

---

## Revised Timeline

**Original Estimate**: 17.5 hours (22 tickets)
**Revised Estimate**: 26 hours (27 tickets)

**Breakdown**:
- Prerequisite (1000, 1007): 6-9 hours
- Phase 1 (Workflow): 7.5 hours
- Phase 2 (Docker Compose): 5 hours
- Phase 3 (Release): 3.25 hours
- Phase 4 (Validation): 4 hours

**Total**: 25.75 hours (~26 hours)
**Timeline**: 3-4 days with parallel work

---

## Next Steps

1. **User Review**: Review this update plan
2. **Apply Updates**: Systematically update all 22 existing tickets
3. **Update Index**: Update DKRHUB_TICKET_INDEX.md with new tickets
4. **Begin Implementation**: Start with DKRHUB-1000 (after user approval)

---

**Update Plan Status**: COMPLETE
**Ready for Implementation**: After user approval
