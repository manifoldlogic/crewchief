# DKRHUB Tickets Review Report
**Date**: 2025-10-29
**Reviewer**: Claude Code
**Project**: DKRHUB (Docker Hub Publishing)
**Ticket Range**: DKRHUB-1001 through DKRHUB-4005 (22 tickets)

---

## Executive Summary

**CRITICAL BLOCKER FOUND**: All DKRHUB tickets reference the WRONG Dockerfile. The tickets cannot proceed as written without fixing the Dockerfile architecture.

**Status**: 🔴 **BLOCKED - Cannot Proceed**

**Impact**: High - Complete rework required before any implementation

---

## Critical Issue #1: Dockerfile Mismatch 🔴

### Problem Statement

All DKRHUB tickets specify building with `Dockerfile.mcp-server`, but the actual production docker-compose.yml uses `Dockerfile.maproom`. **Neither Dockerfile is architecturally correct for the MCP server deployment model.**

### Evidence

1. **Current Production Configuration** (docker-compose.yml:88-90):
   ```yaml
   maproom-mcp:
     build:
       context: ../../..
       dockerfile: packages/maproom-mcp/config/Dockerfile.maproom  # ← CURRENT
   ```

2. **DKRHUB Tickets Specify** (all tickets):
   ```yaml
   DOCKERFILE_PATH: packages/maproom-mcp/config/Dockerfile.mcp-server  # ← WRONG
   ```

3. **Architecture Requirements** (bin/cli.cjs:957):
   ```bash
   docker exec -i maproom-mcp node /app/dist/index.js
   ```
   The maproom-mcp container MUST contain:
   - ✅ Node.js runtime (for running index.js)
   - ✅ Compiled TypeScript in /app/dist/
   - ✅ Rust binary `crewchief-maproom` (spawned by index.js)

4. **Current Dockerfiles Are Incomplete**:
   - **Dockerfile.maproom**: Builds ONLY Rust binary (crewchief-maproom)
     - ❌ Missing: Node.js runtime
     - ❌ Missing: TypeScript compiled dist/
     - ❌ Missing: npm dependencies (pg, pino, zod, execa)

   - **Dockerfile.mcp-server**: Builds ONLY Node.js MCP server
     - ❌ Missing: Rust binary (crewchief-maproom)
     - ❌ Missing: Rust build toolchain and dependencies

### Root Cause Analysis

The MCP server architecture requires **BOTH** components:
1. **Node.js MCP server** (`index.ts`) - Handles stdio JSON-RPC protocol
2. **Rust indexing binary** (`crewchief-maproom`) - Performs scan operations

The current Dockerfiles were created separately for different purposes:
- `Dockerfile.maproom` - Created for standalone Rust binary deployment
- `Dockerfile.mcp-server` - Created for Node.js MCP server

Neither was designed for the integrated architecture where Node spawns Rust.

### Impact on DKRHUB Tickets

**All Phase 1-3 tickets are blocked** because they reference the wrong Dockerfile:

**Phase 1 - Workflow Creation**:
- ❌ DKRHUB-1001: Sets `DOCKERFILE_PATH: packages/maproom-mcp/config/Dockerfile.mcp-server`
- ❌ DKRHUB-1005: Builds with `${{ env.DOCKERFILE_PATH }}`
- ❌ DKRHUB-1006: Scans image built from wrong Dockerfile

**Phase 2 - Docker Compose Updates**:
- ❌ DKRHUB-2001: Updates docker-compose.yml but doesn't address Dockerfile mismatch
- ❌ DKRHUB-2002: Creates override with wrong Dockerfile path
- ❌ DKRHUB-2003: Adds metadata to wrong Dockerfile

**Phase 3 - Release**:
- ❌ DKRHUB-3001 through 3005: Would publish broken images

**Phase 4 - Validation**:
- ❌ DKRHUB-4001, 4002: Would test broken images
- ❌ DKRHUB-4004, 4005: Documentation would be incorrect

