# DKRHUB: Docker Hub Publishing - Quality Strategy

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Quality Strategy

## Quality Objectives

The quality strategy ensures that Docker Hub published images are:
1. **Functional** - Images work correctly on all platforms
2. **Reliable** - Consistent behavior across deployments
3. **Secure** - Free from known vulnerabilities
4. **Performant** - Fast startup and minimal resource usage
5. **Maintainable** - Easy to debug and update

## Testing Pyramid

```
                    ┌─────────────────┐
                    │   End-to-End    │
                    │   Integration   │ (10% - Full workflow)
                    │   Testing       │
                    └────────┬────────┘
                             │
                  ┌──────────┴──────────┐
                  │   Container         │
                  │   Integration       │ (30% - Service level)
                  │   Testing           │
                  └─────────┬───────────┘
                            │
              ┌─────────────┴─────────────┐
              │   Image Validation        │
              │   & Unit Testing          │ (60% - Component level)
              │                           │
              └───────────────────────────┘
```

## Level 1: Image Validation Testing

### 1.1 Image Build Validation

**Objective**: Verify images build successfully for all platforms

**Test Cases**:

| Test ID | Description | Platform | Expected Result |
|---------|-------------|----------|-----------------|
| BLD-001 | Build AMD64 image | linux/amd64 | Build succeeds |
| BLD-002 | Build ARM64 image | linux/arm64 | Build succeeds |
| BLD-003 | Verify build cache usage | Both | Cache hit >80% |
| BLD-004 | Build time < 10min (cold) | Both | Pass |
| BLD-005 | Build time < 5min (warm) | Both | Pass |

**Execution**:
```bash
# Test AMD64 build
docker buildx build \
  --platform linux/amd64 \
  --file packages/maproom-mcp/config/Dockerfile.mcp-server \
  --tag test-amd64 \
  packages/maproom-mcp

# Test ARM64 build (with QEMU)
docker buildx build \
  --platform linux/arm64 \
  --file packages/maproom-mcp/config/Dockerfile.mcp-server \
  --tag test-arm64 \
  packages/maproom-mcp

# Test multi-platform build
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --file packages/maproom-mcp/config/Dockerfile.mcp-server \
  --tag test-multi \
  packages/maproom-mcp
```

**Success Criteria**:
- All builds complete without errors
- Build logs show no warnings
- Image size < 500MB (uncompressed)

### 1.2 Image Metadata Validation

**Objective**: Verify correct labels and metadata

**Test Cases**:

| Test ID | Description | Validation |
|---------|-------------|------------|
| META-001 | Version label | Matches git tag |
| META-002 | Commit SHA label | Matches build commit |
| META-003 | Build date label | Valid timestamp |
| META-004 | OCI title label | "Maproom MCP Server" |
| META-005 | OCI vendor label | "CrewChief" |

**Execution**:
```bash
# Inspect image metadata
docker inspect crewchief/maproom-mcp:1.1.10

# Verify labels
docker inspect crewchief/maproom-mcp:1.1.10 \
  --format='{{json .Config.Labels}}' | jq

# Expected output:
{
  "org.opencontainers.image.version": "1.1.10",
  "org.opencontainers.image.revision": "abc123...",
  "org.opencontainers.image.created": "2025-10-29T...",
  "org.opencontainers.image.title": "Maproom MCP Server",
  "org.opencontainers.image.vendor": "CrewChief"
}
```

### 1.3 Image Size Validation

**Objective**: Ensure images are optimally sized

**Test Cases**:

| Test ID | Description | Threshold | Target |
|---------|-------------|-----------|--------|
| SIZE-001 | Uncompressed size | <500MB | ~300MB |
| SIZE-002 | Compressed size | <200MB | ~120MB |
| SIZE-003 | Layer count | <20 | ~10 |

**Execution**:
```bash
# Check image size
docker images crewchief/maproom-mcp:1.1.10

# Analyze layers
docker history crewchief/maproom-mcp:1.1.10

# Get compressed size (download size)
docker manifest inspect crewchief/maproom-mcp:1.1.10 | \
  jq '.manifests[] | {platform, size}'
```

