# CIFIX: CI Workflow Fixes

**Status**: ⏳ Planning Complete → Ready for Tickets
**Slug**: CIFIX
**Timeline**: 2 hours implementation + validation
**Complexity**: Low (configuration changes only)

## Problem Statement

CrewChief has **two critical CI failures** blocking development:

1. **Test workflow fails** with pnpm version conflict error
   - Impact: Every push to main fails CI
   - Cause: Explicit `version: 10` in workflow conflicts with `packageManager` in package.json

2. **Docker build fails** with workspace dependency error
   - Impact: Cannot publish releases to Docker Hub
   - Cause: `npm install` doesn't understand pnpm's `workspace:` protocol

Both are recent regressions from:
- Adding `packageManager` field to package.json (good practice)
- Extracting daemon-client as workspace package (good architecture)
- Not updating CI/Docker configs to match (oversight)

## Proposed Solution

### Fix 1: Test Workflow (30 minutes)
**Change**: Remove explicit pnpm version from `.github/workflows/test.yml`

```diff
- name: Setup pnpm
  uses: pnpm/action-setup@v4
-  with:
-    version: 10
+  # Auto-detects from package.json packageManager field
```

**Result**: Single source of truth (package.json), no manual sync needed

---

### Fix 2: Docker Build (60 minutes)
**Change**: Use pnpm in Docker builder, add workspace context

```dockerfile
# Install pnpm to understand workspace: protocol
RUN npm install -g pnpm@10.12.1

# Copy workspace configuration
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/maproom-mcp/package.json ./packages/maproom-mcp/
COPY packages/daemon-client/package.json ./packages/daemon-client/
COPY packages/daemon-client/dist ./packages/daemon-client/dist/

# Install with workspace resolution
RUN pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...
```

**Result**: workspace dependencies resolve correctly, matches dev environment

## Key Design Decisions

### Why remove explicit pnpm version?
- **Single source of truth**: `packageManager` field is industry standard
- **Zero maintenance**: Updating pnpm = change one file, not two
- **Official recommendation**: pnpm/action-setup@v4 designed for auto-detection

### Why pnpm in Docker? (vs alternatives)
- **Consistency**: Local dev (pnpm) = CI (pnpm) = Docker (pnpm)
- **Correctness**: Only pnpm understands `workspace:` protocol natively
- **Future-proof**: Works for any number of workspace dependencies
- **Minimal overhead**: 50MB in builder stage (discarded in final image)

**Alternatives considered**:
- ❌ **Full monorepo copy**: Invalidates cache on any workspace change
- ❌ **Pre-bundle dependencies**: Complex, requires build orchestration
- ✅ **pnpm + --filter**: Clean, standard, well-documented

## Implementation Phases

### Phase 1: Test Workflow Fix
**Tickets**: 2
- CIFIX-1001: Remove pnpm version from test.yml
- CIFIX-1002: Update workflow documentation

**Validation**: Watch next CI run succeed

---

### Phase 2: Docker Build Fix
**Tickets**: 5
- CIFIX-2005: ⚠️ **Update release workflow with pnpm build** (CRITICAL - DO FIRST)
- CIFIX-2001: Add pnpm to Docker builder
- CIFIX-2002: Update Dockerfile for workspace deps
- CIFIX-2003: Test multi-platform build
- CIFIX-2004: Update Docker docs

**Validation**: Local build + CI multi-platform build

---

### Phase 3: Documentation
**Tickets**: 3
- CIFIX-3001: Update project documentation
- CIFIX-3002: Add troubleshooting guides
- CIFIX-3003: Set up monitoring (optional)

**Validation**: Team review, no questions

## Relevant Agents

### Primary Implementation
- **github-actions-specialist**: Phase 1 (test workflow fix)
- **docker-engineer**: Phase 2 (Docker build fix)

### Supporting Roles
- **General agent**: Phase 3 (documentation)
- **verify-ticket**: Validation for all phases
- **commit-ticket**: Create commits for each ticket

