# Security Review: CI/CD Workflow Optimization

## Executive Summary

**Security Posture**: ✅ **Safe to Ship**

The proposed CI/CD optimizations use industry-standard GitHub Actions patterns with appropriate security controls. No custom code execution, no elevated privileges, and proper secret management throughout.

**Risk Level**: Low - Standard GitHub Actions security model

---

## Architecture Security Analysis

### Workflow Permissions

**Current State**: Minimal permissions per workflow

**Proposed Changes**: No permission changes required

**Analysis**:

```yaml
# Existing permission model (keep as-is)
permissions:
  contents: read        # Read repo code
  id-token: write      # npm provenance (optional)
  packages: write      # Docker publish (maproom-mcp only)
```

**Security Controls**:
- ✅ Read-only by default
- ✅ Write permissions only where needed (publish jobs)
- ✅ No workflow-level `permissions: write-all`
- ✅ GITHUB_TOKEN scoped per job

**Recommendation**: Keep existing permission model - already follows principle of least privilege.

---

### Secret Management

**Secrets Used**:

| Secret | Purpose | Scope | Exposure Risk |
|--------|---------|-------|---------------|
| `NPM_TOKEN` | npm publish auth | publish jobs only | Low - scoped token |
| `DOCKERHUB_USERNAME` | Docker Hub auth | Docker job only | Low - username only |
| `DOCKERHUB_TOKEN` | Docker Hub auth | Docker job only | Low - personal access token |
| `VSCE_PAT` | VS Code Marketplace (future) | Extension publish only | Low - scoped PAT |
| `OVSX_PAT` | Open VSX (future) | Extension publish only | Low - scoped PAT |

**Security Measures**:

1. **Never logged**:
```yaml
# Good - uses env var, not echoed
env:
  NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

# Bad - would log secret (we don't do this)
run: echo "Token: ${{ secrets.NPM_TOKEN }}"
```

2. **Scoped to specific jobs**:
```yaml
# Secrets only in jobs that need them
jobs:
  build-rust:
    # No secrets needed

  publish-npm:
    env:
      NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}  # Only here
```

3. **No secrets in artifacts**:
- Build artifacts don't contain secrets
- Tokens never passed as build args
- No `.env` files in uploaded artifacts

**Recommendations**:
- ✅ Current secret usage is secure
- ✅ Maintain job-level scoping
- ✅ Consider OIDC for future cloud deployments (more secure than PATs)

---

### Reusable Workflow Security

**Security Model**: Reusable workflows run in caller's context

**Implications**:

```yaml
# Caller workflow (release-cli.yml)
jobs:
  build-rust:
    uses: ./.github/workflows/reusable-rust-build.yml
    # Runs with caller's permissions
    # Has access to caller's secrets (if explicitly passed)
    # Runs in caller's repository context
```

**Potential Risks**:

1. **Malicious reusable workflow** (mitigated):
   - ✅ All reusables in same repository (not external)
   - ✅ Code review required for changes
   - ✅ No user input in workflow commands

2. **Secrets leaking to reusable** (mitigated):
   - ✅ Secrets not automatically passed to reusables
   - ✅ Must be explicitly passed with `secrets: inherit`
   - ✅ Our reusables don't need secrets (build only)

3. **Workflow injection** (mitigated):
   - ✅ No `${{ github.event.inputs.* }}` in run commands
   - ✅ All inputs validated/typed
   - ✅ No dynamic workflow generation

**Recommendations**:
- ✅ Keep reusables in same repo (trusted code)
- ✅ Don't pass secrets unless absolutely necessary
- ✅ Validate all inputs in reusable workflows

---

### Artifact Security

**Artifact Contents**:
- Rust binaries (no sensitive data)
- TypeScript dist/ (compiled code)
- npm packages (.tgz)
- Docker images (public)
- VSIX files (future - public extensions)

**Security Considerations**:

