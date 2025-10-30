# DKRHUB-1901 Test Plan: GitHub Actions Workflow with Pre-Release Tag

**Ticket**: DKRHUB-1901
**Version**: v1.1.10-rc1
**Created**: 2025-10-30
**Status**: Ready for Manual Execution

## Executive Summary

This test plan validates the complete GitHub Actions workflow for publishing Docker images to Docker Hub. The test uses a pre-release tag (v1.1.10-rc1) to verify all pipeline steps before the production v1.1.10 release.

**Critical**: This test requires manual execution by a user with:
- GitHub push access to the repository
- Docker Hub account access to verify published images
- Local Docker installation with buildx support
- Network access to GitHub Actions and Docker Hub

---

## Prerequisites Checklist

Before beginning this test, verify the following prerequisites are met:

### GitHub Configuration
- [ ] GitHub Secrets configured:
  - `DOCKERHUB_USERNAME` - Docker Hub username
  - `DOCKERHUB_TOKEN` - Docker Hub access token (not password)
- [ ] User has push access to the repository
- [ ] User has access to GitHub Actions workflows
- [ ] User has access to GitHub Security tab

### Docker Hub Configuration
- [ ] Docker Hub account exists: `crewchief`
- [ ] User has access to view `crewchief/maproom-mcp` repository
- [ ] Docker Hub repository is public (for testing pulls)

### Local Environment
- [ ] Git client installed and configured
- [ ] Docker installed (version 20.10+)
- [ ] Docker Buildx enabled
- [ ] `jq` command-line tool installed (for JSON parsing)
- [ ] Repository cloned locally

### Code Dependencies
- [ ] DKRHUB-1000: `Dockerfile.combined` exists and is tested locally
- [ ] DKRHUB-1007: Local Dockerfile testing passed completely
- [ ] DKRHUB-1001 through DKRHUB-1006: All workflow steps implemented
- [ ] Workflow file exists: `.github/workflows/publish-maproom-mcp-image.yml`

---

## Test Execution Steps

### Phase 1: Create and Push Test Tag

**Duration**: 5 minutes
**Objective**: Trigger the GitHub Actions workflow with a pre-release tag

#### Step 1.1: Verify Current State

```bash
# Ensure you're on the correct branch
git status

# Verify no uncommitted changes
git diff --stat

# Check current commit
git log -1 --oneline
```

**Expected**: Clean working directory, ready to tag.

#### Step 1.2: Create Pre-Release Tag

```bash
# Create annotated tag
git tag -a v1.1.10-rc1 -m "Test release for workflow validation

This is a pre-release tag to validate the GitHub Actions workflow
before tagging the production v1.1.10 release.

Test validates:
- Workflow trigger on tag push
- Multi-platform image builds (AMD64, ARM64)
- Docker Hub publishing
- Security scanning
- Metadata generation
"

# Verify tag created
git tag -l "v1.1.10*"
```

**Expected Output**:
```
v1.1.10-rc1
```

#### Step 1.3: Push Tag to Remote

```bash
# Push tag to origin (triggers workflow)
git push origin v1.1.10-rc1
```

**Expected Output**:
```
Enumerating objects: 1, done.
Counting objects: 100% (1/1), done.
Writing objects: 100% (1/1), 200 bytes | 200.00 KiB/s, done.
Total 1 (delta 0), reused 0 (delta 0)
To github.com:danielbushman/crewchief.git
 * [new tag]         v1.1.10-rc1 -> v1.1.10-rc1
```

**Action**: Note the exact timestamp of push for workflow correlation.

---

### Phase 2: Monitor GitHub Actions Workflow

**Duration**: 15-20 minutes
**Objective**: Verify all workflow steps complete successfully

#### Step 2.1: Navigate to GitHub Actions

1. Open browser to: `https://github.com/danielbushman/crewchief/actions`
2. Look for workflow run: "Publish Maproom MCP Docker Image"
3. Click on the most recent run (triggered by v1.1.10-rc1 tag)

**Expected**: Workflow run appears within 30 seconds of tag push.

#### Step 2.2: Monitor Workflow Progress

Watch the workflow execute through these steps:

| Step | Name | Duration | Status |
|------|------|----------|--------|
| 1 | Checkout code | ~15s | 🟢 |
| 2 | Set up QEMU | ~30s | 🟢 |
| 3 | Set up Docker Buildx | ~15s | 🟢 |
| 4 | Login to Docker Hub | ~5s | 🟢 |
| 5 | Extract version | ~5s | 🟢 |
| 6 | Generate Docker metadata | ~5s | 🟢 |
| 7 | Build and push Docker image | ~12-15min | 🟢 |
| 8 | Run Trivy security scan | ~2-3min | 🟢 |
| 9 | Upload Trivy results | ~10s | 🟢 |

**Total Expected Duration**: 15-20 minutes

#### Step 2.3: Verify Each Step

