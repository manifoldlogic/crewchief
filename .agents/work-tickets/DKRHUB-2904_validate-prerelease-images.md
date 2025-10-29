# Ticket: DKRHUB-2904: Validate Pre-Release Images

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Pull and validate the pre-release Docker images (v1.1.10-rc1) from Docker Hub before proceeding to production release. This ensures images are functional and multi-platform builds work correctly.

## Background
After DKRHUB-1901 publishes pre-release images to Docker Hub, we must validate them end-to-end before proceeding to v1.1.10 production release. This prevents publishing broken images that would fail for users.

**Gap in Current Plan**:
Phase 3 (Release) jumps directly from workflow testing (DKRHUB-1901) to version bump (DKRHUB-3001) without validating that Docker Hub images actually work.

**This ticket fills the gap**:
- Pull images from Docker Hub
- Test on multiple platforms (AMD64, ARM64)
- Verify both components work (Node.js + Rust)
- Run full MCP workflow
- Catch issues before production release

Reference: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #6"

## Acceptance Criteria

### Image Availability
- [ ] Pre-release tag exists on Docker Hub: `crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Multi-platform manifest exists: `docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1`
- [ ] AMD64 image available: `docker pull --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1`
- [ ] ARM64 image available: `docker pull --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1`

### Component Verification (AMD64)
- [ ] Pull AMD64 image successfully
- [ ] Image size reasonable (< 450MB)
- [ ] Node.js runtime exists: `docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 node --version`
- [ ] Rust binary exists: `docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version`
- [ ] npm dependencies installed: `docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 ls /app/node_modules`

### Component Verification (ARM64)
- [ ] Pull ARM64 image successfully
- [ ] Image size reasonable (< 450MB)
- [ ] Node.js runtime exists: `docker run --rm --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 node --version`
- [ ] Rust binary exists: `docker run --rm --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version`
- [ ] npm dependencies installed: `docker run --rm --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 ls /app/node_modules`

### End-to-End Validation (AMD64)
- [ ] Update docker-compose.yml to use pre-release tag: `MAPROOM_VERSION=1.1.10-rc1`
- [ ] Start full stack: postgres, ollama, maproom-mcp
- [ ] All services healthy
- [ ] MCP server starts: `docker exec -i maproom-mcp node /app/dist/index.js`
- [ ] MCP initialize request succeeds
- [ ] Scan functionality works (Rust binary spawning)
- [ ] Search functionality works (database queries)
- [ ] No errors in logs

### End-to-End Validation (ARM64, if available)
- [ ] Test on macOS ARM64 (Apple Silicon) if available
- [ ] OR use QEMU emulation: `docker run --platform linux/arm64 ...`
- [ ] Verify MCP server starts
- [ ] Verify basic functionality works
- [ ] Note: Performance testing in DKRHUB-4002, this is just validation

### Security Scan Results
- [ ] Check GitHub Security tab for Trivy scan results
- [ ] Review any CRITICAL or HIGH vulnerabilities
- [ ] Ensure no blockers for release (critical vulns in runtime dependencies)

## Technical Requirements

**Validation Script**: `packages/maproom-mcp/tests/validate-prerelease.sh`

```bash
#!/bin/bash
set -e

echo "=== DKRHUB-2904: Pre-Release Image Validation ==="
echo ""

PRERELEASE_TAG="${PRERELEASE_TAG:-1.1.10-rc1}"
IMAGE="crewchief/maproom-mcp:$PRERELEASE_TAG"

echo "Validating: $IMAGE"
echo ""

# ========================================
# Multi-Platform Manifest Check
# ========================================
echo "Step 1: Checking multi-platform manifest..."
docker manifest inspect "$IMAGE" | grep -E "architecture|os" || {
  echo "❌ Manifest not found or incomplete"
  exit 1
}

# ========================================
# AMD64 Validation
# ========================================
echo ""
echo "Step 2: Pulling AMD64 image..."
docker pull --platform linux/amd64 "$IMAGE"

echo ""
echo "Step 3: Validating AMD64 image..."
echo "- Checking Node.js..."
docker run --rm --platform linux/amd64 "$IMAGE" node --version

echo "- Checking Rust binary..."
docker run --rm --platform linux/amd64 "$IMAGE" crewchief-maproom --version

echo "- Checking npm dependencies..."
docker run --rm --platform linux/amd64 "$IMAGE" ls /app/node_modules | wc -l

echo "- Checking image size..."
docker images "$IMAGE" --format "{{.Size}}"

# ========================================
# ARM64 Validation
# ========================================
echo ""
echo "Step 4: Pulling ARM64 image..."
docker pull --platform linux/arm64 "$IMAGE"

echo ""
echo "Step 5: Validating ARM64 image..."
echo "- Checking Node.js..."
docker run --rm --platform linux/arm64 "$IMAGE" node --version

echo "- Checking Rust binary..."
docker run --rm --platform linux/arm64 "$IMAGE" crewchief-maproom --version

