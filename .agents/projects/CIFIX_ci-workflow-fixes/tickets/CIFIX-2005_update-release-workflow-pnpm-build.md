# Ticket: CIFIX-2001: Update release workflow with pnpm build step

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (workflow validation only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Add pnpm setup and build steps to `.github/workflows/publish-maproom-mcp-image.yml` BEFORE the Docker build step. Without this, Docker build will fail immediately because daemon-client dist/ directory won't exist.

## Background
The Dockerfile.combined copies `packages/daemon-client/dist/` (as part of CIFIX-2002 changes), but the release workflow doesn't build workspace packages before running docker build. This creates a hard dependency:

1. daemon-client is a workspace package that must be built via `pnpm build`
2. Building creates the dist/ directory with compiled TypeScript
3. Dockerfile copies this dist/ directory during Docker build
4. If dist/ doesn't exist, Docker build fails with "COPY failed: file not found"

This is a CRITICAL BLOCKER - Docker builds will fail 100% of the time without this fix.

**Priority**: CRITICAL - MUST BE FIRST TICKET IN PHASE 2

This ticket implements the workflow preparation step from the Phase 2 plan, ensuring the build environment is properly configured before Docker builds execute.

## Acceptance Criteria
- [ ] Node.js setup step added to workflow (node-version: '20')
- [ ] pnpm setup step added (uses pnpm/action-setup@v4 with auto-detection)
- [ ] `pnpm install --frozen-lockfile` step added
- [ ] `pnpm build` step added BEFORE Docker build step
- [ ] All steps inserted after "Checkout code" and before "Build and push Docker image"
- [ ] Workflow YAML validates (yamllint passes)
- [ ] daemon-client dist/ will exist when Docker build starts

## Technical Requirements
- **File**: `.github/workflows/publish-maproom-mcp-image.yml`
- **Location**: After "Checkout code" step (approximately line 36)
- **Insert before**: "Build and push Docker image" step
- **Node version**: 20 (matches project requirements)
- **pnpm setup**: Use pnpm/action-setup@v4 with auto-detection from packageManager field
- **Install flags**: --frozen-lockfile (ensures reproducible builds)

## Implementation Notes

Add these steps in sequence after checkout:

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

**Why This is Critical**:
The Docker build WILL FAIL without this because:
- Dockerfile line (post-CIFIX-2002): `COPY packages/daemon-client/dist ./packages/daemon-client/dist/`
- In CI, fresh checkout has no dist/ directory
- Only `pnpm build` creates daemon-client dist/
- Docker COPY fails immediately if source doesn't exist

**Step Order**:
1. Checkout code (existing)
2. **Setup Node.js** (new)
3. **Setup pnpm** (new)
4. **Install dependencies** (new)
5. **Build packages** (new)
6. Build and push Docker image (existing)

## Dependencies
- **None** - This MUST be done FIRST in Phase 2
- **Blocks**: CIFIX-2002, CIFIX-2003, CIFIX-2004 (Docker build will fail without this)

## Risk Assessment
- **Risk**: pnpm build failure in CI environment
  - **Mitigation**: Use --frozen-lockfile to ensure reproducible builds; pnpm/action-setup@v4 is stable and widely used
- **Risk**: Step order incorrect (build after Docker build)
  - **Mitigation**: Verify step placement with grep commands in validation
- **Risk**: Node version mismatch
  - **Mitigation**: Use node-version: '20' which matches project requirements

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml`

## Validation Commands

```bash
# Verify workflow syntax
yamllint .github/workflows/publish-maproom-mcp-image.yml

# Verify packageManager field exists (for pnpm auto-detection)
jq -r '.packageManager' /workspace/package.json
# Expected: pnpm@10.12.1+sha512...

# After implementation, verify step order:
grep -n "name: Setup Node.js" .github/workflows/publish-maproom-mcp-image.yml
grep -n "name: Build packages" .github/workflows/publish-maproom-mcp-image.yml
grep -n "name: Build and push" .github/workflows/publish-maproom-mcp-image.yml
# Build packages line number must be LESS than Build and push line number
```

## Planning References
- Phase 2: Docker Build Fix (CIFIX project plan)
- Addresses daemon-client dist/ dependency for Docker builds
