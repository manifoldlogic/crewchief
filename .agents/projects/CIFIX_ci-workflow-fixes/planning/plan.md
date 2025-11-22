# Implementation Plan: CI Workflow Fixes

## Project Overview

**Goal**: Fix two critical CI workflow failures blocking development and releases

**Scope**: Minimal configuration changes to GitHub Actions and Docker build
- Test workflow: Remove pnpm version conflict (1 line change)
- Docker build: Add pnpm for workspace dependency resolution (~20 lines)

**Timeline**: 1-2 hours of implementation + validation

**Risk**: Low (configuration-only changes, fast rollback)

## Phase 1: Test Workflow Fix

### Tickets

1. **CIFIX-1001**: Remove explicit pnpm version from test.yml
2. **CIFIX-1002**: Update workflow documentation

### Implementation Steps

**CIFIX-1001** (5-10 minutes):
```yaml
# Edit .github/workflows/test.yml
# Lines 56-59: Change from:
- name: Setup pnpm
  uses: pnpm/action-setup@v4
  with:
    version: 10

# To:
- name: Setup pnpm
  uses: pnpm/action-setup@v4
  # Auto-detects pnpm version from package.json packageManager field
```

**CIFIX-1002** (5 minutes):
- Add explanatory comment in test.yml (see above)
- Update `.github/CLAUDE.md` with troubleshooting guidance
- Document packageManager field as source of truth

### Validation

**Pre-commit**:
- [x] Verify YAML syntax: `yamllint .github/workflows/test.yml`
- [x] Verify packageManager field exists: `jq -r '.packageManager' package.json`

**Post-commit**:
- [ ] Push to feature branch
- [ ] Observe CI run
- [ ] Verify "Setup pnpm" step succeeds
- [ ] Verify pnpm version matches package.json (10.12.1)
- [ ] Verify tests execute

**Success criteria**:
- ✅ No "Multiple versions of pnpm specified" error
- ✅ CI completes without infrastructure failures
- ✅ Workflow runtime within 10% of baseline

### Rollback Plan

If pnpm detection fails:
```bash
git revert <commit-sha>
git push
# Fallback: Add explicit version back temporarily while debugging
```

---

## Phase 2: Docker Build Fix

### Tickets

3. **CIFIX-2005**: ⚠️ **CRITICAL** Update release workflow with pnpm build step (MUST BE FIRST)
4. **CIFIX-2001**: Add pnpm to Docker builder stage
5. **CIFIX-2002**: Update Dockerfile for workspace dependencies
6. **CIFIX-2003**: Test multi-platform Docker build
7. **CIFIX-2004**: Update Docker build documentation

### Implementation Steps

**CIFIX-2005** (10 minutes) - **CRITICAL BLOCKER, DO THIS FIRST**:

Update `.github/workflows/publish-maproom-mcp-image.yml` to run `pnpm build` before Docker build.

**File**: `.github/workflows/publish-maproom-mcp-image.yml`
**Location**: After "Checkout code" step (approximately line 36)

**Add these steps BEFORE the Docker build**:

```yaml
# Setup Node.js
- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: '20'

# Setup pnpm (auto-detects from packageManager field)
- name: Setup pnpm
  uses: pnpm/action-setup@v4

# Install dependencies
- name: Install dependencies
  run: pnpm install --frozen-lockfile

# Build all workspace packages (creates daemon-client dist/)
- name: Build packages
  run: pnpm build
```

**Acceptance Criteria**:
- [ ] pnpm setup step added to workflow
- [ ] `pnpm build` runs before Docker build step
- [ ] Workflow YAML validates (yamllint passes)
- [ ] daemon-client dist/ will exist when Docker build starts

**Validation**:
```bash
# Check workflow syntax
yamllint .github/workflows/publish-maproom-mcp-image.yml

# Verify packageManager field exists
jq -r '.packageManager' package.json
# Expected: pnpm@10.12.1+sha512...
```