For each step, click to expand logs and verify:

##### Step 1: Checkout Code
```
✓ Checking out the ref
✓ Fetching repository
✓ Determining the checkout info
✓ Checking out the repository
```

**Checkpoints**:
- [ ] No authentication errors
- [ ] Full history fetched (fetch-depth: 0)
- [ ] Correct commit SHA displayed

##### Step 2: Set Up QEMU
```
✓ Setting up QEMU
✓ Platforms: linux/amd64,linux/arm64
```

**Checkpoints**:
- [ ] Both platforms registered successfully
- [ ] No emulation errors

##### Step 3: Set Up Docker Buildx
```
✓ Creating builder instance
✓ Booting builder
✓ Builder ready
```

**Checkpoints**:
- [ ] Builder created with network=host
- [ ] Buildx version >= 0.10.0
- [ ] Multi-platform support confirmed

##### Step 4: Login to Docker Hub
```
✓ Logging in to Docker Hub
✓ Login Succeeded
```

**Checkpoints**:
- [ ] Login successful
- [ ] **CRITICAL**: Verify secrets are REDACTED in logs
  - Username should show as `***` or be partially masked
  - Token should show as `***`
- [ ] No plaintext credentials visible

**Security Validation**:
```
# In the logs, you should see:
username: ***
password: ***

# Or similar masking patterns, NOT:
username: crewchief
password: dckr_pat_abc123...
```

##### Step 5: Extract Version
```
Extracted versions:
  Full: 1.1.10-rc1
  Minor: 1.1
  Major: 1
```

**Checkpoints**:
- [ ] Version correctly extracted from tag: `1.1.10-rc1`
- [ ] Minor version: `1.1`
- [ ] Major version: `1`
- [ ] No version parsing errors

##### Step 6: Generate Docker Metadata
```
Tags:
  crewchief/maproom-mcp:1.1.10-rc1
  crewchief/maproom-mcp:1.1
  crewchief/maproom-mcp:1
  crewchief/maproom-mcp:latest
```

**Checkpoints**:
- [ ] Four tags generated
- [ ] Full version tag: `1.1.10-rc1`
- [ ] Minor version tag: `1.1`
- [ ] Major version tag: `1`
- [ ] Latest tag included
- [ ] Labels include:
  - `org.opencontainers.image.title=Maproom MCP Server`
  - `org.opencontainers.image.version=1.1.10-rc1`
  - `org.opencontainers.image.vendor=CrewChief`

##### Step 7: Build and Push Docker Image

This is the longest step. Monitor for:

**AMD64 Build**:
```
[linux/amd64 1/8] FROM docker.io/library/node:20-slim
[linux/amd64 2/8] RUN apt-get update && apt-get install -y ...
[linux/amd64 3/8] WORKDIR /app
...
[linux/amd64 8/8] CMD ["node", "dist/index.js"]
✓ linux/amd64 build complete
```

**ARM64 Build**:
```
[linux/arm64 1/8] FROM docker.io/library/node:20-slim
[linux/arm64 2/8] RUN apt-get update && apt-get install -y ...
[linux/arm64 3/8] WORKDIR /app
...
[linux/arm64 8/8] CMD ["node", "dist/index.js"]
✓ linux/arm64 build complete
```

**Push**:
```
Pushing manifest for crewchief/maproom-mcp:1.1.10-rc1
Pushing manifest for crewchief/maproom-mcp:1.1
Pushing manifest for crewchief/maproom-mcp:1
Pushing manifest for crewchief/maproom-mcp:latest
✓ Push complete
```

**Checkpoints**:
- [ ] AMD64 build completes without errors
- [ ] ARM64 build completes without errors (may use QEMU emulation)
- [ ] Build cache shows hits (look for "CACHED" in logs)
- [ ] No timeout errors (build completes in <15 minutes)
- [ ] Both platform manifests pushed
- [ ] All four tags pushed successfully
- [ ] No "push denied" or authentication errors

**Performance Validation**:
- [ ] Build time < 20 minutes (acceptance criteria)
- [ ] Build time < 15 minutes (target)
- [ ] Cache hit rate > 50% (if rebuilding)

##### Step 8: Run Trivy Security Scan
```
Running Trivy scan on crewchief/maproom-mcp:1.1.10-rc1
Scanning image for CRITICAL and HIGH vulnerabilities
...
Total: X vulnerabilities (0 CRITICAL, Y HIGH)
✓ Scan complete
```

**Checkpoints**:
- [ ] Scan completes successfully
- [ ] **CRITICAL**: 0 CRITICAL vulnerabilities (exit-code: 1 would fail workflow)
- [ ] HIGH vulnerabilities count noted (for remediation)
- [ ] SARIF report generated: `trivy-results.sarif`

**If Scan Fails (exit-code: 1)**:
- Workflow will fail at this step
- Review vulnerabilities in logs
- Determine if critical vulnerabilities are:
  - In base image (node:20-slim) - may need to accept or use different base
  - In dependencies - update package versions
  - False positives - add to Trivy ignore list

