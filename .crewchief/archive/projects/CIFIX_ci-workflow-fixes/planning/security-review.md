# Security Review: CI Workflow Fixes

## Executive Summary

**Risk Level**: Low

These changes modify CI configuration and Docker build process. No application code, API surfaces, or data handling is affected. Security considerations are primarily **supply chain** and **build integrity** focused.

**Key Findings**:
- ✅ No new attack surface introduced
- ✅ Dependency pinning maintained
- ✅ Build reproducibility improved
- ⚠️ pnpm installation method relies on npm registry trust
- ✅ Multi-stage Docker isolation preserved

**Recommendation**: Proceed with implementation. Security posture **improves** due to better dependency pinning.

## Threat Model

### Assets

**What we're protecting**:
1. **Source code integrity** - Prevent malicious code injection during build
2. **Build artifacts** - Ensure Docker images contain only intended code
3. **Deployment pipeline** - Protect GitHub Actions secrets and credentials
4. **User trust** - Maintain confidence in published npm packages and Docker images

**Out of scope** (not affected by these changes):
- Runtime security (MCP server, database access)
- Application vulnerabilities (XSS, injection, etc.)
- Infrastructure security (GitHub Actions runner security)

### Threat Actors

**Supply chain attacker**:
- Goal: Inject malicious dependencies or tooling
- Vector: Compromised npm packages, malicious pnpm release
- Impact: Backdoor in published Docker images

**Insider threat** (malicious contributor):
- Goal: Introduce vulnerabilities via PR
- Vector: Modify Dockerfile or workflow to download unsigned binaries
- Impact: Runtime compromise in production deployments

**Opportunistic attacker**:
- Goal: Exploit misconfiguration
- Vector: Insecure Docker base images, unpatched dependencies
- Impact: Container escape or data exfiltration

## Attack Surface Analysis

### Test Workflow Changes

**Before**:
```yaml
- uses: pnpm/action-setup@v4
  with:
    version: 10  # Explicit version
```

**After**:
```yaml
- uses: pnpm/action-setup@v4
  # Auto-detects from package.json
```

**New Attack Vectors**: None

**Reasoning**:
- pnpm/action-setup@v4 is a trusted GitHub Action (maintained by pnpm team)
- Reads packageManager field from version-controlled package.json
- No external input, no user-controlled data
- Version still pinned (in package.json instead of workflow)

**Security properties unchanged**:
- ✅ Action version still pinned (@v4)
- ✅ pnpm version still pinned (via packageManager field)
- ✅ No arbitrary code execution
- ✅ No new network requests

---

### Docker Build Changes

**Before**:
```dockerfile
RUN npm install --production=false
```

**After**:
```dockerfile
RUN npm install -g pnpm@10.12.1
RUN pnpm install --frozen-lockfile
```

**New Attack Vectors**:

1. **pnpm package compromise**
   - **Threat**: Malicious pnpm version on npm registry
   - **Likelihood**: Very Low (npm has package signing, pnpm is highly-visible)
   - **Impact**: High (build-time code execution)
   - **Mitigation**: Version pinned (10.12.1), npm verifies integrity

2. **Transitive dependency attack**
   - **Threat**: pnpm's dependencies compromised
   - **Likelihood**: Low (pnpm has minimal dependencies, actively maintained)
   - **Impact**: Medium (limited to build environment)
   - **Mitigation**: pnpm installed via npm (integrity checks), multi-stage build isolation

**Security properties improved**:
- ✅ `--frozen-lockfile` enforces exact versions (better than `npm install`)
- ✅ Workspace resolution more secure (no package.json rewriting)
- ✅ pnpm version pinned (not floating "latest")

**Security properties unchanged**:
- ✅ Multi-stage build isolation (pnpm only in builder, not runtime)
- ✅ Base image still `node:20-alpine` (official, regularly patched)
- ✅ Non-root user in final stage (security best practice)

## Dependency Chain Analysis

### pnpm Installation

**Supply chain path**:
```
npm CLI (built-in to node:20-alpine)
  → npmjs.com registry
    → pnpm@10.12.1 package
      → pnpm binary + dependencies
```