**Success Criteria**:
- Image size within thresholds
- No excessively large layers (>50MB except base)
- Multi-stage build reduces final image size

### 1.4 Security Scan Validation

**Objective**: No critical vulnerabilities in images

**Test Cases**:

| Test ID | Description | Threshold |
|---------|-------------|-----------|
| SEC-001 | Critical vulnerabilities | 0 |
| SEC-002 | High vulnerabilities | <5 |
| SEC-003 | Medium vulnerabilities | <20 |
| SEC-004 | Base image freshness | <30 days |

**Execution**:
```bash
# Run Trivy scan
trivy image crewchief/maproom-mcp:1.1.10 \
  --severity CRITICAL,HIGH \
  --exit-code 1

# Scan with detailed output
trivy image crewchief/maproom-mcp:1.1.10 \
  --format json \
  --output trivy-results.json

# Check base image age
docker inspect node:20-alpine \
  --format='{{.Created}}'
```

**Success Criteria**:
- Zero critical vulnerabilities
- High vulnerabilities have mitigation plan
- Base image less than 30 days old

## Level 2: Container Integration Testing

### 2.1 Container Startup Validation

**Objective**: Verify containers start successfully

**Test Cases**:

| Test ID | Description | Platform | Expected Result |
|---------|-------------|----------|-----------------|
| START-001 | Container starts | AMD64 | Running |
| START-002 | Container starts | ARM64 | Running |
| START-003 | Health check passes | Both | Healthy |
| START-004 | Startup time | Both | <30s |
| START-005 | Logs show no errors | Both | Clean logs |

**Execution**:
```bash
# Pull and run image
docker pull crewchief/maproom-mcp:1.1.10
docker run -d \
  --name test-maproom \
  -e DATABASE_URL=postgresql://maproom:maproom@postgres:5432/maproom \
  -e EMBEDDING_PROVIDER=ollama \
  crewchief/maproom-mcp:1.1.10

# Wait for startup
sleep 10

# Check status
docker ps --filter name=test-maproom

# Check health
docker inspect test-maproom --format='{{.State.Health.Status}}'

# Check logs
docker logs test-maproom

# Cleanup
docker stop test-maproom && docker rm test-maproom
```

**Success Criteria**:
- Container reaches "running" state
- Health check reports "healthy" within 30s
- Logs show successful initialization
- No error messages in logs

### 2.2 Service Communication Validation

**Objective**: Verify container can communicate with dependencies

**Test Cases**:

| Test ID | Description | Expected Result |
|---------|-------------|-----------------|
| COMM-001 | Connect to PostgreSQL | Success |
| COMM-002 | Connect to Ollama | Success |
| COMM-003 | Database schema created | Tables exist |
| COMM-004 | Embedding endpoint reachable | HTTP 200 |

**Execution**:
```bash
# Start full stack with docker-compose
cd packages/maproom-mcp/config
docker-compose up -d

# Wait for services
sleep 30

# Test PostgreSQL connection
docker exec maproom-mcp \
  pg_isready -h maproom-postgres -U maproom -d maproom

# Test Ollama connection
docker exec maproom-mcp \
  wget -qO- http://ollama:11434/api/tags

# Check database schema
docker exec maproom-postgres \
  psql -U maproom -d maproom -c "\dt"

# Cleanup
docker-compose down
```

**Success Criteria**:
- All connection tests succeed
- Database tables created
- Ollama model available
- No connection errors in logs

### 2.3 Resource Usage Validation

**Objective**: Verify resource consumption is within limits

**Test Cases**:

| Test ID | Description | Threshold |
|---------|-------------|-----------|
| RES-001 | Memory usage (idle) | <200MB |
| RES-002 | Memory usage (load) | <1GB |
| RES-003 | CPU usage (idle) | <5% |
| RES-004 | CPU usage (load) | <100% |
| RES-005 | Disk usage | <100MB |

**Execution**:
```bash
# Monitor resource usage
docker stats maproom-mcp --no-stream

# Get detailed stats
docker inspect maproom-mcp \
  --format='Memory: {{.HostConfig.Memory}} CPU: {{.HostConfig.CpuShares}}'

# Check disk usage
docker exec maproom-mcp du -sh /app
```