##### Step 9: Upload Trivy Results
```
Uploading SARIF file to GitHub Security
✓ Upload successful
```

**Checkpoints**:
- [ ] Upload completes successfully
- [ ] No permission errors (requires `security-events: write` permission)

---

### Phase 3: Verify Docker Hub Publication

**Duration**: 5 minutes
**Objective**: Confirm images are published correctly to Docker Hub

#### Step 3.1: Verify Repository and Tags

1. Open browser to: `https://hub.docker.com/r/crewchief/maproom-mcp`
2. Navigate to "Tags" tab

**Expected Tags**:
- `1.1.10-rc1` - Full version tag
- `1.1` - Minor version tag
- `1` - Major version tag
- `latest` - Latest release tag

**Checkpoints**:
- [ ] All four tags present
- [ ] Tags show correct "Last Pushed" timestamp (within last hour)
- [ ] Tags show correct digest (SHA256 hash)

#### Step 3.2: Verify Multi-Platform Support

Click on `1.1.10-rc1` tag, then check "Architectures" section:

**Expected Architectures**:
- `linux/amd64`
- `linux/arm64`

**Checkpoints**:
- [ ] Both architectures present
- [ ] Both show same OS/Version
- [ ] Image sizes reasonable:
  - AMD64: ~350-450MB uncompressed
  - ARM64: ~350-450MB uncompressed

#### Step 3.3: Verify Image Metadata

In Docker Hub UI, check image details:

**Expected Metadata**:
- **OS/Arch**: Linux (amd64, arm64)
- **Compressed Size**: ~150-200MB per platform
- **Layers**: ~8-12 layers
- **Description**: "Semantic code search MCP server with local LLM embeddings"

**Checkpoints**:
- [ ] Metadata complete and accurate
- [ ] Image description present
- [ ] Layer count reasonable (<20 layers)
- [ ] Compressed size < 200MB per platform

---

### Phase 4: Test Image Functionality

**Duration**: 10 minutes
**Objective**: Verify images can be pulled and executed correctly

#### Step 4.1: Pull Multi-Platform Image (Default)

```bash
# Pull image (Docker will select platform automatically)
docker pull crewchief/maproom-mcp:1.1.10-rc1
```

**Expected Output**:
```
1.1.10-rc1: Pulling from crewchief/maproom-mcp
<hash>: Pull complete
<hash>: Pull complete
...
Digest: sha256:<full-hash>
Status: Downloaded newer image for crewchief/maproom-mcp:1.1.10-rc1
docker.io/crewchief/maproom-mcp:1.1.10-rc1
```

**Checkpoints**:
- [ ] Pull succeeds without errors
- [ ] Image digest matches Docker Hub
- [ ] Pull completes in <2 minutes

#### Step 4.2: Verify Platform Detection

```bash
# Check which platform was pulled
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Architecture}}'
```

**Expected Output** (depends on local machine):
- On Intel/AMD Mac or Linux: `amd64`
- On Apple Silicon Mac: `arm64`

**Checkpoints**:
- [ ] Platform matches local architecture
- [ ] Docker automatically selected correct platform

#### Step 4.3: Test AMD64 Image Specifically

```bash
# Pull AMD64 image explicitly
docker pull --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1

# Verify architecture
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Architecture}}'
```

**Expected Output**: `amd64`

**Checkpoints**:
- [ ] AMD64 image pulls successfully
- [ ] Architecture is `amd64`

#### Step 4.4: Test ARM64 Image Specifically

```bash
# Pull ARM64 image explicitly
docker pull --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1

# Verify architecture
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Architecture}}'
```

**Expected Output**: `arm64`

**Checkpoints**:
- [ ] ARM64 image pulls successfully
- [ ] Architecture is `arm64`

**Note**: On non-ARM64 systems, this may use QEMU emulation.

#### Step 4.5: Verify Multi-Platform Manifest

```bash
# Inspect multi-platform manifest
docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1
```

**Expected Output**:
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.docker.distribution.manifest.list.v2+json",
  "manifests": [
    {
      "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
      "size": 1234,
      "digest": "sha256:<amd64-hash>",
      "platform": {
        "architecture": "amd64",
        "os": "linux"
      }
    },
    {
      "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
      "size": 1234,
      "digest": "sha256:<arm64-hash>",
      "platform": {
        "architecture": "arm64",
        "os": "linux"
      }
    }
  ]
}
```

**Extract Platform Info**:
```bash
# Pretty-print platforms
docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1 | \
  jq '.manifests[] | {platform: .platform, size: .size}'
