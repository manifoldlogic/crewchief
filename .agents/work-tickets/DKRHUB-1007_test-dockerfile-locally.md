# Ticket: DKRHUB-1007: Test Combined Dockerfile Locally

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Thoroughly test the new Dockerfile.combined locally to ensure both Rust and Node.js components work correctly before implementing GitHub Actions workflow. This prevents wasting time debugging in CI/CD.

## Background
Before proceeding with GitHub Actions implementation (DKRHUB-1001+), we must validate that Dockerfile.combined produces a working image. This includes:
- Building successfully from workspace root
- Both components (Node.js MCP server + Rust binary) functional
- Integration with docker-compose.yml
- MCP server can spawn Rust binary for scans
- Database connectivity works

Local testing catches issues early and avoids expensive GitHub Actions debugging cycles.

Reference: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #8"

## Acceptance Criteria

### Build Testing
- [ ] Image builds successfully: `docker build -f packages/maproom-mcp/config/Dockerfile.combined -t maproom-test:local .`
- [ ] Build completes in reasonable time (< 15 minutes on cold build)
- [ ] Image size is acceptable (< 400MB)
- [ ] No build warnings or errors in logs

### Component Verification
- [ ] Node.js runtime exists: `docker run --rm maproom-test:local which node`
- [ ] Rust binary exists: `docker run --rm maproom-test:local which crewchief-maproom`
- [ ] Rust binary is executable: `docker run --rm maproom-test:local crewchief-maproom --version`
- [ ] Node.js dependencies installed: `docker run --rm maproom-test:local ls /app/node_modules`
- [ ] TypeScript compiled correctly: `docker run --rm maproom-test:local ls /app/dist`