**Success Criteria**:
- Memory usage within limits
- CPU usage reasonable for workload
- No memory leaks over time
- Disk usage stable

## Level 3: End-to-End Integration Testing

### 3.1 npm Package Installation Test

**Objective**: Verify full installation workflow from npm

**Test Cases**:

| Test ID | Description | Platform | Expected Result |
|---------|-------------|----------|-----------------|
| NPM-001 | Install globally | Linux AMD64 | Success |
| NPM-002 | Install globally | macOS ARM64 | Success |
| NPM-003 | Install locally | Both | Success |
| NPM-004 | npx command works | Both | Success |

**Execution**:
```bash
# Test on clean system (use Docker container)
docker run -it --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  node:20-alpine sh

# Inside container:
npm install -g @crewchief/maproom-mcp@1.1.10
which maproom-mcp
maproom-mcp --version

# Test npx
npx -y @crewchief/maproom-mcp --help
```

**Success Criteria**:
- npm install completes without errors
- Binary is executable
- Help text displays correctly

### 3.2 Full Startup Test

**Objective**: Verify complete startup flow from npm package

**Test Cases**:

| Test ID | Description | Expected Result |
|---------|-------------|-----------------|
| E2E-001 | First run (cold) | Services start |
| E2E-002 | Subsequent run (warm) | Fast startup |
| E2E-003 | Docker images pulled | All images present |
| E2E-004 | Services healthy | All health checks pass |
| E2E-005 | MCP responds | Request succeeds |

**Execution**:
```bash
# Clean environment
docker system prune -af
docker volume prune -f

# Install and start (first run)
time npx -y @crewchief/maproom-mcp@1.1.10 start

# Wait for startup
sleep 60

# Check services
docker ps

# Check health
docker inspect maproom-mcp --format='{{.State.Health.Status}}'
docker inspect maproom-postgres --format='{{.State.Health.Status}}'
docker inspect maproom-ollama --format='{{.State.Health.Status}}'

# Test MCP request (if applicable)
# curl http://localhost:3000/health

# Stop and restart (warm startup)
npx @crewchief/maproom-mcp stop
time npx @crewchief/maproom-mcp start
```

**Success Criteria**:
- First run completes in <3 minutes
- Subsequent runs complete in <30 seconds
- All services reach healthy state
- MCP server responds to requests

### 3.3 Multi-Platform Validation

**Objective**: Verify functionality across architectures

**Test Platforms**:

1. **Linux AMD64** (Ubuntu 22.04):
   - GitHub Actions runner
   - AWS EC2 t3.medium
   - DigitalOcean Droplet

2. **macOS ARM64** (Apple Silicon):
   - MacBook Pro M1/M2/M3
   - Mac Mini M1

3. **macOS Intel** (x86_64):
   - MacBook Pro Intel
   - iMac Intel

4. **Windows WSL2** (optional):
   - Windows 11 with WSL2
   - Ubuntu 22.04 in WSL

**Test Matrix**:

| Platform | Docker Version | Test Status | Notes |
|----------|----------------|-------------|-------|
| Linux AMD64 | 24.0+ | Required | Primary platform |
| macOS ARM64 | 24.0+ | Required | Apple Silicon |
| macOS Intel | 24.0+ | Should have | Legacy Macs |
| Windows WSL2 | 24.0+ | Nice to have | Advanced users |

**Execution**:
```bash
# On each platform:

# 1. Verify Docker
docker --version
docker info | grep Architecture

# 2. Install package
npm install -g @crewchief/maproom-mcp@1.1.10

# 3. Start services
npx @crewchief/maproom-mcp start

# 4. Verify platform-specific image pulled
docker inspect crewchief/maproom-mcp:1.1.10 \
  --format='{{.Architecture}}'

# 5. Run functionality tests
# (Add specific MCP tests here)

# 6. Collect results
docker logs maproom-mcp > logs-$(uname -m).txt
```

**Success Criteria**:
- Works on Linux AMD64 (must)
- Works on macOS ARM64 (must)
- Works on macOS Intel (should)
- Docker selects correct platform automatically
- No platform-specific errors

### 3.4 Version Pinning Test