### Recommended Solution

**Option 1: Create New Combined Dockerfile** (RECOMMENDED)
Create `Dockerfile.combined` that builds BOTH components:

```dockerfile
# Stage 1: Build Rust binary
FROM rust:1.82-slim AS rust-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/maproom/ ./crates/maproom/
RUN cargo build --release --bin crewchief-maproom

# Stage 2: Build Node.js MCP server
FROM node:20-alpine AS node-builder
WORKDIR /build
COPY packages/maproom-mcp/package.json ./
RUN npm install --production=false
COPY packages/maproom-mcp/tsconfig.json ./
COPY packages/maproom-mcp/src/ ./src/
RUN npx tsc

# Stage 3: Runtime with both components
FROM node:20-alpine
RUN apk add --no-cache ca-certificates libgcc libssl3 postgresql-client
# Copy Rust binary
COPY --from=rust-builder /build/target/release/crewchief-maproom /usr/local/bin/
# Copy Node.js app
WORKDIR /app
COPY --from=node-builder /build/package.json ./
RUN npm install --production
COPY --from=node-builder /build/dist ./dist
USER node
ENTRYPOINT ["node", "/app/dist/index.js"]
```

**Option 2: Fix Dockerfile.maproom** (ALTERNATIVE)
Extend the existing Dockerfile.maproom to include Node.js in the runtime stage.

**Option 3: Fix Dockerfile.mcp-server** (NOT RECOMMENDED)
Add Rust build stages to Dockerfile.mcp-server. This would make the file confusing since it's named "mcp-server" but also builds the indexer.

### Required Actions Before Proceeding

1. **BLOCK**: Do NOT proceed with any DKRHUB tickets until Dockerfile is fixed
2. **CREATE**: New ticket to create proper combined Dockerfile
3. **UPDATE**: All DKRHUB tickets to reference correct Dockerfile
4. **TEST**: Verify new Dockerfile works locally before GitHub Actions
5. **VALIDATE**: Test image contains both Node.js and Rust components

---

## Critical Issue #2: Missing Build Context Validation 🟡

### Problem Statement

The GitHub Actions workflow will use `packages/maproom-mcp` as build context, but this doesn't include the Rust workspace root needed to build the Rust binary.

### Evidence

DKRHUB-1001 specifies:
```yaml
env:
  BUILD_CONTEXT: packages/maproom-mcp  # ← Missing workspace root!
  DOCKERFILE_PATH: packages/maproom-mcp/config/Dockerfile.mcp-server
```

But Dockerfile.maproom (the correct one currently in use) requires:
```dockerfile
COPY Cargo.toml Cargo.lock ./          # ← Workspace root files
COPY crates/maproom/ ./crates/maproom/ # ← Workspace structure
```

### Impact

Even if we fix the Dockerfile to be combined, the build context is too narrow. We need the workspace root to build Rust.

### Recommended Solution

Update DKRHUB-1001 and related tickets:
```yaml
env:
  BUILD_CONTEXT: .  # Workspace root (repository root)
  DOCKERFILE_PATH: packages/maproom-mcp/config/Dockerfile.combined
```

Or use the monorepo build context pattern:
```yaml
- name: Build and push Docker image
  uses: docker/build-push-action@v5
  with:
    context: .  # Repository root
    file: packages/maproom-mcp/config/Dockerfile.combined
```

---

## Issue #3: Integration Test Impact 🟡

### Problem Statement

The integration test script (`packages/maproom-mcp/tests/startup-integration.sh`) has already been modified to use absolute paths, but it may fail when docker-compose.yml switches from `build:` to `image:` if the image doesn't exist yet.

### Evidence

From conversation summary:
- Fixed test script paths in commit 4673606
- Tests still fail due to Docker build issue
- User rejected disabling maproom-mcp service for tests

### Current Test Flow

1. Test starts docker-compose up
2. docker-compose tries to build from source (currently broken)
3. Tests fail

### Post-DKRHUB Test Flow