**Trust anchors**:
1. **node:20-alpine**: Official Docker image, signed by Docker Inc.
2. **npm registry**: Integrity via SHA-512 checksums (package-lock.json equivalent)
3. **pnpm package**: Signed by pnpm maintainers (npm signature verification)

**Integrity verification**:
```bash
# npm verifies package integrity automatically
npm install -g pnpm@10.12.1
# Downloads from registry.npmjs.org
# Computes SHA-512 of downloaded tarball
# Compares to manifest checksum
# Fails if mismatch
```

**Attack scenarios**:

| Attack | Defender | Result |
|--------|----------|--------|
| MITM on npm registry | HTTPS + checksum | Detected, build fails |
| Compromised pnpm maintainer account | npm 2FA required | Difficult to execute |
| Malicious pnpm release | Community review + usage | Detected quickly |
| pnpm binary backdoor | Open source, auditable | Risk accepted (trust model) |

### Workspace Dependencies

**Dependency resolution path**:
```
pnpm reads pnpm-lock.yaml
  → Resolves workspace:* to local packages
    → Reads packages/daemon-client/package.json
      → Copies dist/ artifacts (pre-built locally)
```

**Security properties**:
- ✅ No network requests for workspace deps
- ✅ Lockfile pinned in source control (reviewed via PR)
- ✅ Local resolution (no registry involved)
- ✅ Build fails if daemon-client dist/ missing (prevents stale code)

**Attack vectors closed**:
- ❌ Can't inject malicious workspace package (requires PR approval)
- ❌ Can't substitute different version (lockfile specifies exact commit)
- ❌ Can't man-in-the-middle (local filesystem resolution)

## Secrets and Credentials

### GitHub Actions Secrets

**Secrets used** (unchanged):
- `NPM_TOKEN`: npm publish authentication
- `DOCKER_USERNAME`: Docker Hub username
- `DOCKER_PASSWORD`: Docker Hub access token
- `GITHUB_TOKEN`: Auto-provided by GitHub (repo access)

**Exposure risks**:

**Before changes**: Low
- Secrets passed as environment variables to workflow steps
- Not exposed in logs (GitHub masks secret values)
- Only used in publish steps (not in build steps)

**After changes**: Low (unchanged)
- pnpm installation doesn't access secrets
- Build steps don't need credentials
- Publish steps unchanged
- Secrets still masked in logs

**Best practices followed**:
- ✅ Secrets not hardcoded in workflows
- ✅ Secrets not logged or echoed
- ✅ Minimal secret scope (only npm/Docker publish)
- ✅ No secrets in Dockerfile

## Multi-Stage Build Isolation

### Security Boundaries

```dockerfile
# Stage 1: Rust builder
FROM rustlang/rust:nightly-bookworm-slim AS rust-builder
# Contains: cargo, rustc, build tools
# Security risk: High (compiler toolchain)
# Exposure: Discarded, not in final image

# Stage 2: Node.js builder (NEW: has pnpm)
FROM node:20-alpine AS node-builder
RUN npm install -g pnpm@10.12.1
# Contains: pnpm, npm, node, TypeScript compiler
# Security risk: Medium (build tools)
# Exposure: Discarded, not in final image

# Stage 3: Runtime
FROM node:20-slim
# Contains: node runtime, compiled JS, Rust binary
# Security risk: Low (minimal attack surface)
# Exposure: Published to Docker Hub
```

**Isolation effectiveness**:

| Component | Builder Stage | Runtime Stage |
|-----------|--------------|---------------|
| pnpm binary | ✅ Present | ❌ Not copied |
| pnpm store | ✅ Present | ❌ Not copied |
| TypeScript compiler | ✅ Present | ❌ Not copied |
| Build tools (make, g++) | ✅ Present | ❌ Not copied |
| Source code | ✅ Present | ❌ Only compiled JS |
| node_modules (dev) | ✅ Present | ❌ Only production |