```

**Expected Output**:
```json
{
  "platform": {
    "architecture": "amd64",
    "os": "linux"
  },
  "size": 1234
}
{
  "platform": {
    "architecture": "arm64",
    "os": "linux"
  },
  "size": 5678
}
```

**Checkpoints**:
- [ ] Manifest list exists (not single manifest)
- [ ] Two platform manifests present
- [ ] Both platforms have correct OS (`linux`)
- [ ] Both platforms have correct architectures (`amd64`, `arm64`)
- [ ] Both platforms have reasonable sizes

---

### Phase 5: Test Component Functionality

**Duration**: 10 minutes
**Objective**: Verify both Node.js runtime and Rust binary exist and function

#### Step 5.1: Test Node.js Runtime (AMD64)

```bash
# Verify Node.js version
docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 node --version
```

**Expected Output**: `v20.x.x` (or configured Node version)

**Checkpoints**:
- [ ] Node.js executable exists
- [ ] Node.js version is correct (v20.x)
- [ ] Container starts and exits cleanly

#### Step 5.2: Test Rust Binary (AMD64)

```bash
# Verify Rust binary version
docker run --rm --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version
```

**Expected Output**: `crewchief-maproom 0.x.x` (or current version)

**Checkpoints**:
- [ ] Rust binary executable exists
- [ ] Rust binary version is correct
- [ ] Binary runs without missing dependencies

#### Step 5.3: Test Node.js Runtime (ARM64)

```bash
# Verify Node.js version
docker run --rm --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 node --version
```

**Expected Output**: `v20.x.x`

**Checkpoints**:
- [ ] Node.js executable exists (ARM64 build)
- [ ] Node.js version matches AMD64
- [ ] Container starts on ARM64 platform

#### Step 5.4: Test Rust Binary (ARM64)

```bash
# Verify Rust binary version
docker run --rm --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version
```

**Expected Output**: `crewchief-maproom 0.x.x`

**Checkpoints**:
- [ ] Rust binary executable exists (ARM64 build)
- [ ] Rust binary version matches AMD64
- [ ] Binary compiled for correct architecture

#### Step 5.5: Test MCP Server Startup

```bash
# Start MCP server (AMD64)
docker run --rm --platform linux/amd64 \
  -e DATABASE_URL="postgresql://user:pass@host:5432/db" \
  crewchief/maproom-mcp:1.1.10-rc1 &

# Wait 5 seconds
sleep 5

# Check logs
docker logs <container-id>
```

**Expected Output** (or similar):
```
Maproom MCP Server starting...
Connecting to database...
Server ready on stdio
```

**Checkpoints**:
- [ ] Server starts without errors
- [ ] Database connection attempted (even if fails due to test URL)
- [ ] No missing file errors
- [ ] No permission errors

**Stop Container**:
```bash
docker stop <container-id>
```

---

### Phase 6: Validate Image Metadata and Size

**Duration**: 5 minutes
**Objective**: Ensure image quality and optimization

#### Step 6.1: Check Image Sizes

```bash
# List all pulled images
docker images crewchief/maproom-mcp:1.1.10-rc1
```

**Expected Output**:
```
REPOSITORY                 TAG          IMAGE ID       CREATED         SIZE
crewchief/maproom-mcp     1.1.10-rc1   <hash>        15 minutes ago   380MB
```

**Checkpoints**:
- [ ] Image size < 450MB (acceptance criteria)
- [ ] Image size ~350-400MB (target for Rust + Node.js combined)
- [ ] Size reasonable for included components

**If Image Size > 450MB**:
- Review Dockerfile for optimization opportunities
- Check for unnecessary dependencies
- Verify multi-stage build is working correctly

#### Step 6.2: Inspect Image Labels

```bash
# View all labels
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{json .Config.Labels}}' | jq
```

**Expected Output**:
```json
{
  "org.opencontainers.image.title": "Maproom MCP Server",
  "org.opencontainers.image.description": "Semantic code search MCP server with local LLM embeddings",
  "org.opencontainers.image.vendor": "CrewChief",
  "org.opencontainers.image.version": "1.1.10-rc1",
  "org.opencontainers.image.revision": "<commit-sha>",
  "org.opencontainers.image.created": "2025-10-30T...",
  "org.opencontainers.image.source": "https://github.com/danielbushman/crewchief"
}
```

**Checkpoints**:
- [ ] Title label: "Maproom MCP Server"
- [ ] Vendor label: "CrewChief"
- [ ] Version label: "1.1.10-rc1"
- [ ] Revision label: valid commit SHA (40 chars)
- [ ] Created label: valid ISO 8601 timestamp
- [ ] Description label present

#### Step 6.3: Analyze Image Layers

```bash
# View layer history
docker history crewchief/maproom-mcp:1.1.10-rc1
```

**Expected Output** (abbreviated):
```
IMAGE          CREATED         CREATED BY                                      SIZE
<hash>        15 minutes ago  CMD ["node" "dist/index.js"]                    0B
<hash>        15 minutes ago  COPY . /app                                     50MB
<hash>        16 minutes ago  RUN cargo build --release                       120MB
<hash>        18 minutes ago  COPY Cargo.toml Cargo.lock /build               10kB
<hash>        20 minutes ago  FROM node:20-slim                              180MB
```

**Checkpoints**:
- [ ] Layer count < 20 (acceptance criteria)
- [ ] No excessively large layers (except base image)
- [ ] Multi-stage build evident (separate build and runtime stages)
- [ ] Final layer is CMD directive (0B size)

#### Step 6.4: Verify Entrypoint and Command

```bash
# Check entrypoint and command
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Config.Entrypoint}}'
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{.Config.Cmd}}'
```

**Expected Output**:
```
[]
[node dist/index.js]
```

**Checkpoints**:
- [ ] Entrypoint is empty or correct
- [ ] Command starts Node.js server
- [ ] Working directory is `/app`

---

### Phase 7: Verify GitHub Security Integration

**Duration**: 5 minutes
**Objective**: Confirm security scan results uploaded correctly

#### Step 7.1: Navigate to GitHub Security Tab

1. Open browser to: `https://github.com/danielbushman/crewchief/security`
2. Click "Code scanning" in left sidebar
3. Look for Trivy scan results for v1.1.10-rc1