**Why This is Critical**:
Docker build WILL FAIL without this step because Dockerfile copies `packages/daemon-client/dist/` which doesn't exist in CI checkout without running `pnpm build` first.

---

**CIFIX-2001** (10 minutes):
Add pnpm installation to Dockerfile.combined Stage 2:

**File**: `packages/maproom-mcp/config/Dockerfile.combined`
**Line**: Insert after line 38 (after `FROM node:20-alpine AS node-builder`)

```dockerfile
# Install pnpm matching packageManager version
RUN npm install -g pnpm@10.12.1
```

**Acceptance Criteria**:
- [ ] pnpm installation line added to Dockerfile
- [ ] Version matches package.json packageManager field (10.12.1)
- [ ] Placed BEFORE apk add command

**Validation**:
```bash
# Verify pnpm version matches
grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined
# Should show: RUN npm install -g pnpm@10.12.1
```

**CIFIX-2002** (20 minutes):
Update Dockerfile for workspace dependencies.

**File**: `packages/maproom-mcp/config/Dockerfile.combined`
**Lines to Replace**: 46-59 (entire dependency installation and build section in Stage 2)

See architecture.md "Precise Dockerfile Implementation" section for exact before/after diff with line numbers.

**Key Changes**:
1. Copy workspace root configs (package.json, pnpm-lock.yaml, pnpm-workspace.yaml)
2. Copy both package manifests (maproom-mcp + daemon-client)
3. Copy daemon-client pre-built dist/ directory
4. Replace `npm install` with `pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...`
5. Change WORKDIR to packages/maproom-mcp before build
6. Replace `npx tsc` with `pnpm build`

**Step-by-step**:

```dockerfile
# DELETE lines 46-59 (old implementation):
# WORKDIR /build
# COPY packages/maproom-mcp/package.json ./
# RUN npm install --production=false --no-audit --no-fund
# COPY packages/maproom-mcp/tsconfig.json ./
# COPY packages/maproom-mcp/src/ ./src/
# RUN npx tsc

# REPLACE with new implementation (lines 49-71):
WORKDIR /build

# Copy workspace configuration
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy package manifests (for dependency caching)
COPY packages/maproom-mcp/package.json ./packages/maproom-mcp/
COPY packages/daemon-client/package.json ./packages/daemon-client/

# Copy daemon-client build artifacts (pre-built via pnpm build)
COPY packages/daemon-client/dist ./packages/daemon-client/dist/
COPY packages/daemon-client/tsconfig.json ./packages/daemon-client/

# Install dependencies with workspace resolution
RUN pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...

# Copy TypeScript config and source code
COPY packages/maproom-mcp/tsconfig.json ./packages/maproom-mcp/
COPY packages/maproom-mcp/src/ ./packages/maproom-mcp/src/

# Change to package directory and build
WORKDIR /build/packages/maproom-mcp
RUN pnpm build
```

**Acceptance Criteria**:
- [ ] Workspace configs copied (package.json, pnpm-lock.yaml, pnpm-workspace.yaml)
- [ ] daemon-client dist/ copied to correct location
- [ ] pnpm install uses --filter flag
- [ ] WORKDIR changed to packages/maproom-mcp before build
- [ ] Build command is `pnpm build` not `npx tsc`

**Validation**:
```bash
# Verify workspace copy commands exist
grep "COPY package.json pnpm-lock.yaml pnpm-workspace.yaml" packages/maproom-mcp/config/Dockerfile.combined

# Verify daemon-client dist copy
grep "COPY packages/daemon-client/dist" packages/maproom-mcp/config/Dockerfile.combined

# Verify pnpm install with filter
grep "pnpm install --frozen-lockfile --filter" packages/maproom-mcp/config/Dockerfile.combined
```

**CIFIX-2003** (Local testing - 25 minutes):

Test multi-platform Docker build with comprehensive validation.