echo "- Checking npm dependencies..."
docker run --rm --platform linux/arm64 "$IMAGE" ls /app/node_modules | wc -l

# ========================================
# End-to-End Test
# ========================================
echo ""
echo "Step 6: End-to-end validation with docker-compose..."

# Backup current docker-compose.yml
cd "$(dirname "${BASH_SOURCE[0]}")/../config"
cp docker-compose.yml docker-compose.yml.backup

# Update to use pre-release tag
export MAPROOM_VERSION="$PRERELEASE_TAG"

echo "- Starting services with MAPROOM_VERSION=$MAPROOM_VERSION..."
docker-compose up -d

echo "- Waiting for services to be healthy..."
sleep 45

echo "- Checking service status..."
docker-compose ps

echo "- Testing MCP server..."
timeout 5 docker exec -i maproom-mcp node /app/dist/index.js <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}
EOF

echo ""
echo "- Checking logs for errors..."
docker logs maproom-mcp 2>&1 | tail -20

echo ""
echo "- Cleaning up..."
docker-compose down
mv docker-compose.yml.backup docker-compose.yml

cd -

# ========================================
# Summary
# ========================================
echo ""
echo "✅ Pre-release validation complete!"
echo ""
echo "Images validated:"
echo "  - $IMAGE (AMD64)"
echo "  - $IMAGE (ARM64)"
echo ""
echo "Next steps:"
echo "1. Review GitHub Security tab for Trivy scan results"
echo "2. If all clear, proceed to DKRHUB-3001 (version bump)"
echo "3. If issues found, fix and re-publish pre-release"
```

**Execution**:
```bash
# Run validation script
bash packages/maproom-mcp/tests/validate-prerelease.sh

# Or specify custom tag
PRERELEASE_TAG=1.1.10-rc2 bash packages/maproom-mcp/tests/validate-prerelease.sh
```

**Manual Checks**:
- [ ] Visit Docker Hub: https://hub.docker.com/r/crewchief/maproom-mcp/tags
- [ ] Verify tag `1.1.10-rc1` exists with multi-arch icon
- [ ] Check last updated timestamp
- [ ] Visit GitHub Security tab for scan results
- [ ] Review vulnerability report

## Implementation Notes

**Why Pre-Release Validation Matters**:
1. **Catch Build Issues**: Multi-platform builds might succeed but produce broken binaries
2. **Verify Functionality**: Ensure both Rust and Node.js components work
3. **Platform Compatibility**: ARM64 binaries might have different issues than AMD64
4. **Early Detection**: Find problems before production release
5. **Confidence**: Proceed to v1.1.10 with certainty

**Common Issues to Watch For**:
- **Missing Dependencies**: Runtime libraries not installed (libgcc, libssl3)
- **Binary Corruption**: Cross-compilation issues on ARM64
- **Path Issues**: Binaries not in PATH or wrong location
- **Permission Issues**: Files not executable or wrong ownership
- **Size Issues**: Image unexpectedly large (> 500MB)

**What to Do if Validation Fails**:
1. **Document the failure**: Screenshot/copy error messages
2. **Create bug ticket**: DKRHUB-BUGFIX-xxx with details
3. **Fix Dockerfile.combined**: Update DKRHUB-1000
4. **Re-run DKRHUB-1007**: Local testing with fixes
5. **Re-run DKRHUB-1901**: Publish new pre-release (1.1.10-rc2)
6. **Re-run DKRHUB-2904**: Validate again
7. **Iterate until passing**

**QEMU Emulation for ARM64**:
If macOS ARM64 hardware unavailable, use QEMU:
```bash
# Docker Desktop enables QEMU automatically
docker run --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 node --version

# Note: QEMU is slower but validates architecture compatibility
```

## Dependencies
- DKRHUB-1901: Pre-release images must be published to Docker Hub
- DKRHUB-2001: docker-compose.yml must support MAPROOM_VERSION environment variable

## Blocks
- DKRHUB-3001: Don't bump version to v1.1.10 until pre-release validated
- DKRHUB-3002: Don't create v1.1.10 tag until pre-release validated

## Risk Assessment
- **Risk**: Pre-release images have critical bugs
  - **Mitigation**: This ticket catches them before production release
- **Risk**: Platform-specific issues (ARM64)
  - **Mitigation**: Explicit ARM64 testing required
- **Risk**: Validation passes but production fails
  - **Mitigation**: DKRHUB-4001, 4002 provide additional platform testing

## Files/Packages Affected
- NEW: `packages/maproom-mcp/tests/validate-prerelease.sh`
- TEMPORARY MODIFY: `packages/maproom-mcp/config/docker-compose.yml` (restored after testing)

## Estimated Effort
2-3 hours (includes script creation, execution, troubleshooting, and documentation)

## Related Issues
- Fixes: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #6"
- Prevents: Publishing broken v1.1.10 production images
- Complements: DKRHUB-4001, 4002 (comprehensive platform testing)
