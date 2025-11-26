# Ticket: CIFIX-2001: Add pnpm to Docker builder stage

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (infrastructure change, validated via build)
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Install pnpm@10.12.1 in the Docker builder stage (Stage 2: node-builder) to enable workspace dependency resolution for the maproom-mcp package build.

## Background
The current Dockerfile uses `npm install` which doesn't understand pnpm's `workspace:` protocol. This causes the build to fail with "EUNSUPPORTEDPROTOCOL" when it encounters `@maproom/daemon-client: workspace:*` in maproom-mcp's package.json.

By installing pnpm in the builder stage, we can use `pnpm install --filter` which natively understands workspace dependencies. The pnpm binary is only present in the builder stage and discarded in the final runtime image due to multi-stage build isolation.

This ticket implements Phase 2 (Docker Build Fix) of the CIFIX project plan, specifically addressing the missing pnpm installation required for workspace protocol support.

## Acceptance Criteria
- [x] pnpm installation line added to Dockerfile after `FROM node:20-alpine AS node-builder`
- [x] Version matches package.json packageManager field (10.12.1)
- [x] Installation placed BEFORE `apk add` command (npm is needed for pnpm install)
- [x] Exact command used: `RUN npm install -g pnpm@10.12.1`
- [x] Verification: grep command successfully finds the pnpm install line in Dockerfile

## Technical Requirements
- **File**: `packages/maproom-mcp/config/Dockerfile.combined`
- **Stage**: Stage 2 (Node.js builder)
- **Insertion point**: After line 38 (`FROM node:20-alpine AS node-builder`)
- **Position**: Before line 42 (apk add command)
- **Version**: Must match packageManager field in root package.json (currently 10.12.1)
- **Global installation**: Required so pnpm is available in PATH for subsequent RUN commands

## Implementation Notes

### Exact insertion location:
```dockerfile
# Line 38: FROM node:20-alpine AS node-builder
# Line 39: (blank)
# INSERT HERE:
# Install pnpm matching packageManager version
RUN npm install -g pnpm@10.12.1
# Line 40: (blank)
# Line 41: # Install Node.js build dependencies
# Line 42: RUN apk add --no-cache \
```

### Why pnpm@10.12.1 specifically:
- Matches packageManager field in root package.json
- Ensures consistency between local dev, CI, and Docker builds
- Must be manually synchronized when updating pnpm version in package.json

### Why npm install -g:
- Base node:20-alpine image includes npm by default
- npm is required to install pnpm globally
- After pnpm is installed, subsequent commands will use pnpm for workspace operations

### Validation commands:
```bash
# Verify pnpm installation line exists
grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined
# Expected output: RUN npm install -g pnpm@10.12.1

# Verify version matches package.json
PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')
[ "$PACKAGE_PNPM" = "$DOCKERFILE_PNPM" ] && echo "✅ Versions match" || echo "❌ Version mismatch"
```

## Dependencies
- **Requires**: CIFIX-2005 (release workflow must build daemon-client dist/ before Docker build)
- **Blocks**: CIFIX-2002 (pnpm is needed for workspace install command)

## Risk Assessment
- **Risk**: Image size increase in builder stage
  - **Mitigation**: pnpm adds ~20MB to builder stage but is discarded in final multi-stage build (runtime image unaffected)

- **Risk**: Version drift between package.json and Dockerfile
  - **Mitigation**: Validation script checks version consistency; document requirement to sync versions when updating pnpm

- **Risk**: npm not available in base image
  - **Mitigation**: node:20-alpine includes npm by default; no additional dependencies needed

## Files/Packages Affected
- `packages/maproom-mcp/config/Dockerfile.combined` - Add pnpm installation to Stage 2 (node-builder)