**Prerequisites Validation** (CRITICAL):
```bash
cd /workspace

# ⚠️ BLOCKER: Verify daemon-client dist/ exists
if [ ! -d "packages/daemon-client/dist" ]; then
  echo "❌ ERROR: daemon-client dist/ not found"
  echo "Run 'pnpm build' before testing Docker"
  exit 1
fi

# Verify dist/ has expected files
EXPECTED_FILES=("index.js" "index.d.ts" "client.js" "client.d.ts")
for file in "${EXPECTED_FILES[@]}"; do
  if [ ! -f "packages/daemon-client/dist/$file" ]; then
    echo "❌ ERROR: Missing $file in daemon-client/dist/"
    exit 1
  fi
done

echo "✅ daemon-client dist/ validated"

# Verify pnpm version sync
PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')

if [ "$PACKAGE_PNPM" != "$DOCKERFILE_PNPM" ]; then
  echo "❌ ERROR: pnpm version mismatch"
  echo "package.json: $PACKAGE_PNPM"
  echo "Dockerfile: $DOCKERFILE_PNPM"
  exit 1
fi

echo "✅ pnpm versions synced ($PACKAGE_PNPM)"
```

**Build and Test**:
```bash
# Ensure daemon-client is built (if validation passed, this is redundant but safe)
pnpm build

# Test local Docker build (amd64 only for speed)
docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:cifix-test \
  --platform linux/amd64 \
  .

# Verify image size
docker images maproom-mcp:cifix-test
# Expected: ~220MB (±10MB)

# Smoke test container startup
docker run --rm maproom-mcp:cifix-test node -e "console.log('OK')"
# Expected: OK

# Verify pnpm not in final image
docker run --rm maproom-mcp:cifix-test which pnpm || echo "Good: pnpm not in runtime"
# Expected: "Good: pnpm not in runtime"

# Test MCP server binary exists
docker run --rm maproom-mcp:cifix-test ls -la dist/index.js
# Should show file

echo "✅ Local Docker build validated"
```

**Acceptance Criteria**:
- [ ] daemon-client dist/ validated before build
- [ ] pnpm versions match between package.json and Dockerfile
- [ ] Docker build completes without errors
- [ ] Image size ~220MB (±10MB)
- [ ] Container starts successfully
- [ ] pnpm not present in runtime image
- [ ] MCP server dist/ exists in final image

**CIFIX-2004** (10 minutes):
- Add comment in Dockerfile explaining pnpm strategy
- Update `packages/maproom-mcp/CLAUDE.md` with build requirements
- Note prerequisite: `pnpm build` before `docker build`

### Validation

**Pre-commit**:
- [x] Local Docker build succeeds (amd64)
- [x] Image size acceptable (~220MB)
- [x] Container starts without errors
- [x] pnpm version matches package.json
- [x] Documented build prerequisites

**Post-commit** (CI validation):
- [ ] Create test tag or manual workflow trigger
- [ ] Monitor multi-platform build (amd64 + arm64)
- [ ] Verify both platforms build successfully
- [ ] Pull published image
- [ ] Smoke test: `docker run ... node dist/index.js --help`

**Success criteria**:
- ✅ No "EUNSUPPORTEDPROTOCOL" errors
- ✅ workspace: dependencies resolve correctly
- ✅ Multi-platform build completes (~10 minutes)
- ✅ Image size within 5% of baseline
- ✅ Container functional

### Rollback Plan

If Docker build fails:
```bash
# Revert Dockerfile changes
git revert <commit-sha>
git push

# Manually re-tag previous Docker image as latest
docker tag old-image:version latest
docker push latest

# Update GitHub release to point to rollback image
```

---

## Phase 3: Documentation and Monitoring

### Tickets

7. **CIFIX-3001**: Update project documentation
8. **CIFIX-3002**: Add troubleshooting guides
9. **CIFIX-3003**: Set up monitoring alerts

### Implementation Steps

**CIFIX-3001** (15 minutes):

Update key documentation files:

1. `.github/CLAUDE.md`:
   ```markdown
   ## Troubleshooting Workflows

   ### Test Workflow
   - pnpm version auto-detected from package.json packageManager field
   - To change pnpm version: Update package.json only
   - Do NOT add explicit version in test.yml
   ```

2. `packages/maproom-mcp/CLAUDE.md`:
   ```markdown
   ## Docker Build

   Prerequisites:
   - Run `pnpm build` before building Docker image
   - Ensures daemon-client dist/ exists

   Build command:
   docker build -f packages/maproom-mcp/config/Dockerfile.combined .
   ```

3. `packages/maproom-mcp/config/Dockerfile.combined`:
   Add comments explaining pnpm installation and workspace strategy

**CIFIX-3002** (10 minutes):

Create troubleshooting section in `.github/CLAUDE.md`:

```markdown
## Common CI Issues

### "Multiple versions of pnpm specified"
- Cause: Explicit version in workflow + packageManager in package.json
- Fix: Remove explicit version from workflow YAML
- Prevention: Never add `with: version:` to pnpm/action-setup

### "Unsupported URL Type workspace:"
- Cause: npm doesn't understand pnpm workspace: protocol
- Fix: Use pnpm in Docker build (see Dockerfile.combined)
- Prevention: Maintain pnpm throughout build pipeline

### "daemon-client not found" in Docker
- Cause: daemon-client dist/ not copied or not built
- Fix: Run `pnpm build` before `docker build`
- Prevention: Add to CI workflow (already done in release workflow)
```

**CIFIX-3003** (Optional - Future):

Set up GitHub Actions alerts for:
- Test workflow failures
- Docker build failures
- Unexpected image size changes

(Not blocking for MVP - existing GitHub notifications sufficient)

---

## Agent Assignments

### Phase 1: Test Workflow
**Agent**: github-actions-specialist
- Expertise: GitHub Actions workflows, YAML configuration
- Tasks: CIFIX-1001, CIFIX-1002

### Phase 2: Docker Build
**Agent**: docker-engineer
- Expertise: Dockerfile optimization, multi-stage builds, pnpm/npm
- Tasks: CIFIX-2001, CIFIX-2002, CIFIX-2003, CIFIX-2004

### Phase 3: Documentation
**Agent**: General implementation agent (no specialist needed)
- Tasks: CIFIX-3001, CIFIX-3002

---

## Testing Milestones

### Milestone 1: Test Workflow Fixed
**Exit criteria**:
- [x] PR merged to main
- [ ] Next CI run completes without pnpm errors
- [ ] pnpm version correct in workflow logs
- [ ] Tests execute successfully

### Milestone 2: Docker Build Fixed
**Exit criteria**:
- [x] PR merged to main
- [x] Local Docker build validates
- [ ] Multi-platform CI build succeeds
- [ ] Images published to Docker Hub
- [ ] Smoke test passes

### Milestone 3: Documentation Complete
**Exit criteria**:
- [ ] All CLAUDE.md files updated
- [ ] Troubleshooting guides added
- [ ] Inline code comments added
- [ ] Team aware of changes

---

## Security Checkpoints

### Before Implementation
- [x] Verify pnpm version pinned (not latest)
- [x] Confirm npm registry as source (not arbitrary URLs)
- [x] Check --frozen-lockfile used
- [x] Verify multi-stage isolation preserved

### During Implementation
- [ ] Code review security checklist
- [ ] No secrets in Dockerfile
- [ ] No secrets in workflow logs
- [ ] .dockerignore excludes sensitive files

### After Deployment
- [ ] Scan published image for vulnerabilities
- [ ] Verify non-root user in container
- [ ] Confirm pnpm not in runtime image
- [ ] Validate image provenance attestations

---

## Dependencies and Prerequisites

### Before Starting Phase 1
- ✅ package.json has packageManager field
- ✅ pnpm/action-setup@v4 compatible with auto-detection
- ✅ Test workflow currently failing (expected baseline)

