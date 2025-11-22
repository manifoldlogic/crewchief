# Ticket: CIFIX-2002: Update Dockerfile for workspace dependencies

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (Dockerfile configuration only, no test execution required)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Modify Dockerfile.combined Stage 2 to copy workspace configuration files, daemon-client package, and use pnpm for dependency installation with workspace resolution.

## Background
The current Dockerfile copies only maproom-mcp/package.json in isolation and uses npm install, which fails because:
1. npm doesn't understand pnpm's `workspace:` protocol
2. daemon-client package files aren't available for resolution
3. No workspace context (pnpm-workspace.yaml, root package.json)

This ticket implements the complete workspace-aware build strategy, including copying pre-built daemon-client dist/ artifacts. This is part of Phase 2 of the CI Workflow Fixes project to restore Docker build functionality.

Reference: `.agents/projects/CIFIX_ci-workflow-fixes/planning/architecture.md` Section "Precise Dockerfile Implementation" (lines 131-220)

## Acceptance Criteria
- [x] Workspace root configs copied (package.json, pnpm-lock.yaml, pnpm-workspace.yaml)
- [x] Both package manifests copied (maproom-mcp + daemon-client)
- [x] daemon-client dist/ directory copied to correct location
- [x] daemon-client tsconfig.json copied
- [x] `npm install` replaced with `pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...`
- [x] WORKDIR changed to `packages/maproom-mcp` before build
- [x] Build command changed from `npx tsc` to `pnpm build`
- [x] All validation commands pass

## Technical Requirements

**File**: `packages/maproom-mcp/config/Dockerfile.combined`
**Stage**: 2 (Node.js builder)
**Lines to Replace**: 46-59 (entire dependency installation and build section)

### DELETE lines 46-59:
```dockerfile
WORKDIR /build

# Copy package files for dependency caching
COPY packages/maproom-mcp/package.json ./

# Install all dependencies (including devDependencies for TypeScript)
RUN npm install --production=false --no-audit --no-fund

# Copy TypeScript config and source code
COPY packages/maproom-mcp/tsconfig.json ./
COPY packages/maproom-mcp/src/ ./src/

# Compile TypeScript to JavaScript
RUN npx tsc
```

### REPLACE with:
```dockerfile
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

## Implementation Notes

### Key Changes Explained:
1. **Workspace configs**: Enable pnpm to understand monorepo structure
2. **daemon-client package.json**: Required for dependency resolution
3. **daemon-client dist/**: Pre-built artifacts (not rebuilt in Docker)
4. **--filter flag**: Installs only maproom-mcp and its workspace dependencies
5. **WORKDIR change**: Build from package subdirectory (pnpm requirement)
6. **pnpm build**: Uses package.json scripts instead of raw tsc

### Validation Commands:
```bash
# Verify workspace copy commands exist
grep "COPY package.json pnpm-lock.yaml pnpm-workspace.yaml" packages/maproom-mcp/config/Dockerfile.combined

# Verify daemon-client dist copy
grep "COPY packages/daemon-client/dist" packages/maproom-mcp/config/Dockerfile.combined

# Verify pnpm install with filter
grep "pnpm install --frozen-lockfile --filter" packages/maproom-mcp/config/Dockerfile.combined

# Verify WORKDIR change
grep "WORKDIR /build/packages/maproom-mcp" packages/maproom-mcp/config/Dockerfile.combined

# Verify pnpm build command
grep "RUN pnpm build" packages/maproom-mcp/config/Dockerfile.combined
```

## Dependencies
- **Requires**:
  - CIFIX-2001 (pnpm must be installed in Dockerfile Stage 2)
  - CIFIX-2005 (daemon-client dist/ must exist before Docker build)
- **Blocks**:
  - CIFIX-2003 (Docker build testing)

## Risk Assessment
- **Risk**: Complex multi-line change requiring precise editing
  - **Mitigation**: Exact diff provided in architecture.md, validation commands verify each change component

- **Risk**: Breaking existing Docker build if lines are misaligned
  - **Mitigation**: Rollback via git revert restores npm-based build immediately

## Files/Packages Affected
- `packages/maproom-mcp/config/Dockerfile.combined` (Stage 2, lines 46-71)

## Planning References
- `.agents/projects/CIFIX_ci-workflow-fixes/planning/architecture.md` (lines 131-220) - "Precise Dockerfile Implementation"
- `.agents/projects/CIFIX_ci-workflow-fixes/planning/architecture.md` (lines 99-127) - "Proposed Architecture"
