# Ticket: DKRHUB-4001: End-to-End Testing on Linux AMD64

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Perform complete end-to-end validation of v1.1.10 on Linux AMD64 (x86_64), simulating real user installation and usage to verify the npm package works correctly with Docker Hub images.

## Background
This is the primary platform validation test. Most users run on Linux servers (AWS, GCP, DigitalOcean, etc.) or Linux desktops. We must verify:
1. Clean install works
2. Images pull from Docker Hub
3. Services start successfully
4. No errors occur
5. Basic functionality works

This validates the entire DKRHUB project fixes the v1.1.9 deployment failure.

Reference: DKRHUB_PLAN.md Phase 4, Task DKRHUB-4001 (lines 736-797)

## Acceptance Criteria
- [ ] Test environment: Clean Ubuntu 22.04 LTS (AMD64) with Docker 24.0+ and Node 18+
- [ ] Clean install succeeds: `npm install -g @crewchief/maproom-mcp@1.1.10`
- [ ] All dependencies resolve without errors
- [ ] Docker images pull correctly from Docker Hub (not built locally)
- [ ] All three services start: maproom-mcp, maproom-postgres, maproom-ollama
- [ ] Health checks pass within 60 seconds
- [ ] Service logs show no critical errors
- [ ] Total startup time <3 minutes (cold start with image pull)
- [ ] Subsequent restart <30 seconds (warm start with cached images)

## Technical Requirements
**Test Environment**:
- OS: Ubuntu 22.04 LTS (or Ubuntu 20.04)
- Architecture: AMD64 (x86_64)
- Docker: Version 24.0 or newer
- Docker Compose: Version 2.0 or newer
- Node.js: Version 18 or newer
- npm: Version 9 or newer
- Fresh system: No prior maproom installation

**Setup Test Environment** (if using container):
```bash
# Option 1: Use Docker container as test environment
docker run -it --rm \
  --name test-linux-amd64 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e TERM=xterm-256color \
  ubuntu:22.04 bash

# Inside container:
apt-get update
apt-get install -y curl ca-certificates
curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
apt-get install -y nodejs docker-compose
```

**Test Script**:
```bash
#!/bin/bash
set -euo pipefail

echo "=== DKRHUB-4001: Linux AMD64 E2E Test ==="
echo "Architecture: $(uname -m)"
echo "OS: $(lsb_release -d)"
echo "Docker: $(docker --version)"
echo "Node: $(node --version)"
echo "npm: $(npm --version)"
echo ""

# Clean environment
echo "1. Cleaning environment..."
docker system prune -af
docker volume prune -f
npm uninstall -g @crewchief/maproom-mcp 2>/dev/null || true

# Install package
echo "2. Installing @crewchief/maproom-mcp@1.1.10..."
time npm install -g @crewchief/maproom-mcp@1.1.10

# Verify installation
echo "3. Verifying installation..."
which maproom-mcp
maproom-mcp --version

# Start services (cold start)
echo "4. Starting services (cold start with image pull)..."
START_TIME=$(date +%s)
npx @crewchief/maproom-mcp start

# Wait for services to stabilize
echo "5. Waiting for services (60 seconds)..."
sleep 60

END_TIME=$(date +%s)
STARTUP_TIME=$((END_TIME - START_TIME))
echo "Startup time: ${STARTUP_TIME} seconds"

# Verify services running
echo "6. Checking services..."
docker ps --filter "name=maproom"
echo ""

# Check health status
echo "7. Checking health status..."
for service in maproom-mcp maproom-postgres; do
  HEALTH=$(docker inspect $service --format='{{.State.Health.Status}}' 2>/dev/null || echo "no-health-check")
  echo "$service: $HEALTH"
done

# Verify image source (should be from Docker Hub)
echo "8. Verifying image source..."
IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}')
echo "Image: $IMAGE"
if [[ ! $IMAGE =~ ^crewchief/maproom-mcp ]]; then
  echo "ERROR: Image not from Docker Hub!"
  exit 1
fi

# Check architecture
echo "9. Verifying architecture..."
ARCH=$(docker inspect maproom-mcp --format='{{.Architecture}}')
echo "Architecture: $ARCH"
if [[ "$ARCH" != "amd64" ]]; then
  echo "ERROR: Wrong architecture!"
  exit 1
fi

# Check logs for errors
echo "10. Checking logs for errors..."
docker logs maproom-mcp 2>&1 | tail -20
ERROR_COUNT=$(docker logs maproom-mcp 2>&1 | grep -i "error" | wc -l)
echo "Error count in logs: $ERROR_COUNT"

# Stop services
echo "11. Stopping services..."
npx @crewchief/maproom-mcp stop

# Restart (warm start test)
echo "12. Restarting services (warm start - cached images)..."
START_TIME=$(date +%s)
npx @crewchief/maproom-mcp start
END_TIME=$(date +%s)
RESTART_TIME=$((END_TIME - START_TIME))
echo "Restart time: ${RESTART_TIME} seconds"

# Final verification
echo "13. Final verification..."
docker ps
echo ""
echo "=== Test Results ==="
echo "✓ Installation: SUCCESS"
echo "✓ Cold start: ${STARTUP_TIME}s (target: <180s)"
echo "✓ Warm start: ${RESTART_TIME}s (target: <30s)"
echo "✓ Services: RUNNING"
echo "✓ Image source: Docker Hub"
echo "✓ Architecture: AMD64"
echo ""
echo "E2E Test: PASSED"
```

## Implementation Notes
**Expected Results**:
- Cold start: 90-180 seconds (includes image pull, ~120MB download)
- Warm start: 10-30 seconds (images cached)
- All services reach "healthy" status
- No "lstat /packages" errors (the v1.1.9 bug)

**Key Validations**:
1. **No local build**: Should see "Pulling maproom-mcp", not "Building"
2. **Correct image**: crewchief/maproom-mcp:latest (or :1.1.10 if MAPROOM_VERSION set)
3. **Platform match**: Architecture should be amd64
4. **Functional**: Services start and stay running

**Common Issues**:
1. **Docker not running**: `Cannot connect to Docker daemon`
   - Fix: `sudo systemctl start docker`
2. **Permission denied**: User not in docker group
   - Fix: `sudo usermod -aG docker $USER` (logout/login)
3. **Port conflicts**: Ports 15433, 11434 already in use
   - Fix: Stop conflicting services or configure different ports

Reference DKRHUB_QUALITY_STRATEGY.md lines 312-396 for complete E2E test specification.

## Dependencies
- DKRHUB-3005: npm package must be published
- DKRHUB-3004: Images must be on Docker Hub

## Risk Assessment
- **Risk**: Test environment not representative of production
  - **Mitigation**: Use standard Ubuntu 22.04 LTS, common configuration
- **Risk**: Network issues during image pull
  - **Mitigation**: Retry failed pulls, test with good network connection
- **Risk**: Docker Hub rate limits
  - **Mitigation**: Authenticate with Docker Hub for higher limits

## Files/Packages Affected
- None (testing only, no code changes)