### Before Starting Phase 2
- ✅ daemon-client package extracted (already done)
- ✅ workspace: dependencies in package.json (already done)
- ✅ pnpm-workspace.yaml exists (already done)
- ⚠️ daemon-client built (run `pnpm build` before Docker)

### Before Starting Phase 3
- ✅ Phases 1 and 2 complete
- ✅ CI validations passed
- ✅ Docker images published

---

## Success Metrics

### Immediate (Post-Implementation)
- ✅ Zero "Multiple versions of pnpm" errors
- ✅ Zero "Unsupported URL Type" errors
- ✅ CI passing on main branch
- ✅ Docker images publishing successfully

### Short-term (Week 1)
- ✅ 3-5 successful CI runs without intervention
- ✅ 1-2 successful releases published
- ✅ Zero rollbacks needed
- ✅ Developer onboarding smooth (no confusion)

### Long-term (Month 1)
- ✅ 100% CI success rate (excluding code bugs)
- ✅ Zero manual workflow interventions
- ✅ Image size stable
- ✅ Build time within 10% of baseline

---

## Risk Management

### High-Priority Risks

**Risk 1: Test workflow auto-detection fails**
- **Likelihood**: Low
- **Impact**: High (CI blocked)
- **Mitigation**: Validate packageManager field syntax before merge
- **Rollback**: Revert commit, add explicit version temporarily

**Risk 2: Docker multi-platform build timeout**
- **Likelihood**: Medium (QEMU can be slow)
- **Impact**: Medium (delays release)
- **Mitigation**: Test locally first, be patient (allow 10-15 min)
- **Rollback**: Disable arm64 temporarily, debug separately

**Risk 3: Image size bloat**
- **Likelihood**: Low (pnpm only in builder)
- **Impact**: Low (slightly slower pulls)
- **Mitigation**: Verify size in local build before pushing
- **Rollback**: Revert Dockerfile

### Medium-Priority Risks

**Risk 4: pnpm version drift (package.json vs Dockerfile)**
- **Likelihood**: Medium (manual sync required)
- **Impact**: Low (builds may use slightly different versions)
- **Mitigation**: Document in PR template, code review checklist
- **Future fix**: Automate sync via build script

**Risk 5: Forgotten prerequisite (pnpm build before docker build)**
- **Likelihood**: Medium (new requirement)
- **Impact**: Low (build fails fast with clear error)
- **Mitigation**: Document in CLAUDE.md, add to CI workflow
- **Recovery**: Run `pnpm build`, retry Docker build

---

## Timeline

**Day 1 (Implementation)**:
- Phase 1: 30 minutes (test workflow fix + validation)
- Phase 2: 60 minutes (Docker fix + local testing + CI validation)
- Phase 3: 30 minutes (documentation)

**Total**: 2 hours + CI wait time

**Contingency**: +30 minutes for unexpected issues

**Expected completion**: Same day (assuming no major blockers)

---

## Communication Plan

### Before Starting
- [ ] Notify team: "Fixing CI workflows, expect temporary test failures"
- [ ] Create GitHub issue: Track progress and blockers
- [ ] Link issue in PR descriptions

### During Implementation
- [ ] Update GitHub issue with progress
- [ ] Share CI logs if issues arise
- [ ] Request reviews from maintainers

### After Completion
- [ ] Announce fix in team channel
- [ ] Close GitHub issue with summary
- [ ] Update project README if needed
- [ ] Document lessons learned

---

## Acceptance Criteria

### Project Complete When:
- [x] All 9 tickets closed
- [ ] Test workflow passing consistently
- [ ] Docker builds completing successfully
- [ ] Documentation updated
- [ ] No rollbacks triggered
- [ ] Team trained on new workflow
- [ ] Monitoring in place

### Definition of Done:
- [ ] Code merged to main
- [ ] CI green for 3+ consecutive runs
- [ ] Docker images published and functional
- [ ] Docs reviewed and approved
- [ ] Zero open bugs related to this fix
