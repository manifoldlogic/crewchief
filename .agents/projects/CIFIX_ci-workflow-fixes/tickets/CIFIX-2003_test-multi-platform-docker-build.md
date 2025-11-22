# Ticket: CIFIX-2003: Test multi-platform Docker build

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - validation commands executed successfully
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Validate the updated Dockerfile by building locally (amd64) with comprehensive pre-flight checks for daemon-client dist/ and pnpm version sync.

## Background
After modifying the Dockerfile to use pnpm and workspace dependencies (CIFIX-2001, CIFIX-2002), we must validate the changes work correctly before relying on them in CI/CD.

This ticket implements critical validation steps to ensure:
1. daemon-client dist/ exists before attempting build (prevents EUNSUPPORTEDPROTOCOL errors)
2. pnpm versions match between package.json and Dockerfile (prevents version drift)
3. Docker build completes without errors
4. Image size remains acceptable (~220MB)
5. Final image doesn't contain build tools (pnpm, etc.)
6. Container starts successfully

This ticket focuses on LOCAL validation only. CI multi-platform testing happens in the release workflow (CIFIX-2005).

## Acceptance Criteria
- [ ] daemon-client dist/ validated (exists with expected files: index.js, index.d.ts, client.js, client.d.ts)
- [ ] pnpm version sync validated (package.json matches Dockerfile)
- [ ] Local Docker build completes successfully (linux/amd64)
- [ ] Image size is approximately 220MB (±10MB tolerance)
- [ ] Container starts without errors
- [ ] pnpm not present in final runtime image
- [ ] MCP server dist/index.js exists in final image
- [ ] All validation commands pass without errors

## Technical Requirements
- **Platform**: linux/amd64 (local testing for speed)
- **Build context**: Repository root (`/workspace`)
- **Dockerfile**: `packages/maproom-mcp/config/Dockerfile.combined`
- **Tag**: `maproom-mcp:cifix-test`
- **Pre-requisite**: `pnpm build` must be run to generate daemon-client dist/

## Implementation Notes

### CRITICAL Pre-flight Validation
Must pass before attempting Docker build:

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

### Build and Test Commands

```bash
# Ensure daemon-client is built
pnpm build

# Build Docker image (amd64 only for speed)
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
# Should show file with proper permissions

echo "✅ Local Docker build validated"
```

### Expected Build Time
- amd64: ~5 minutes (with warm cache)
- First build: ~8-10 minutes (cold cache)

### Validation Success Criteria
- No "EUNSUPPORTEDPROTOCOL" errors
- No "daemon-client not found" errors
- No "workspace: not resolved" errors
- Build completes without errors
- All smoke tests pass

## Dependencies
- **Requires**:
  - CIFIX-2001 (pnpm in Dockerfile)
  - CIFIX-2002 (workspace configuration)
  - CIFIX-2005 (workflow builds daemon-client before Docker)
- **Blocks**: CIFIX-2004 (documentation of validated approach)

## Risk Assessment
- **Risk**: Local testing only, no CI/production impact
  - **Mitigation**: This is intentional - validate locally before CI changes
- **Risk**: Build fails due to daemon-client dist/ missing
  - **Mitigation**: Pre-flight validation script catches this immediately
- **Risk**: Image size bloats beyond acceptable limits
  - **Mitigation**: Check .dockerignore excludes node_modules and other build artifacts
- **Risk**: Failure requires returning to CIFIX-2002
  - **Mitigation**: Clear error messages guide debugging; all changes are reversible

## Files/Packages Affected
### Read Only (validation):
- `packages/maproom-mcp/config/Dockerfile.combined`
- `packages/daemon-client/dist/index.js`
- `packages/daemon-client/dist/index.d.ts`
- `packages/daemon-client/dist/client.js`
- `packages/daemon-client/dist/client.d.ts`
- `package.json` (for pnpm version check)

### No modifications required - this is a validation-only ticket