**Checkpoints**:
- [ ] Trivy scan results present
- [ ] Scan timestamp matches workflow run
- [ ] Scan shows image tag: `1.1.10-rc1`

#### Step 7.2: Review Security Findings

**Expected Findings**:
- **Critical**: 0 (workflow would have failed otherwise)
- **High**: 0-10 (acceptable range, review each)
- **Medium**: May vary
- **Low**: May vary

**For Each Finding**:
- Note affected package
- Note CVE identifier
- Determine if:
  - False positive
  - Fixed in newer version
  - Requires base image update
  - Acceptable risk

**Checkpoints**:
- [ ] No critical vulnerabilities (required)
- [ ] High vulnerabilities reviewed and documented
- [ ] Each finding has remediation plan (if needed)

#### Step 7.3: Download and Inspect SARIF Report

```bash
# Download SARIF report from GitHub Actions artifacts
# (Navigate to workflow run -> Artifacts section)

# Inspect SARIF report locally
jq '.' trivy-results.sarif
```

**Checkpoints**:
- [ ] SARIF report well-formed JSON
- [ ] Results array contains vulnerability details
- [ ] Severity levels correctly mapped
- [ ] Location information present for each finding

---

### Phase 8: Performance Validation

**Duration**: 5 minutes
**Objective**: Verify workflow performance meets acceptance criteria

#### Step 8.1: Review Workflow Timing

Navigate to GitHub Actions workflow run, review timing:

| Step | Target Duration | Acceptable Duration | Actual Duration |
|------|----------------|---------------------|-----------------|
| Checkout | <30s | <1min | _____ |
| QEMU Setup | <30s | <1min | _____ |
| Buildx Setup | <30s | <1min | _____ |
| Docker Login | <10s | <30s | _____ |
| Extract Version | <10s | <30s | _____ |
| Generate Metadata | <10s | <30s | _____ |
| Build and Push | <15min | <18min | _____ |
| Trivy Scan | <3min | <5min | _____ |
| Upload Results | <30s | <1min | _____ |
| **Total** | **<20min** | **<25min** | **_____** |

**Checkpoints**:
- [ ] Total workflow time < 20 minutes (acceptance criteria)
- [ ] Build step < 15 minutes (target)
- [ ] No timeout errors
- [ ] Performance acceptable for CI/CD pipeline

#### Step 8.2: Review Cache Utilization

In "Build and Push" step logs, search for cache hits:

**Expected Cache Behavior**:
```
--> FROM node:20-slim
--> CACHED [1/8] FROM docker.io/library/node:20-slim
--> CACHED [2/8] RUN apt-get update ...
--> CACHED [3/8] WORKDIR /app
--> CACHED [4/8] COPY package*.json ./
--> CACHED [5/8] RUN npm ci --production
--> [6/8] COPY . .
--> [7/8] RUN cargo build --release
--> [8/8] CMD ["node", "dist/index.js"]
```

**Checkpoints**:
- [ ] Base image layers cached
- [ ] Dependency installation layers cached (if unchanged)
- [ ] Cache hit rate > 50% (warm builds)
- [ ] Build speed improved by caching

---

## Success Criteria Validation

After completing all test phases, validate against acceptance criteria:

### Workflow Execution
- [ ] Test tag created: `v1.1.10-rc1` and pushed to repository
- [ ] GitHub Actions workflow triggered automatically on tag push
- [ ] All workflow steps completed without errors
- [ ] Build completed in <20 minutes

### Docker Hub Publication
- [ ] Images appear on Docker Hub at `crewchief/maproom-mcp:1.1.10-rc1`
- [ ] Multi-platform manifest includes both platforms
- [ ] AMD64 image builds successfully
- [ ] ARM64 image builds successfully
- [ ] AMD64 image can be pulled
- [ ] ARM64 image can be pulled