**Objective**: Verify users can pin to specific versions

**Test Cases**:

| Test ID | Description | Expected Result |
|---------|-------------|-----------------|
| VER-001 | Use latest | Pulls :latest |
| VER-002 | Pin to patch (1.1.10) | Pulls :1.1.10 |
| VER-003 | Pin to minor (1.1) | Pulls :1.1 |
| VER-004 | Pin to major (1) | Pulls :1 |
| VER-005 | Invalid version | Error message |

**Execution**:
```bash
# Test latest (default)
npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test specific version
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test minor version
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test invalid version
MAPROOM_VERSION=99.99.99 npx @crewchief/maproom-mcp start
# Should fail with clear error
```

**Success Criteria**:
- Version pinning works as expected
- Correct image tag pulled
- Invalid versions fail gracefully
- Error messages are clear

## Level 4: Regression Testing

### 4.1 Backward Compatibility

**Objective**: Ensure new images don't break existing deployments

**Test Cases**:

| Test ID | Description | Expected Result |
|---------|-------------|-----------------|
| REG-001 | Existing env vars work | No errors |
| REG-002 | Existing volumes mount | Data persists |
| REG-003 | Existing networks connect | Services communicate |
| REG-004 | Database schema compatible | Migrations succeed |

**Execution**:
```bash
# Simulate upgrade from v1.1.9 to v1.1.10

# 1. Start with old version (if images existed)
# MAPROOM_VERSION=1.1.9 docker-compose up -d

# 2. Create some data
# (Add data via MCP)

# 3. Upgrade to new version
MAPROOM_VERSION=1.1.10 docker-compose up -d

# 4. Verify data intact
# (Query data via MCP)

# 5. Verify functionality unchanged
# (Run same requests as before)
```

**Success Criteria**:
- Upgrade completes without errors
- Data persists across upgrade
- Functionality unchanged
- Performance not degraded

### 4.2 Development Workflow Preservation

**Objective**: Ensure developers can still build locally

**Test Cases**:

| Test ID | Description | Expected Result |
|---------|-------------|-----------------|
| DEV-001 | Local build works | Image builds |
| DEV-002 | Override file works | Uses local build |
| DEV-003 | Production compose works | Uses Docker Hub |
| DEV-004 | CI/CD pipeline works | Tests pass |

**Execution**:
```bash
# Test development workflow
cd packages/maproom-mcp/config

# Create override file
cat > docker-compose.override.yml <<EOF
services:
  maproom-mcp:
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
EOF

# Build locally
docker-compose build

# Run with local build
docker-compose up -d

# Verify local image used
docker inspect maproom-mcp --format='{{.Config.Image}}'

# Test production mode (no override)
rm docker-compose.override.yml
docker-compose down
docker-compose up -d

# Verify Docker Hub image used
docker inspect maproom-mcp --format='{{.Config.Image}}'
```

**Success Criteria**:
- Local builds work
- Override file respected
- Production mode uses Docker Hub images
- Development workflow documented

## Test Automation Strategy

### GitHub Actions Test Workflow

**File**: `.github/workflows/test-docker-images.yml`

```yaml
name: Test Docker Images

on:
  pull_request:
    paths:
      - 'packages/maproom-mcp/**'
      - '.github/workflows/publish-maproom-mcp-image.yml'
  workflow_dispatch:

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      - name: Build test image
        run: |
          docker buildx build \
            --platform linux/amd64,linux/arm64 \
            --file packages/maproom-mcp/config/Dockerfile.mcp-server \
            --tag test-image \
            packages/maproom-mcp
      - name: Run Trivy scan
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: test-image
          exit-code: 1
          severity: CRITICAL

  integration-test:
    runs-on: ubuntu-latest
    needs: build-test
    steps:
      - uses: actions/checkout@v4
      - name: Build image
        run: |
          docker build \
            --file packages/maproom-mcp/config/Dockerfile.mcp-server \
            --tag test-image \
            packages/maproom-mcp
      - name: Start services
        run: |
          cd packages/maproom-mcp/config
          docker-compose up -d
      - name: Wait for services
        run: sleep 30
      - name: Check health
        run: |
          docker ps
          docker logs maproom-mcp
          docker inspect maproom-mcp --format='{{.State.Health.Status}}'
      - name: Cleanup
        if: always()
        run: |
          cd packages/maproom-mcp/config
          docker-compose down -v
```