1. Test starts docker-compose up
2. docker-compose tries to pull image from Docker Hub
3. If image doesn't exist (pre-publish), tests fail

### Recommended Solution

Create test-specific docker-compose.override.yml:
```yaml
# packages/maproom-mcp/tests/docker-compose.test.yml
services:
  maproom-mcp:
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.combined
    image: maproom-mcp:test  # Tag for local testing
```

Update test script:
```bash
# Use test override that builds locally
docker-compose -f config/docker-compose.yml -f tests/docker-compose.test.yml up -d
```

**Add new ticket**: DKRHUB-2004 - Create test-specific Docker Compose configuration

---

## Issue #4: Development Workflow Breaking Change 🟡

### Problem Statement

DKRHUB-2001 removes `build:` section from docker-compose.yml, breaking local development workflows that expect to build from source.

### Evidence

DKRHUB-2001 acknowledges this:
```
**Backwards Compatibility**:
- This is a breaking change for local development (no longer builds from source)
- Solution: docker-compose.override.yml (created in DKRHUB-2002)
```

### Impact

Developers currently running:
```bash
docker-compose up  # Builds from source
```

Will break after DKRHUB-2001 unless they:
```bash
docker-compose up  # Tries to pull image (may not exist or be outdated)
```

### Recommended Solution

DKRHUB-2002 creates override, but this requires explicit usage:
```bash
docker-compose -f docker-compose.yml -f docker-compose.override.yml up
```

**Better approach**: Update docker-compose.yml to check for override automatically:
```yaml
# In docker-compose.yml - add comment
maproom-mcp:
  # Production: Pulls pre-built image
  # Development: Use docker-compose.override.yml to build from source
  # See: docker-compose.override.yml
  image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}
```

**Add to DKRHUB-4004**: Document development setup in README:
```markdown
## Development

For local development with live rebuilds:
1. Copy `docker-compose.override.example.yml` to `docker-compose.override.yml`
2. Run `docker-compose up` (automatically uses override)
```

---

## Issue #5: Ticket Sequencing Problem 🟡

### Problem Statement

DKRHUB-2003 (Add Dockerfile Metadata Labels) depends on knowing which Dockerfile we're using, but this isn't resolved until Issue #1 is fixed.

### Evidence

DKRHUB-2003 says:
```
**File**: packages/maproom-mcp/config/Dockerfile.mcp-server
```

But we established this is the wrong file.

### Recommended Solution

**Update DKRHUB-2003**:
- Change dependency: DKRHUB-2003 depends on NEW ticket (create combined Dockerfile)
- Update file reference to correct Dockerfile
- Ensure labels are added to the FINAL runtime stage

---

## Issue #6: Missing Pre-Publish Validation Ticket 🟠

### Problem Statement

Phase 3 (Release) jumps directly from testing the workflow (DKRHUB-1901) to updating package version (DKRHUB-3001) without validating that the Docker Hub images actually work end-to-end.

### Recommended Solution

**Add new ticket**: DKRHUB-2904 - Validate Pre-Release Images Locally

```markdown
## Summary
Pull the pre-release images from Docker Hub and validate they work end-to-end before proceeding to v1.1.10 release.

## Acceptance Criteria
- Pull pre-release image: `docker pull crewchief/maproom-mcp:1.1.10-rc1`
- Test image on Linux AMD64
- Test image on macOS ARM64
- Verify Node.js runtime exists
- Verify Rust binary exists
- Verify MCP server starts and responds
- Verify scan functionality works

## Dependencies
- DKRHUB-1901: Pre-release image must be published
- DKRHUB-2001: docker-compose.yml must be updated to use images

## Blocks
- DKRHUB-3001: Don't bump version until pre-release validated
```

---

## Issue #7: Missing Rollback Plan 🟠

### Problem Statement

No tickets address what happens if the Docker Hub publishing fails mid-release or if images are corrupted.

### Recommended Solution