### Component Validation
- [ ] Both components exist in AMD64 image: Node.js runtime + Rust binary
- [ ] Both components exist in ARM64 image: Node.js runtime + Rust binary
- [ ] Image size reasonable (< 450MB for combined image)

### Security and Compliance
- [ ] No credentials visible in GitHub Actions logs (secrets redacted)
- [ ] Trivy scan results uploaded to GitHub Security tab
- [ ] No critical vulnerabilities found

### Metadata
- [ ] Image labels present and correct
- [ ] Version metadata matches tag
- [ ] All four tags created (1.1.10-rc1, 1.1, 1, latest)

---

## Rollback Procedures

If any test fails, follow these rollback procedures:

### Scenario 1: Workflow Fails During Build

**Symptoms**:
- Build step fails with errors
- Timeout during build
- Authentication errors

**Actions**:
1. Review workflow logs to identify failure point
2. Fix issue in workflow YAML or Dockerfile
3. Delete test tag:
   ```bash
   git tag -d v1.1.10-rc1
   git push origin :refs/tags/v1.1.10-rc1
   ```
4. Test fix locally with `docker buildx build`
5. Re-run test with new tag: `v1.1.10-rc2`

### Scenario 2: Security Scan Fails (Critical Vulnerabilities)

**Symptoms**:
- Trivy scan exits with code 1
- Critical vulnerabilities detected
- Workflow fails at security scan step

**Actions**:
1. Review Trivy findings in workflow logs
2. Identify affected packages and CVEs
3. Determine remediation:
   - Update base image version
   - Update affected dependencies
   - Add Trivy ignore file (`.trivyignore`) if false positive
4. Delete test tag and images:
   ```bash
   git tag -d v1.1.10-rc1
   git push origin :refs/tags/v1.1.10-rc1
   ```
5. Apply fixes and retest locally
6. Re-run test with new tag: `v1.1.10-rc2`

### Scenario 3: Images Published But Don't Work

**Symptoms**:
- Images pull successfully
- Container fails to start
- Missing files or dependencies

**Actions**:
1. Delete images from Docker Hub:
   - Navigate to Docker Hub web UI
   - Select `crewchief/maproom-mcp` repository
   - Delete tags: `1.1.10-rc1`, `1.1`, `1`, `latest`
2. Delete test tag:
   ```bash
   git tag -d v1.1.10-rc1
   git push origin :refs/tags/v1.1.10-rc1
   ```
3. Fix Dockerfile issues
4. Test Dockerfile locally:
   ```bash
   docker build -f packages/maproom-mcp/config/Dockerfile.combined -t test-local .
   docker run --rm test-local crewchief-maproom --version
   docker run --rm test-local node --version
   ```
5. Re-run test with new tag: `v1.1.10-rc2`

### Scenario 4: Secrets Exposed in Logs

**Symptoms**:
- Credentials visible in plaintext in workflow logs
- Docker Hub token not redacted
- Username not masked

**Actions**:
1. **IMMEDIATELY** rotate exposed credentials:
   - Docker Hub: Revoke and regenerate access token
   - GitHub: Update `DOCKERHUB_TOKEN` secret
2. Contact GitHub Support to purge workflow logs
3. Delete test tag and images:
   ```bash
   git tag -d v1.1.10-rc1
   git push origin :refs/tags/v1.1.10-rc1
   ```
4. Review workflow YAML for secret handling issues
5. Test secret masking with workflow_dispatch (set push_to_registry: false)
6. Re-run test with new tag: `v1.1.10-rc2`

### Scenario 5: Wrong Platform Images

**Symptoms**:
- Only AMD64 image exists
- Only ARM64 image exists
- Manifest doesn't show both platforms

**Actions**:
1. Review build logs for platform-specific errors
2. Check QEMU setup logs
3. Verify Buildx configuration
4. Delete test tag and images
5. Test multi-platform build locally:
   ```bash
   docker buildx build \
     --platform linux/amd64,linux/arm64 \
     -f packages/maproom-mcp/config/Dockerfile.combined \
     -t test-multi \
     .
   ```
6. Re-run test with new tag: `v1.1.10-rc2`

---

## Post-Test Cleanup

### If Test Succeeds

**Keep Test Artifacts**:
- Leave test tag `v1.1.10-rc1` in repository (documents test)
- Leave test images on Docker Hub (clearly marked as -rc1)
- These serve as reference for production release

**Document Success**:
1. Update ticket DKRHUB-1901 with test results
2. Note actual timings, image sizes, and any findings
3. Proceed to Phase 2 (docker-compose updates)

### If Test Fails

**Clean Up Test Artifacts**:
1. Delete test tag:
   ```bash
   git tag -d v1.1.10-rc1
   git push origin :refs/tags/v1.1.10-rc1
   ```