## Success Metrics

**Immediate** (Day 1):
- ✅ CI runs pass on main branch
- ✅ Docker images publish successfully
- ✅ Zero "pnpm version" errors
- ✅ Zero "workspace:" errors

**Short-term** (Week 1):
- ✅ 100% CI success rate (excluding code bugs)
- ✅ 2-3 successful releases
- ✅ No manual interventions
- ✅ Developers unblocked

**Long-term** (Month 1):
- ✅ Stable image size (~220MB)
- ✅ Build time within 10% of baseline
- ✅ Zero regressions
- ✅ pnpm versions stay in sync

## Risk Assessment

**Overall Risk**: Low

**Why low risk?**
- Configuration changes only (no application code)
- Fast rollback (revert commit)
- Offline validation possible (local Docker build)
- Well-understood tools (pnpm, Docker, GitHub Actions)

**Key risks mitigated**:
- ✅ Version drift: Documented in PR template
- ✅ Image bloat: pnpm only in builder stage
- ✅ Build failures: Pre-validated locally before CI

## Security Review

**Impact**: Improved security posture

**Improvements**:
- ✅ Better dependency pinning (`--frozen-lockfile` vs npm)
- ✅ Explicit version control (pnpm@10.12.1, not floating)
- ✅ Workspace resolution (no package.json rewriting)

**Accepted risks**:
- ⚠️ Manual pnpm version sync (Dockerfile vs package.json)
- ⚠️ npm registry trust (industry standard)

**Verdict**: Approve for implementation

## Documentation

All planning documents in `planning/` directory:

### Strategic Planning
- **[analysis.md](planning/analysis.md)**: Problem definition, root cause analysis, industry solutions
- **[architecture.md](planning/architecture.md)**: Solution design, technology choices, implementation details
- **[plan.md](planning/plan.md)**: Phase-by-phase execution plan, tickets, validation steps

### Quality & Security
- **[quality-strategy.md](planning/quality-strategy.md)**: Testing approach, validation checklist, risk mitigation
- **[security-review.md](planning/security-review.md)**: Threat model, attack surface analysis, security recommendations

## Next Steps

1. **Review planning docs** - Ensure team alignment
2. **Create tickets** - Run `/create-project-tickets CIFIX`
3. **Execute phases** - Run `/work-on-project CIFIX`
4. **Validate** - Watch CI, test Docker builds
5. **Archive** - Move to archive after completion

## Quick Reference

**Files to modify**:
- `.github/workflows/test.yml` (remove 3 lines)
- `packages/maproom-mcp/config/Dockerfile.combined` (~20 lines)
- `.github/CLAUDE.md` (add troubleshooting)
- `packages/maproom-mcp/CLAUDE.md` (document prereqs)

**Prerequisites**:
- ✅ package.json has `packageManager` field
- ✅ pnpm-workspace.yaml exists
- ⚠️ **CRITICAL**: Run `pnpm build` before `docker build` (creates daemon-client dist/)
- ⚠️ **CRITICAL**: Release workflow MUST run `pnpm build` before Docker build (CIFIX-2005)

**Validation commands**:
```bash
# Test workflow: Check packageManager field
jq -r '.packageManager' package.json

# Docker build: Validate prerequisites
pnpm build
ls -la packages/daemon-client/dist/  # Must show index.js, client.js, etc.

# Docker build: Local test
docker build -f packages/maproom-mcp/config/Dockerfile.combined -t test .
docker images test  # Should be ~220MB
docker run --rm test node -e "console.log('OK')"

# Verify pnpm version sync
PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')
[ "$PACKAGE_PNPM" = "$DOCKERFILE_PNPM" ] && echo "✅ pnpm versions match" || echo "❌ Version mismatch"
```

## Timeline

- **Planning**: ✅ Complete
- **Ticket Creation**: ⏳ Next
- **Implementation**: 2 hours
- **Validation**: 30 minutes
- **Total**: Half day

**Target completion**: Same day as ticket creation
