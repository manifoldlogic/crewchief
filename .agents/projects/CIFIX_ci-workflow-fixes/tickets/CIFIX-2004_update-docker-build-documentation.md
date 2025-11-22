# Ticket: CIFIX-2004: Update Docker Build Documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add inline comments to Dockerfile explaining the pnpm workspace strategy and update `packages/maproom-mcp/CLAUDE.md` to document the `pnpm build` prerequisite.

## Background
The Dockerfile now uses a workspace-aware pnpm build strategy (implemented in CIFIX-2002) that requires daemon-client to be pre-built. Future developers need clear documentation explaining:

1. Why pnpm is installed in the builder stage
2. Why workspace configs are copied
3. Why daemon-client dist/ is copied (not rebuilt in Docker)
4. The prerequisite: `pnpm build` must run before `docker build`

Without this documentation, developers may:
- Wonder why the Dockerfile is more complex than typical single-package builds
- Attempt Docker builds without running pnpm build first
- Be confused by "daemon-client/dist not found" errors

This ticket completes Phase 2 (Docker Build Fix) by documenting the new build process.

## Acceptance Criteria
- [ ] Inline comment added to Dockerfile explaining pnpm installation strategy
- [ ] Comment explains why workspace configs are copied
- [ ] Comment notes daemon-client dist/ is pre-built (not rebuilt in Docker)
- [ ] `packages/maproom-mcp/CLAUDE.md` updated with "Docker Build" section
- [ ] Prerequisites clearly stated: must run `pnpm build` before `docker build`
- [ ] Build command documented with correct context and flags
- [ ] Common error scenarios documented (dist/ missing, etc.)

## Technical Requirements

### Files to Modify
1. **`packages/maproom-mcp/config/Dockerfile.combined`**
   - Add inline comments explaining pnpm workspace strategy
   - Document why daemon-client dist/ is pre-built
   - Clarify prerequisite: `pnpm build` before Docker build

2. **`packages/maproom-mcp/CLAUDE.md`**
   - Add new "Docker Build" section
   - Document prerequisites (pnpm build requirement)
   - Include standard build commands
   - Include multi-platform build commands
   - Add troubleshooting section with common errors

### Documentation Content Requirements

**Dockerfile Comments** (add after pnpm installation):
```dockerfile
# Install pnpm matching packageManager version
RUN npm install -g pnpm@10.12.1
# NOTE: pnpm enables workspace dependency resolution for @maproom/daemon-client
# It's installed in builder stage only and discarded in final runtime image
```

**Dockerfile Comments** (add before daemon-client dist copy):
```dockerfile
# Copy daemon-client build artifacts (pre-built via pnpm build)
# NOTE: daemon-client must be built BEFORE docker build
# Run 'pnpm build' at repo root to create dist/ directory
COPY packages/daemon-client/dist ./packages/daemon-client/dist/
```

**CLAUDE.md Section** (add to packages/maproom-mcp/CLAUDE.md):
```markdown
## Docker Build

### Prerequisites

**CRITICAL**: Run `pnpm build` before building Docker image.

The Dockerfile requires pre-built workspace packages:
- daemon-client must be compiled to dist/ directory
- Run `pnpm build` at repository root before Docker build
- Failure to do so will cause "COPY failed: file not found" error

### Build Command

```bash
# From repository root
pnpm build  # Build all workspace packages first

docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:latest \
  .
```

### Multi-Platform Build

```bash
docker buildx build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-mcp:latest \
  --platform linux/amd64,linux/arm64 \
  .
```

### Troubleshooting

**"COPY failed: daemon-client/dist not found"**
- **Cause**: daemon-client not built before Docker build
- **Fix**: Run `pnpm build` at repository root
- **Verify**: `ls -la packages/daemon-client/dist/` should show index.js

**"workspace: protocol not resolved"**
- **Cause**: pnpm not installed or wrong version in Dockerfile
- **Fix**: Verify Dockerfile has `RUN npm install -g pnpm@10.12.1`
- **Check**: Version should match package.json packageManager field

**Image size larger than expected (>230MB)**
- **Cause**: node_modules or pnpm store copied to final image
- **Fix**: Verify .dockerignore excludes node_modules
- **Expected**: Final image ~220MB (pnpm only in builder stage)
```

## Implementation Notes

### Documentation Strategy
1. **Inline Comments**: Add comments directly in Dockerfile at critical points
   - Keep comments concise (2-3 lines max)
   - Explain "why" not just "what"
   - Use "NOTE:" prefix for important context

2. **CLAUDE.md Section**: Comprehensive developer guide
   - Emphasize prerequisite (`pnpm build`) with CRITICAL marker
   - Provide copy-paste ready commands
   - Include troubleshooting for predictable errors
   - Document both single and multi-platform builds

### Validation Commands
```bash
# Verify Dockerfile comments exist
grep -A 2 "NOTE:" packages/maproom-mcp/config/Dockerfile.combined

# Verify CLAUDE.md has Docker Build section
grep "## Docker Build" packages/maproom-mcp/CLAUDE.md

# Verify prerequisite documented
grep "pnpm build" packages/maproom-mcp/CLAUDE.md | grep -i "prerequisite\|critical\|before"
```

### Documentation Principles
- **Clarity**: Explain why the complexity exists (workspace dependencies)
- **Actionable**: Provide exact commands developers can run
- **Defensive**: Document common errors before they happen
- **Concise**: Keep inline comments brief, detailed docs in CLAUDE.md

## Dependencies

### Prerequisites
- **CIFIX-2001**: Add pnpm to Docker builder stage (provides pnpm installation)
- **CIFIX-2002**: Update Dockerfile workspace dependencies (establishes pattern to document)
- **CIFIX-2003**: Test multi-platform Docker build (validates approach works)

### Blocks
- None (documentation is final step in Phase 2)

## Risk Assessment

**Risk**: Documentation becomes stale if build process changes
- **Mitigation**: Document in two places (inline + CLAUDE.md) so changes are obvious
- **Mitigation**: Include validation commands that fail if structure changes

**Risk**: Developers skip reading documentation
- **Mitigation**: Use "CRITICAL" marker for prerequisite
- **Mitigation**: Error messages point to specific troubleshooting sections

**Risk**: None (documentation-only changes, no code modification)

## Files/Packages Affected
- `packages/maproom-mcp/config/Dockerfile.combined` (add inline comments)
- `packages/maproom-mcp/CLAUDE.md` (add Docker Build section)