2. Delete test images from Docker Hub (web UI)
3. Close or cancel workflow run in GitHub Actions

**Document Failure**:
1. Update ticket DKRHUB-1901 with failure details
2. Create issues for identified problems
3. Plan fixes before retesting

---

## Troubleshooting Guide

### Issue: "Permission denied" when pushing tag

**Cause**: No push access to repository

**Solution**:
1. Verify GitHub user has write access
2. Check authentication: `git remote -v`
3. Ensure using correct authentication method (SSH or HTTPS with token)

### Issue: Workflow doesn't trigger

**Cause**: Tag pattern doesn't match workflow trigger

**Solution**:
1. Verify tag matches pattern `v*.*.*`
2. Check `.github/workflows/publish-maproom-mcp-image.yml` trigger configuration
3. Manually trigger with workflow_dispatch if needed

### Issue: Docker login fails in workflow

**Cause**: Missing or incorrect GitHub Secrets

**Solution**:
1. Verify secrets exist: Settings → Secrets and variables → Actions
2. Check secret names match workflow: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`
3. Regenerate Docker Hub token if expired

### Issue: ARM64 build times out

**Cause**: ARM64 emulation is slow on AMD64 runners

**Solution**:
1. This is expected behavior (ARM64 builds can take 2-3x longer)
2. Verify timeout settings in workflow
3. Consider using native ARM64 runners (GitHub hosted or self-hosted)

### Issue: Image size too large (> 450MB)

**Cause**: Inefficient Dockerfile or unnecessary dependencies

**Solution**:
1. Review Dockerfile for optimization opportunities
2. Ensure multi-stage build is working correctly
3. Check for development dependencies in production image
4. Use `docker history` to identify large layers

### Issue: Trivy scan fails with critical vulnerabilities

**Cause**: Known vulnerabilities in dependencies or base image

**Solution**:
1. Review specific CVEs in scan results
2. Update base image to newer version
3. Update affected npm packages
4. Add `.trivyignore` for false positives (with justification)

---

## Test Report Template

After completing the test, fill out this report:

```
# DKRHUB-1901 Test Execution Report

**Test Date**: YYYY-MM-DD HH:MM
**Test Executor**: [Your Name]
**Tag Tested**: v1.1.10-rc1
**Result**: [SUCCESS / FAILURE]

## Test Summary

- **Workflow Duration**: ___ minutes
- **AMD64 Image Size**: ___ MB
- **ARM64 Image Size**: ___ MB
- **Critical Vulnerabilities**: ___
- **High Vulnerabilities**: ___

## Detailed Results

### Phase 1: Tag Creation
- [ ] PASS / [ ] FAIL - Tag created and pushed
- Notes: ___

### Phase 2: Workflow Execution
- [ ] PASS / [ ] FAIL - All steps completed
- Build Time: ___ minutes
- Notes: ___

### Phase 3: Docker Hub Publication
- [ ] PASS / [ ] FAIL - Images published
- Tags Created: ___
- Notes: ___

### Phase 4: Image Functionality
- [ ] PASS / [ ] FAIL - AMD64 image works
- [ ] PASS / [ ] FAIL - ARM64 image works
- Notes: ___

### Phase 5: Component Validation
- [ ] PASS / [ ] FAIL - Node.js runtime present
- [ ] PASS / [ ] FAIL - Rust binary present
- Notes: ___

### Phase 6: Metadata Validation
- [ ] PASS / [ ] FAIL - Labels correct
- [ ] PASS / [ ] FAIL - Size within limits
- Notes: ___

### Phase 7: Security Integration
- [ ] PASS / [ ] FAIL - Scan results uploaded
- [ ] PASS / [ ] FAIL - No critical vulnerabilities
- Notes: ___

### Phase 8: Performance Validation
- [ ] PASS / [ ] FAIL - Build time < 20 minutes
- [ ] PASS / [ ] FAIL - Cache utilization good
- Notes: ___

## Issues Encountered

[List any issues, workarounds, or unexpected behavior]

## Recommendations

[Any recommendations for production release or improvements]

## Next Steps

[If success: Proceed to Phase 2]
[If failure: List required fixes]

## Approvals

Tested by: ___________
Reviewed by: ___________
Approved for production: [ ] YES / [ ] NO
```

---

## Quick Reference: Essential Commands

```bash
# Create and push tag
git tag -a v1.1.10-rc1 -m "Test release"
git push origin v1.1.10-rc1

# Pull and inspect image
docker pull crewchief/maproom-mcp:1.1.10-rc1
docker images crewchief/maproom-mcp:1.1.10-rc1
docker inspect crewchief/maproom-mcp:1.1.10-rc1

# Test multi-platform
docker manifest inspect crewchief/maproom-mcp:1.1.10-rc1
docker pull --platform linux/amd64 crewchief/maproom-mcp:1.1.10-rc1
docker pull --platform linux/arm64 crewchief/maproom-mcp:1.1.10-rc1