**Add new ticket**: DKRHUB-3006 - Create Rollback Procedure

```markdown
## Summary
Document and test rollback procedure if v1.1.10 release fails.

## Acceptance Criteria
- Document how to unpublish npm package (npm unpublish @crewchief/maproom-mcp@1.1.10)
- Document how to delete Docker Hub tags
- Test rollback to v1.1.9
- Create rollback checklist
- Update release process with rollback steps

## Scenarios
1. GitHub Actions workflow fails mid-push
2. Docker Hub images are corrupted
3. npm publish succeeds but images don't work
4. Multi-platform build fails for one architecture
```

---

## Issue #8: Platform Binary Distribution Not Addressed 🟠

### Problem Statement

The current maproom-mcp package includes pre-built platform-specific binaries in `packages/cli/bin/<platform>/crewchief-maproom`. After switching to Docker Hub, these binaries may no longer be needed for the npm package, but no ticket addresses removing or updating them.

### Evidence

From package.json:
```json
"files": [
  "bin/cli.cjs",
  "config/docker-compose.yml",
  "config/Dockerfile.mcp-server",  // Note: Also wrong Dockerfile!
  "config/init.sql",
  "dist/",
  "src/",
  // No mention of platform binaries
]
```

### Questions to Resolve

1. Does the npm package still need platform binaries?
2. Should we remove binaries to reduce package size?
3. Do any CLI commands use these binaries directly?

### Recommended Solution

**Add to DKRHUB-3001**:
- Audit npm package contents
- Remove unused platform binaries if Docker images provide Rust binary
- Update package.json files array
- Measure package size reduction

---

## Issue #9: Missing ARM64 Testing in Phase 1 🟠

### Problem Statement

DKRHUB-1901 (Test Workflow with Pre-Release Tag) doesn't explicitly require testing on ARM64 (Apple Silicon), yet Phase 4 tickets (DKRHUB-4002) are dedicated to macOS ARM64 testing.

### Risk

We might publish multi-platform images without verifying ARM64 builds work, only to discover issues during Phase 4 validation.

### Recommended Solution

**Update DKRHUB-1901 acceptance criteria**:
```markdown
## Additional Acceptance Criteria
- [ ] ARM64 image builds successfully (verify in GitHub Actions logs)
- [ ] AMD64 image builds successfully (verify in GitHub Actions logs)
- [ ] Docker manifest includes both platforms: `docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Pull and test on macOS ARM64: `docker pull --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Pull and test on Linux AMD64: `docker pull --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1`
```

Move DKRHUB-4002 earlier or merge with DKRHUB-1901.

---

## Recommendations Summary

### MUST FIX (Blocking Release)

1. **🔴 CRITICAL**: Create proper combined Dockerfile (NEW TICKET REQUIRED)
   - Create `Dockerfile.combined` with both Rust and Node.js
   - Update all tickets to reference correct Dockerfile
   - Test locally before any GitHub Actions work
   - **Estimate**: 4-6 hours

2. **🔴 CRITICAL**: Update BUILD_CONTEXT in DKRHUB-1001
   - Change from `packages/maproom-mcp` to `.` (workspace root)
   - **Estimate**: 15 minutes

### SHOULD FIX (High Priority)

3. **🟡 HIGH**: Create integration test Docker Compose config (NEW TICKET)
   - DKRHUB-2004: Test-specific docker-compose.test.yml
   - Prevents test failures during development
   - **Estimate**: 1 hour

4. **🟡 HIGH**: Add pre-release validation ticket (NEW TICKET)
   - DKRHUB-2904: Validate images work before v1.1.10 release
   - Catch issues before production publish
   - **Estimate**: 2 hours

5. **🟡 HIGH**: Fix DKRHUB-2003 dependencies
   - Update to reference correct Dockerfile
   - Add dependency on Dockerfile creation ticket
   - **Estimate**: 15 minutes

### NICE TO HAVE (Medium Priority)