### Manual Test Checklist

**Pre-Release Validation** (run before publishing):

- [ ] Build multi-platform images locally
- [ ] Test on Linux AMD64
- [ ] Test on macOS ARM64
- [ ] Verify image size <500MB
- [ ] Run Trivy security scan (0 critical)
- [ ] Test npm install from tarball
- [ ] Test first-run experience (cold)
- [ ] Test subsequent runs (warm)
- [ ] Verify all services start
- [ ] Check health checks pass
- [ ] Test version pinning
- [ ] Review logs for errors
- [ ] Test rollback to previous version

**Post-Release Validation** (run after publishing):

- [ ] Verify images on Docker Hub
- [ ] Test npm install from registry
- [ ] Test on fresh system
- [ ] Verify pull count increases
- [ ] Check GitHub Actions logs
- [ ] Monitor error reports
- [ ] Update documentation
- [ ] Announce release

## Performance Benchmarks

### Startup Time Benchmarks

| Scenario | Target | Threshold |
|----------|--------|-----------|
| First run (cold) | <2 min | <3 min |
| Subsequent run (warm) | <20s | <30s |
| Image pull | <2 min | <5 min |
| Container start | <10s | <15s |
| Health check pass | <15s | <30s |

### Resource Usage Benchmarks

| Metric | Target | Threshold |
|--------|--------|-----------|
| Image size | <300MB | <500MB |
| Memory (idle) | <150MB | <200MB |
| Memory (load) | <500MB | <1GB |
| CPU (idle) | <2% | <5% |
| Disk usage | <50MB | <100MB |

## Quality Gates

### Required (Must Pass)

1. **All builds succeed** (AMD64 + ARM64)
2. **Zero critical vulnerabilities**
3. **Image size <500MB**
4. **Startup time <30s (warm)**
5. **Health checks pass on all platforms**
6. **Integration tests pass**

### Recommended (Should Pass)

1. **Image size <300MB**
2. **<5 high vulnerabilities**
3. **Startup time <20s (warm)**
4. **Memory usage <200MB (idle)**
5. **Documentation updated**

### Nice-to-Have (Can Fail)

1. **Image size <200MB**
2. **Zero high vulnerabilities**
3. **Startup time <15s (warm)**
4. **Windows WSL2 support**

## Issue Tracking and Reporting

### Test Failure Protocol

**When tests fail**:
1. **Document failure**: Capture logs, screenshots, error messages
2. **Categorize severity**: Critical, High, Medium, Low
3. **Create GitHub issue**: Use template with test case ID
4. **Block release**: If critical/high severity
5. **Fix and retest**: Implement fix, rerun test suite

### Test Report Template

```markdown
## Test Report: Docker Hub Images v1.1.10

**Date**: 2025-10-29
**Tester**: [Name]
**Platforms**: Linux AMD64, macOS ARM64

### Summary
- Total Tests: 45
- Passed: 43
- Failed: 2
- Skipped: 0

### Failures
1. **SEC-001**: 1 critical vulnerability found
   - Severity: High
   - Impact: Blocks release
   - Mitigation: Update base image

2. **RES-002**: Memory usage 1.2GB under load
   - Severity: Medium
   - Impact: Performance degradation
   - Mitigation: Optimize code, increase limit

### Platform Results
- Linux AMD64: ✅ All pass
- macOS ARM64: ⚠️  2 failures
- macOS Intel: ✅ All pass

### Recommendations
- Update node:20-alpine to latest
- Optimize memory usage
- Retest on macOS ARM64
- Do not release until failures resolved
```

## Next Steps

1. **Review DKRHUB_SECURITY_REVIEW.md** for security testing details
2. **Review DKRHUB_PLAN.md** for implementation timeline
3. **Set up test environments** (Linux, macOS)
4. **Prepare test data** for integration tests
5. **Begin implementation** with testing mindset

---

**Status**: Quality strategy defined, ready for implementation with testing