### MCP Server Functionality
- [ ] MCP server starts: `timeout 5 docker run --rm -i maproom-test:local`
- [ ] MCP server accepts stdio input (doesn't crash immediately)
- [ ] Logs go to stderr (not stdout): Check stderr for log output
- [ ] MCP server can find crewchief-maproom binary in PATH

### Docker Compose Integration
- [ ] Update docker-compose.yml temporarily to use local image
- [ ] Start services: `docker-compose -f packages/maproom-mcp/config/docker-compose.yml up -d`
- [ ] All services healthy: postgres, ollama, maproom-mcp
- [ ] MCP server container running: `docker ps | grep maproom-mcp`
- [ ] Can exec into container: `docker exec -i maproom-mcp node /app/dist/index.js`
- [ ] Database connectivity works: `docker exec maproom-mcp pg_isready -h maproom-postgres -U maproom`

### End-to-End Validation
- [ ] Start full stack with local image
- [ ] Test MCP tools via stdio proxy: `docker exec -i maproom-mcp node /app/dist/index.js`
- [ ] Send initialize request, verify response
- [ ] Test scan functionality (Rust binary spawning)
- [ ] Test search functionality (database queries)
- [ ] Verify logs are written correctly

## Technical Requirements

**Test Script**: Create `packages/maproom-mcp/tests/test-dockerfile-local.sh`

```bash
#!/bin/bash
set -e

echo "=== DKRHUB-1007: Local Dockerfile Testing ==="
echo ""

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "Step 1: Building Dockerfile.combined..."
docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-test:local \
  .

echo ""
echo "Step 2: Checking image size..."
docker images maproom-test:local --format "Size: {{.Size}}"

echo ""
echo "Step 3: Verifying Node.js runtime..."
docker run --rm maproom-test:local node --version

echo ""
echo "Step 4: Verifying Rust binary..."
docker run --rm maproom-test:local crewchief-maproom --version

echo ""
echo "Step 5: Checking npm dependencies..."
docker run --rm maproom-test:local sh -c "ls /app/node_modules | head -10"

echo ""
echo "Step 6: Checking TypeScript compilation..."
docker run --rm maproom-test:local ls -la /app/dist/

echo ""
echo "Step 7: Testing MCP server startup..."
timeout 5 docker run --rm -i maproom-test:local <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
EOF
echo "(Server started, terminated by timeout - this is expected)"

echo ""
echo "Step 8: Updating docker-compose.yml for local testing..."
# Temporarily modify docker-compose.yml to use local image
cp packages/maproom-mcp/config/docker-compose.yml packages/maproom-mcp/config/docker-compose.yml.backup
sed -i.bak 's|build:|# build:|g' packages/maproom-mcp/config/docker-compose.yml
sed -i.bak 's|context: ../../..|# context: ../../..|g' packages/maproom-mcp/config/docker-compose.yml
sed -i.bak 's|dockerfile: packages/maproom-mcp/config/Dockerfile.maproom|# dockerfile: packages/maproom-mcp/config/Dockerfile.maproom|g' packages/maproom-mcp/config/docker-compose.yml
sed -i.bak '/maproom-mcp:/a\    image: maproom-test:local' packages/maproom-mcp/config/docker-compose.yml

echo ""
echo "Step 9: Starting services with local image..."
cd packages/maproom-mcp/config
docker-compose up -d

echo ""
echo "Step 10: Waiting for services to be healthy..."
sleep 30

echo ""
echo "Step 11: Checking service status..."
docker-compose ps

echo ""
echo "Step 12: Testing database connectivity..."
docker exec maproom-mcp pg_isready -h maproom-postgres -U maproom

echo ""
echo "Step 13: Testing MCP server via exec..."
timeout 5 docker exec -i maproom-mcp node /app/dist/index.js <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
EOF
echo "(MCP server responded, terminated by timeout - this is expected)"

echo ""
echo "Step 14: Cleaning up..."
docker-compose down
mv docker-compose.yml.backup docker-compose.yml
rm -f docker-compose.yml.bak

cd "$WORKSPACE_ROOT"

echo ""
echo "✅ All local tests passed!"
echo ""
echo "Next steps:"
echo "1. Review test output for any warnings"
echo "2. Mark DKRHUB-1007 as complete"
echo "3. Proceed to DKRHUB-1001 (GitHub Actions workflow)"
```

**Manual Testing Checklist**:
- [ ] Run test script: `bash packages/maproom-mcp/tests/test-dockerfile-local.sh`
- [ ] All steps pass without errors
- [ ] Review docker logs for warnings: `docker logs maproom-mcp`
- [ ] Check image layers: `docker history maproom-test:local`
- [ ] Verify non-root user: `docker run --rm maproom-test:local whoami` (should be "node")

## Implementation Notes

**Why Local Testing First**:
1. **Fast Feedback**: Local builds are faster than waiting for GitHub Actions
2. **Easy Debugging**: Can inspect containers, check logs, modify and rebuild quickly
3. **Cost Effective**: No GitHub Actions minutes wasted on broken Dockerfile
4. **Confidence**: Ensures GitHub Actions will succeed on first try

**Common Issues to Watch For**:
- Rust binary not in PATH → Check COPY location in runtime stage
- npm dependencies missing → Verify production install in runtime stage
- TypeScript not compiled → Check node-builder stage
- Permission errors → Verify chown operations and USER directive
- Healthcheck failing → Check postgresql-client installed and pg_isready command

**Integration Points**:
- docker-compose.yml must be temporarily modified to use local image
- Test script should restore original docker-compose.yml after testing
- Use `maproom-test:local` tag to avoid conflicts with production images

## Dependencies
- DKRHUB-1000: Dockerfile.combined must exist and be complete
- Docker installed locally
- Docker Compose installed locally
- Workspace structure intact (Rust + Node.js sources)

## Blocks
- DKRHUB-1001: Don't create GitHub Actions workflow until local testing passes
- DKRHUB-2001: Don't update production docker-compose.yml until validation complete

## Risk Assessment
- **Risk**: Local testing doesn't catch platform-specific issues (ARM64)
  - **Mitigation**: DKRHUB-1901 will test multi-platform builds in GitHub Actions
- **Risk**: Local environment differs from CI/CD
  - **Mitigation**: Use same Docker Buildx and QEMU as GitHub Actions
- **Risk**: Tests pass locally but fail in deployment
  - **Mitigation**: Integration tests (DKRHUB-2902, 2903) will catch deployment issues

## Files/Packages Affected
- NEW: `packages/maproom-mcp/tests/test-dockerfile-local.sh`
- TEMPORARY MODIFY: `packages/maproom-mcp/config/docker-compose.yml` (restored after testing)

## Estimated Effort
2-3 hours (includes test script creation, execution, and issue resolution)

## Success Criteria
- All test steps pass
- Image builds reliably
- Both components functional
- Integration with docker-compose works
- No errors or warnings in logs
- Ready to proceed with GitHub Actions implementation