6. **🟠 MEDIUM**: Add rollback procedure ticket (NEW TICKET)
   - DKRHUB-3006: Document rollback steps
   - Safety net for failed releases
   - **Estimate**: 1.5 hours

7. **🟠 MEDIUM**: Enhance DKRHUB-1901 with ARM64 testing
   - Add explicit ARM64 validation steps
   - Move earlier in pipeline
   - **Estimate**: 30 minutes

8. **🟠 MEDIUM**: Audit npm package contents
   - Add to DKRHUB-3001
   - Remove unused platform binaries
   - **Estimate**: 1 hour

### FUTURE IMPROVEMENTS (Low Priority)

9. **🔵 LOW**: Improve development documentation
   - Add to DKRHUB-4004
   - Document docker-compose.override.yml usage
   - **Estimate**: 30 minutes

---

## Proposed New Ticket: DKRHUB-1000 (Prerequisite)

```markdown
# Ticket: DKRHUB-1000: Create Combined Dockerfile

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create Dockerfile.combined that builds BOTH the Rust indexing binary (crewchief-maproom) AND the Node.js MCP server (index.ts) in a single multi-stage build.

## Background
The MCP server architecture requires both components:
1. Node.js MCP server handles stdio JSON-RPC protocol
2. Rust binary performs scan operations (spawned by Node.js)

Current Dockerfiles are incomplete:
- Dockerfile.maproom: Only Rust binary
- Dockerfile.mcp-server: Only Node.js server

## Acceptance Criteria
- [ ] File created: `packages/maproom-mcp/config/Dockerfile.combined`
- [ ] Stage 1: Builds Rust binary (crewchief-maproom)
- [ ] Stage 2: Builds Node.js MCP server (compiles TypeScript)
- [ ] Stage 3: Runtime image contains BOTH components
- [ ] Runtime image uses node:20-alpine base (for minimal size)
- [ ] Rust binary located at `/usr/local/bin/crewchief-maproom`
- [ ] Node.js app located at `/app/dist/`
- [ ] Image size < 400MB (target)
- [ ] Healthcheck configured
- [ ] Entrypoint: node /app/dist/index.js
- [ ] Test: docker build succeeds
- [ ] Test: docker run shows MCP server starts
- [ ] Test: crewchief-maproom binary is accessible

## Technical Requirements
See "Recommended Solution" in review report.

## Dependencies
BLOCKS: All other DKRHUB tickets

## Estimated Effort
4-6 hours (includes testing and validation)
```

---

## Conclusion

**The DKRHUB project cannot proceed as currently designed.** The fundamental Dockerfile architecture issue must be resolved first.

### Recommended Action Plan

1. **PAUSE**: Stop all DKRHUB implementation work
2. **CREATE**: DKRHUB-1000 ticket for combined Dockerfile
3. **IMPLEMENT**: DKRHUB-1000 and validate locally
4. **UPDATE**: All 22 existing tickets with correct Dockerfile references
5. **CREATE**: Missing tickets (2004, 2904, 3006)
6. **REVIEW**: Updated ticket set before resuming implementation
7. **RESUME**: Begin Phase 1 with correct Dockerfile

### Timeline Impact

- Original DKRHUB timeline: 2-3 days (17.5 hours)
- Additional Dockerfile work: +4-6 hours
- Additional missing tickets: +4.5 hours
- **Revised timeline**: 3-4 days (26 hours total)

### Risk Assessment

**If we proceed without fixes**:
- ❌ GitHub Actions will build broken images
- ❌ Docker Hub will contain non-functional images
- ❌ v1.1.10 will be as broken as v1.1.9
- ❌ Users will experience same deployment failure
- ❌ Additional rollback and fix time required

**If we fix issues first**:
- ✅ Clean implementation with working images
- ✅ Single release cycle to fix v1.1.9 issue
- ✅ Proper testing and validation
- ✅ Minimal user impact

---

**Review Status**: COMPLETE
**Next Steps**: Present findings to user and await decision on how to proceed