# Test components
docker run --rm crewchief/maproom-mcp:1.1.10-rc1 node --version
docker run --rm crewchief/maproom-mcp:1.1.10-rc1 crewchief-maproom --version

# Verify metadata
docker inspect crewchief/maproom-mcp:1.1.10-rc1 --format='{{json .Config.Labels}}' | jq
docker history crewchief/maproom-mcp:1.1.10-rc1

# Cleanup (if needed)
git tag -d v1.1.10-rc1
git push origin :refs/tags/v1.1.10-rc1
docker rmi crewchief/maproom-mcp:1.1.10-rc1
```

---

## Contact and Support

**If you encounter issues during testing**:

1. Review troubleshooting section above
2. Check workflow logs in GitHub Actions
3. Review Docker Hub for published images
4. Document issue in test report
5. Contact project maintainer: danielbushman

**For workflow failures**:
- Review GitHub Actions logs: https://github.com/danielbushman/crewchief/actions
- Check Docker Hub status: https://status.docker.com/

**For security concerns**:
- Review Trivy scan results in GitHub Security tab
- Consult CVE database: https://cve.mitre.org/
- Contact security team if critical vulnerabilities found

---

## Appendix A: Expected GitHub Actions Output

### Successful Workflow Run Summary

```
✓ Checkout code (15s)
✓ Set up QEMU (28s)
✓ Set up Docker Buildx (12s)
✓ Login to Docker Hub (4s)
✓ Extract version (3s)
✓ Generate Docker metadata (5s)
✓ Build and push Docker image (14m 23s)
✓ Run Trivy security scan (2m 15s)
✓ Upload Trivy results to GitHub Security (8s)

Total: 17m 53s
```

### Expected Build Log Excerpt

```
#1 [internal] load build definition from Dockerfile.combined
#1 transferring dockerfile: 2.1kB
#1 DONE 0.1s

#2 [internal] load .dockerignore
#2 transferring context: 34B
#2 DONE 0.1s

#3 [linux/amd64 internal] load metadata for docker.io/library/node:20-slim
#3 DONE 0.8s

#4 [linux/arm64 internal] load metadata for docker.io/library/node:20-slim
#4 DONE 0.8s

#5 [linux/amd64 1/8] FROM docker.io/library/node:20-slim
#5 CACHED

#6 [linux/arm64 1/8] FROM docker.io/library/node:20-slim
#6 CACHED

...

#20 [linux/amd64] exporting to image
#20 exporting layers 12.3s
#20 writing image sha256:abc123... 0.1s
#20 naming to docker.io/crewchief/maproom-mcp:1.1.10-rc1 0.1s
#20 DONE 12.5s

#21 [linux/arm64] exporting to image
#21 exporting layers 14.1s
#21 writing image sha256:def456... 0.1s
#21 naming to docker.io/crewchief/maproom-mcp:1.1.10-rc1 0.1s
#21 DONE 14.3s
```

---

## Appendix B: Security Scan Interpretation

### Trivy Severity Levels

| Severity | Description | Action Required |
|----------|-------------|-----------------|
| **CRITICAL** | Exploitable vulnerability with high impact | **MUST FIX** - Workflow fails |
| **HIGH** | Serious vulnerability with potential impact | **SHOULD FIX** - Review and plan remediation |
| **MEDIUM** | Moderate vulnerability | Monitor and fix in next release |
| **LOW** | Minor vulnerability or low impact | Fix when convenient |

### Common Findings and Resolutions

**Finding**: `CVE-2023-XXXXX in libssl1.1`
**Resolution**: Update base image to newer version with patched OpenSSL

**Finding**: `CVE-2024-XXXXX in npm package`
**Resolution**: Update package version in package.json

**Finding**: `CVE-2022-XXXXX in Rust dependency`
**Resolution**: Update Cargo.toml dependencies

**Finding**: False positive in documentation files
**Resolution**: Add to `.trivyignore` with justification comment

---

## Appendix C: Docker Hub Multi-Platform Verification

### Visual Indicators of Success

When viewing `crewchief/maproom-mcp:1.1.10-rc1` on Docker Hub:

```
Tag: 1.1.10-rc1

Architectures:
  📦 linux/amd64
  📦 linux/arm64

Digest: sha256:abc123def456...

Last Pushed: 2025-10-30 14:23:15 UTC

Compressed Size:
  AMD64: 165 MB
  ARM64: 168 MB

Layers: 10
```

### Manifest Inspection Details

The `docker manifest inspect` command should show:

- **schemaVersion**: 2
- **mediaType**: `application/vnd.docker.distribution.manifest.list.v2+json`
- **manifests**: Array with 2 entries (AMD64, ARM64)
- Each manifest has:
  - Unique digest
  - Platform object with correct architecture and OS
  - Size in bytes

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-30 | integration-tester | Initial test plan created |

---

**End of Test Plan**