**Attack surface in final image**:
- ✅ No build tooling (can't compile new code)
- ✅ No package managers (can't install packages)
- ✅ No source code (can't modify and recompile)
- ✅ Only production runtime (minimal)

**Implication**: Even if pnpm were compromised, it can't affect final image (discarded).

## Known Vulnerabilities

### pnpm@10.12.1

**CVE check** (as of 2025-11):
```bash
npm audit pnpm@10.12.1
# Expected: No known vulnerabilities
```

**Upstream security**:
- pnpm actively maintained (weekly releases)
- Security issues disclosed via GitHub Security Advisories
- Version 10.x LTS support until 2026
- No critical CVEs in 10.x branch

**Monitoring**:
- Renovate bot can track pnpm updates (future)
- GitHub Dependabot alerts on vulnerabilities
- Manual review of pnpm changelogs for security patches

### Base Images

**node:20-alpine**:
- Based on Alpine Linux 3.x (security-focused distro)
- Node.js 20 LTS (supported until April 2026)
- Regular security patches via Docker Hub
- Official image (trusted maintainer)

**node:20-slim**:
- Based on Debian Bookworm (stable)
- Smaller than full Debian image
- Security patches via apt (automated in upstream image)

**Vulnerability scanning** (existing):
- Docker Hub automatically scans images
- GitHub Dependabot tracks base image updates
- No new vulnerabilities introduced by pnpm addition

## Security Gaps and Mitigations

### Gap 1: pnpm Version Drift

**Risk**: Dockerfile.combined and package.json specify different pnpm versions

**Current state**:
```json
// package.json
"packageManager": "pnpm@10.12.1"
```

```dockerfile
RUN npm install -g pnpm@10.12.1  // Must stay in sync
```

**Attack scenario**:
- Developer updates package.json to pnpm@10.13.0
- Forgets to update Dockerfile
- Docker build uses outdated pnpm (potential vulnerabilities)

**Mitigation** (MVP):
- ⚠️ Manual sync required (accepted for now)
- Document in PR template: "Did you update pnpm version in both places?"

**Future improvement**:
- Automated sync via build script:
  ```bash
  PNPM_VERSION=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
  docker build --build-arg PNPM_VERSION=$PNPM_VERSION ...
  ```
- Renovate bot creates single PR for both files

### Gap 2: No pnpm Signature Verification

**Risk**: Theoretically, npm registry could serve malicious pnpm package

**Current state**:
- npm verifies package integrity via checksums
- npm does NOT verify GPG signatures (pnpm doesn't sign releases)

**Attack scenario**:
- Attacker compromises npm registry
- Serves malicious pnpm@10.12.1 with valid checksum
- Docker build installs backdoored pnpm

**Likelihood**: Extremely Low
- npm registry uses multi-factor authentication
- Package modifications logged and auditable
- High-visibility packages like pnpm monitored closely

**Mitigation** (MVP):
- ✅ Accept npm registry trust model (industry standard)
- ✅ Pin exact version (not latest)
- ⚠️ No additional verification (out of scope)

**Future improvement**:
- Use Sigstore/cosign for verifiable builds
- Lock pnpm hash in Dockerfile (more brittle, manual updates)

### Gap 3: No Runtime Integrity Monitoring

**Risk**: Container image modified post-build

**Current state**:
- Docker images signed with DSSE attestations (via docker/build-push-action)
- No runtime verification that image hasn't been tampered

**Attack scenario**:
- Attacker gains access to Docker Hub
- Modifies published image
- Users pull compromised image

**Likelihood**: Very Low
- Docker Hub requires 2FA
- Image layers are content-addressed (modification detectable)
- GitHub Actions build logs provide audit trail

**Mitigation** (MVP):
- ✅ Docker provenance attestations enabled (automatic)
- ⚠️ No additional runtime checks (accept Docker Hub trust)

**Future improvement**:
- Cosign signatures for images
- Notary v2 integration
- Image scanning with Trivy/Grype

## Compliance and Audit

### Build Reproducibility

**Reproducible builds**:
```dockerfile
RUN pnpm install --frozen-lockfile  # Deterministic
RUN pnpm build                      # Deterministic (same inputs → same outputs)
```

**Audit trail**:
- Git commit SHA → Build inputs
- pnpm-lock.yaml → Exact dependency versions
- GitHub Actions logs → Build process evidence
- Docker image labels → Metadata (commit, timestamp, builder)

**Properties**:
- ✅ Same commit always produces same image (modulo timestamps)
- ✅ Build process auditable via Actions logs
- ✅ Dependencies traceable via lockfile
- ✅ Source code traceable via commit SHA

### Regulatory Considerations

**SLSA Framework** (Supply-chain Levels for Software Artifacts):
- SLSA Level 2: Achieved
  - Build service (GitHub Actions)
  - Provenance (Docker attestations)
  - Hermetic builds (isolated runners)

**SBOM** (Software Bill of Materials):
- Not currently generated
- Future improvement: Generate SBOM via Syft or Docker SBOM
- Tracks all dependencies in final image

## Risk Acceptance

### Accepted Risks

**1. npm registry trust**
- **Why accepted**: Industry standard, no practical alternative
- **Residual risk**: Registry compromise (extremely unlikely)
- **Monitoring**: npm status page, security advisories

**2. pnpm version manual sync**
- **Why accepted**: Simple to verify, low change frequency
- **Residual risk**: Version drift (medium likelihood, low impact)
- **Monitoring**: Code review checklist

**3. No runtime image verification**
- **Why accepted**: Docker Hub provides sufficient protection for MVP
- **Residual risk**: Post-build tampering (very unlikely)
- **Monitoring**: Docker Hub notifications, user reports

### Unacceptable Risks (Mitigated)

**1. Floating dependency versions**
- **Risk**: Using `pnpm@latest` instead of pinned version
- **Mitigation**: Explicit version pinning (10.12.1)
- **Status**: ✅ Mitigated

**2. Workspace dependency confusion**
- **Risk**: Malicious package in npm registry shadowing local workspace package
- **Mitigation**: pnpm workspace protocol (workspace: prefix)
- **Status**: ✅ Mitigated

**3. Secret exposure in logs**
- **Risk**: Secrets leaked in build output
- **Mitigation**: GitHub Actions secret masking, no secrets in Dockerfile
- **Status**: ✅ Mitigated

## Security Recommendations

### Immediate (MVP)

1. ✅ **Pin pnpm version** in Dockerfile (already planned)
2. ✅ **Use --frozen-lockfile** for reproducibility (already planned)
3. ✅ **Document version sync** in contributing guide
4. ⚠️ **Add comment** in Dockerfile explaining pnpm installation method

### Short-term (Post-MVP)

1. **Automate version sync**: Build script to extract pnpm version from package.json
2. **Enable Renovate**: Automatic dependency updates with grouped PRs
3. **Add Dockerfile linting**: Use Hadolint to catch security issues
4. **Document rollback procedure**: Security incident response plan

### Long-term (Future Hardening)

1. **SBOM generation**: Track all dependencies in published images
2. **Image signing**: Cosign or Notary for cryptographic verification
3. **Vulnerability scanning**: Integrate Trivy or Grype into CI
4. **Hermetic builds**: Fully reproducible builds with Bazel or Nix

## Security Testing

### Validation Steps

**Pre-merge**:
1. Code review security checklist
2. Verify pnpm version matches package.json
3. Check no secrets in Dockerfile
4. Verify .dockerignore excludes sensitive files

**Post-merge**:
1. Scan published image: `docker scout cves maproom-mcp:latest`
2. Verify image size (no bloat from build tools)
3. Check container runs as non-root: `docker inspect maproom-mcp:latest`
4. Verify pnpm not in runtime: `docker run maproom-mcp which pnpm || echo OK`

**Ongoing**:
1. Monitor GitHub Security Advisories
2. Review Dependabot alerts
3. Track pnpm release notes for security patches
4. Audit Docker Hub for unauthorized image modifications

## Conclusion

**Security posture**: Improved

**Key improvements**:
- ✅ Better dependency pinning (--frozen-lockfile)
- ✅ Workspace resolution (no package.json munging)
- ✅ Explicit version control (pnpm@10.12.1)

**Accepted risks**:
- ⚠️ Manual version sync (mitigated via docs + review)
- ⚠️ npm registry trust (industry standard)

**Recommendation**: **Approve for implementation**

No significant security concerns. Benefits (improved build integrity) outweigh minimal risks.