1. **Artifact Access**:
   - ✅ Accessible within workflow run only
   - ✅ Requires authentication to download
   - ✅ Deleted after retention period (7-90 days)

2. **Artifact Integrity**:
   - ✅ SHA checksums generated automatically
   - ✅ Artifacts immutable once uploaded
   - ✅ Can verify artifact wasn't tampered with

3. **Sensitive Data**:
   - ✅ No secrets in build artifacts
   - ✅ No API keys, tokens, or credentials
   - ✅ No user data or PII
   - ✅ Source code only (already public)

**Recommendations**:
- ✅ Current artifact usage is secure
- ✅ Keep retention periods reasonable (7 days for builds, 90 for releases)
- ✅ Don't upload anything with secrets

---

### Caching Security

**Cache Types**:
1. **Rust dependencies** (Swatinem/rust-cache)
2. **pnpm store** (actions/cache)
3. **Docker layers** (buildx GHA cache)

**Security Considerations**:

1. **Cache Poisoning**:
   - **Risk**: Malicious actor corrupts cache
   - **Likelihood**: Low (requires repo write access)
   - **Mitigation**:
     ```yaml
     # Cache keys include lock file hash
     key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
     # Attacker would need to modify lock file (caught in PR review)
     ```

2. **Cache Leaking Secrets**:
   - **Risk**: Secrets cached and exposed
   - **Likelihood**: Very low (we don't cache anything with secrets)
   - **Mitigation**:
     - ✅ Only caching dependencies (public packages)
     - ✅ No build outputs with secrets
     - ✅ Rust cache excludes target/debug (only release)

3. **Cache Access**:
   - ✅ Scoped to repository
   - ✅ Requires GitHub authentication
   - ✅ Not accessible outside workflow runs

**Recommendations**:
- ✅ Current caching is secure
- ✅ Cache keys properly scoped (lock file hash)
- ✅ No sensitive data in cached artifacts

---

## Action Dependencies Security

### Trusted Actions (Official GitHub)

Used actions from `actions/*` organization:
- ✅ `actions/checkout@v4` - Official, widely trusted
- ✅ `actions/setup-node@v4` - Official
- ✅ `actions/cache@v4` - Official
- ✅ `actions/upload-artifact@v4` - Official
- ✅ `actions/download-artifact@v4` - Official

**Security**: High trust - maintained by GitHub

---

### Community Actions (Vetted)

**Swatinem/rust-cache@v2**:
- ✅ 1.3k+ stars, widely used in Rust ecosystem
- ✅ Active maintenance
- ✅ Source code reviewed
- ✅ No secret access required

**docker/login-action@v3**:
- ✅ Official Docker GitHub Action
- ✅ 1k+ stars
- ✅ Maintained by Docker, Inc.

**docker/build-push-action@v5**:
- ✅ Official Docker GitHub Action
- ✅ 3k+ stars
- ✅ Industry standard for Docker builds

**docker/metadata-action@v5**:
- ✅ Official Docker GitHub Action
- ✅ 800+ stars
- ✅ Generates tags/labels only (no secret access)

**docker/setup-buildx-action@v3**:
- ✅ Official Docker GitHub Action
- ✅ Sets up BuildKit (no secret access)

**docker/setup-qemu-action@v3**:
- ✅ Official Docker GitHub Action
- ✅ Enables multi-platform builds

**dtolnay/rust-toolchain@stable**:
- ✅ Maintained by prominent Rust contributor (dtolnay)
- ✅ Used by Rust Foundation projects
- ✅ 600+ stars

**Security**: Medium-High trust - vetted community actions

---

### Version Pinning Strategy

**Current**: Using major version tags (`@v4`, `@v3`)

**Recommendation**: Continue with major versions

**Rationale**:
- ✅ Automatically get security patches
- ✅ No breaking changes within major version
- ✅ Balance between security and maintenance

**Alternative** (more secure but higher maintenance):
```yaml
# Pin to specific SHA (maximum security)
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11  # v4.1.1
```

**Trade-off**: We prioritize convenience over maximum security (acceptable for our risk profile).

---

## Known Security Gaps

### Gap 1: No Workflow Attestation

**Issue**: Workflow artifacts not signed/attested

**Impact**: Can't cryptographically prove artifact came from our workflow

**Mitigation** (future):
```yaml
# Add Sigstore signing for artifacts
- uses: sigstore/gh-action-sigstore-python@v1
  with:
    inputs: ./dist/*.tgz
```

**Priority**: Low - our artifacts are public, not distributed binaries

---

### Gap 2: No SBOM Generation

**Issue**: No Software Bill of Materials for releases

**Impact**: Can't track dependency vulnerabilities in released artifacts

**Mitigation** (future):
```yaml
# Generate SBOM with syft
- name: Generate SBOM
  uses: anchore/sbom-action@v0
  with:
    artifact-name: sbom.spdx.json
```

**Priority**: Low - dependencies already tracked in lock files

---

### Gap 3: Limited Docker Scanning

**Current**: Only Trivy scan in Docker workflow

**Enhancement** (optional):
```yaml
# Add Snyk scanning
- uses: snyk/actions/docker@master
  with:
    image: ${{ env.DOCKER_HUB_REPO }}:${{ steps.version.outputs.full }}
    args: --severity-threshold=high
```

**Priority**: Low - Trivy sufficient for MVP

---

## VSCode Extension Security (Future)

### Marketplace Publishing Security

**Personal Access Tokens**:

1. **Microsoft Marketplace (VSCE_PAT)**:
   - Scope: `Marketplace (Manage)`
   - Expiration: Set to 90 days (Azure DevOps PAT)
   - Rotation: Manual (update secret quarterly)

2. **Open VSX (OVSX_PAT)**:
   - Scope: Publishing only
   - Expiration: Set to 90 days
   - Rotation: Manual (update secret quarterly)

**Security Measures**:

```yaml
# Only publish if secrets present (fail gracefully)
- name: Publish to VS Code Marketplace
  if: ${{ secrets.VSCE_PAT != '' }}
  run: vsce publish -p ${{ secrets.VSCE_PAT }}

# No token in logs
# No token in artifacts
# Token scoped to publish job only
```

**Recommendations**:
- ✅ Use PATs with minimum required scope
- ✅ Set expiration (90 days)
- ✅ Rotate regularly
- ✅ Monitor marketplace for unauthorized publishes
- 🔄 Consider OAuth2/OIDC when available (not yet supported)

---

### Extension Code Security

**Considerations**:

1. **Extension Permissions**:
   - Review requested permissions in package.json
   - Only request what's needed
   - Document why each permission is required

2. **Bundling**:
   - Use webpack/esbuild to bundle extension
   - Tree-shake unused dependencies
   - Minimize attack surface

3. **Dependencies**:
   - Audit npm dependencies regularly
   - Use `pnpm audit`
   - Update dependencies promptly

**Not in scope for CI/CD project** (extension not built yet).

---

## Compliance and Best Practices

### GitHub Actions Security Best Practices

**Followed** (✅):
- ✅ Minimal permissions (least privilege)
- ✅ Secrets scoped to jobs
- ✅ No secrets in logs/artifacts
- ✅ Pinned action versions (major)
- ✅ Code review for workflow changes
- ✅ No user input in shell commands
- ✅ Trusted actions only

**Not Followed** (but acceptable):
- ⚠️ SHA pinning (using major versions instead)
- ⚠️ Artifact signing (not required for public packages)
- ⚠️ SBOM generation (lock files sufficient)

### Supply Chain Security

**Mitigations in Place**:

1. **Dependency Integrity**:
   - ✅ Lock files committed (pnpm-lock.yaml, Cargo.lock)
   - ✅ `--frozen-lockfile` enforced in CI
   - ✅ Dependency audit in local dev (`pnpm audit`)

2. **Build Integrity**:
   - ✅ Reproducible builds (locked dependencies)
   - ✅ Artifacts checksummed automatically
   - ✅ Same binaries in npm package and Docker image

3. **Publish Integrity**:
   - ✅ npm provenance enabled (OIDC token)
   - ✅ Docker content trust possible (not enforced)
   - ✅ Tagged releases (git tags)

---

## Threat Model

### Threat: Malicious Dependency

**Scenario**: Compromised npm or cargo package in dependencies

**Likelihood**: Low (we use popular, audited packages)

**Impact**: High (could inject malicious code)

**Mitigations**:
- ✅ Lock files prevent unexpected updates
- ✅ `pnpm audit` in local dev
- ✅ Dependabot alerts enabled
- 🔄 Add `pnpm audit` to CI (future enhancement)

---

### Threat: Compromised Secrets

**Scenario**: GitHub PAT or npm token leaked

**Likelihood**: Very low (proper secret management)

**Impact**: High (unauthorized publishes)

**Mitigations**:
- ✅ Secrets never logged
- ✅ Secrets scoped to jobs
- ✅ Token expiration (PATs)
- ✅ 2FA required for GitHub/npm accounts
- 🔄 Rotate PATs quarterly (manual process)

**Detection**:
- Monitor npm/Docker Hub for unauthorized publishes
- GitHub audit log for PAT usage
- Alerts on unusual package downloads

---

### Threat: Workflow Injection

**Scenario**: Attacker injects commands via workflow inputs

**Likelihood**: Very low (no dynamic command execution)

**Impact**: High (arbitrary code execution)

**Mitigations**:
- ✅ No user input in `run:` commands
- ✅ All inputs strongly typed
- ✅ No dynamic workflow generation
- ✅ Code review required for workflow changes

**Example of what we DON'T do**:
```yaml
# UNSAFE (we don't do this)
run: echo "${{ github.event.inputs.version }}"
# Attacker could inject: "; rm -rf /"

# SAFE (what we do)
- id: version
  run: echo "version=$VERSION" >> $GITHUB_OUTPUT
  env:
    VERSION: ${{ github.event.inputs.version }}
# Input sanitized via environment variable
```

---

### Threat: Artifact Tampering

**Scenario**: Attacker modifies artifact between jobs

**Likelihood**: Very low (requires GitHub infrastructure compromise)

**Impact**: Medium (wrong binaries published)

**Mitigations**:
- ✅ Artifacts immutable once uploaded
- ✅ SHA checksums generated automatically
- ✅ Artifact access requires authentication
- 🔄 Add explicit artifact verification (future enhancement)

---

## Recommendations

### Immediate (Ship with MVP)

1. ✅ Use existing permission model (no changes needed)
2. ✅ Keep secrets scoped to jobs
3. ✅ Maintain action version pinning strategy
4. ✅ Continue using trusted actions only

### Short-Term (Post-MVP)

1. 🔄 Add `pnpm audit` to CI workflow
2. 🔄 Document PAT rotation procedure
3. 🔄 Set calendar reminders for quarterly PAT rotation
4. 🔄 Add explicit artifact SHA verification

### Long-Term (Future Enhancements)

1. 🔄 Migrate to OIDC for npm publish (when stable)
2. 🔄 Consider artifact signing (Sigstore)
3. 🔄 Generate SBOM for releases
4. 🔄 Pin actions to SHA (if security posture changes)

---

## Conclusion

**Security Assessment**: ✅ **APPROVED FOR PRODUCTION**

The proposed CI/CD optimization:
- ✅ Follows GitHub Actions security best practices
- ✅ Uses appropriate security controls
- ✅ Mitigates identified threats
- ✅ No elevated risk vs current workflows
- ✅ Known gaps are low priority

**Can ship without security concerns.**

**Next**: Execution plan with phased rollout.
