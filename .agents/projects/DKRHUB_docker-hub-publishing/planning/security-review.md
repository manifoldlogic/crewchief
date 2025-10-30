# DKRHUB: Docker Hub Publishing - Security Review

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Security Review

## Executive Summary

This security review analyzes the risks and mitigations for publishing Docker images to Docker Hub via GitHub Actions. The primary security concerns are:

1. **Credential Management** - Protecting Docker Hub access tokens
2. **Supply Chain Security** - Ensuring image integrity and provenance
3. **Container Runtime Security** - Hardening the container environment
4. **Access Control** - Limiting who can trigger releases
5. **Vulnerability Management** - Detecting and remediating security issues

**Overall Risk Level**: Medium (acceptable with mitigations in place)

## Threat Model

### Assets

1. **Docker Hub Account** (`crewchief`)
   - Value: Ability to publish official images
   - Impact if compromised: Malicious images distributed to users

2. **Docker Hub Access Token** (stored in GitHub Secrets)
   - Value: Write access to Docker Hub repository
   - Impact if compromised: Unauthorized image pushes

3. **GitHub Repository** (`danielbushman/crewchief`)
   - Value: Source code and CI/CD pipelines
   - Impact if compromised: Malicious code in releases

4. **Published Docker Images**
   - Value: Trusted by users for deployment
   - Impact if compromised: Widespread security breach

5. **User Systems**
   - Value: Run Maproom MCP for code search
   - Impact if compromised: Code theft, system compromise

### Threat Actors

1. **External Attackers**
   - Goal: Inject malware into images
   - Capability: Network attacks, credential theft
   - Likelihood: Low-Medium

2. **Malicious Contributors**
   - Goal: Backdoor in code or images
   - Capability: Submit pull requests
   - Likelihood: Low

3. **Compromised Dependencies**
   - Goal: Supply chain attack via npm/Docker
   - Capability: Malicious package updates
   - Likelihood: Low-Medium

4. **Insider Threats**
   - Goal: Abuse access privileges
   - Capability: Direct repository/account access
   - Likelihood: Very Low

### Attack Vectors

```
┌─────────────────────────────────────────────────────────────┐
│                     Attack Vectors                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. GitHub Account Compromise                              │
│     │                                                       │
│     ├─► Push malicious tag → Trigger workflow             │
│     ├─► Modify workflow YAML → Inject backdoor            │
│     └─► Access GitHub Secrets → Steal Docker Hub token    │
│                                                             │
│  2. Docker Hub Account Compromise                          │
│     │                                                       │
│     ├─► Overwrite legitimate images → Distribute malware  │
│     └─► Delete images → Deny service                      │
│                                                             │
│  3. Supply Chain Attack                                    │
│     │                                                       │
│     ├─► Malicious npm package → Bundled in image          │
│     ├─► Compromised base image → Inherited vulnerabilities│
│     └─► Typosquatting → Wrong dependency installed        │
│                                                             │
│  4. Man-in-the-Middle Attack                               │
│     │                                                       │
│     ├─► Intercept Docker push → Replace image             │
│     └─► DNS poisoning → Pull from malicious registry      │
│                                                             │
│  5. Insider Threat                                         │
│     │                                                       │
│     ├─► Intentional backdoor → Merge malicious PR         │
│     └─► Accidental secret exposure → Leak credentials     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Security Controls

### 1. Credential Management

#### 1.1 Docker Hub Access Token

**Current Implementation**:
- Access token stored in GitHub Secrets: `DOCKERHUB_TOKEN`
- Username stored in GitHub Secrets: `DOCKERHUB_USERNAME`
- Secrets never exposed in logs or outputs

**Best Practices**:

1. **Use Access Tokens, Not Passwords**
   - ✅ Use Docker Hub access token (not account password)
   - ✅ Token can be revoked independently
   - ✅ Token has limited scope (read/write, not delete)

2. **Limit Token Permissions**
   ```
   Docker Hub Token Settings:
   - Scope: Read/Write only (not Delete)
   - Repositories: Specific to crewchief/maproom-mcp
   - Expiration: 1 year (set calendar reminder)
   ```

3. **Rotate Tokens Regularly**
   - Schedule: Every 12 months
   - Process:
     1. Create new token in Docker Hub
     2. Update GitHub Secret
     3. Test workflow
     4. Revoke old token
   - Document rotation date in README

4. **Enable Two-Factor Authentication (2FA)**
   - ✅ Enable 2FA on Docker Hub account
   - ✅ Enable 2FA on GitHub account
   - Use authenticator app (not SMS)

**Risk Level**: Low (with controls)

#### 1.2 GitHub Secrets Security

**Current Implementation**:
- Secrets stored encrypted at rest
- Only accessible to workflows in same repository
- Not exposed in forks or pull requests from forks

**Best Practices**:

1. **Limit Secret Access**
   ```yaml
   # Only specific workflow can access secrets
   permissions:
     contents: read
     packages: write  # Minimal permissions
   ```

2. **Audit Secret Usage**
   - Review GitHub audit log regularly
   - Monitor workflow runs for unexpected secret access
   - Alert on secret modifications

3. **Prevent Secret Leakage**
   ```yaml
   # DON'T: Expose secrets in output
   - run: echo "Token: ${{ secrets.DOCKERHUB_TOKEN }}"  # ❌ NEVER

   # DO: Use secrets only in secure contexts
   - uses: docker/login-action@v3
     with:
       password: ${{ secrets.DOCKERHUB_TOKEN }}  # ✅ Secure
   ```

**Risk Level**: Low

### 2. Supply Chain Security

#### 2.1 Base Image Security

**Current Base Image**: `node:20-alpine`

**Verification Steps**:

1. **Use Official Images**
   - ✅ `node:20-alpine` is official Node.js image
   - ✅ Maintained by Docker Official Images team
   - ✅ Regularly updated with security patches

2. **Pin to Specific Digest** (Recommended)
   ```dockerfile
   # Current (good)
   FROM node:20-alpine

   # Better (pinned to digest)
   FROM node:20-alpine@sha256:abc123...

   # Best (pinned + regular updates)
   FROM node:20-alpine@sha256:abc123...  # Updated 2025-10-29
   ```

3. **Monitor Base Image Updates**
   - Use Renovate or Dependabot
   - Auto-update base image monthly
   - Test updates in CI before merging

4. **Verify Image Provenance**
   ```bash
   # Inspect image signature
   docker trust inspect node:20-alpine

   # Verify publisher
   docker image inspect node:20-alpine \
     --format='{{.RepoDigests}}'
   ```

**Risk Level**: Low (official images)

#### 2.2 Dependency Security

**npm Dependencies**: `pg`, `pino`, `zod`, `execa`

**Verification Steps**:

1. **Audit Dependencies**
   ```bash
   # Run on every build
   npm audit --audit-level=high --prod

   # Fail build on critical/high vulnerabilities
   npm audit --audit-level=high --audit-level=critical
   ```

2. **Pin Dependency Versions**
   ```json
   // package.json
   {
     "dependencies": {
       "pg": "8.11.3",         // Exact version
       "pino": "8.16.2",       // Exact version
       "zod": "3.22.4",        // Exact version
       "execa": "8.0.1"        // Exact version
     }
   }
   ```

3. **Use Lock Files**
   - Commit `package-lock.json` or `pnpm-lock.yaml`
   - Ensures reproducible builds
   - Prevents unexpected updates

4. **SBOM Generation** (Software Bill of Materials)
   ```bash
   # Generate SBOM with Syft
   syft crewchief/maproom-mcp:1.1.10 -o json > sbom.json

   # Scan SBOM with Grype
   grype sbom:./sbom.json
   ```

**Risk Level**: Medium (mitigated with audits)

#### 2.3 Build Environment Security

**GitHub Actions Runners**:

1. **Use Official Actions**
   ```yaml
   # ✅ Official actions from trusted sources
   - uses: actions/checkout@v4
   - uses: docker/setup-buildx-action@v3
   - uses: docker/login-action@v3
   - uses: docker/build-push-action@v5

   # ❌ Avoid third-party actions unless verified
   # - uses: random-user/sketchy-action@main  # Dangerous
   ```

2. **Pin Action Versions to SHA** (Recommended)
   ```yaml
   # Current (good)
   - uses: actions/checkout@v4

   # Better (pinned to SHA)
   - uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11  # v4.1.1

   # Why: Protects against tag hijacking
   ```

3. **Limit Workflow Permissions**
   ```yaml
   permissions:
     contents: read      # Read code only
     packages: write     # Write to container registry
     security-events: write  # Upload security scan results
   ```

4. **Isolate Workflow Steps**
   - Each step runs in clean environment
   - No state persists between steps
   - Secrets only available to steps that need them

**Risk Level**: Low (GitHub-hosted runners are secure)

### 3. Image Security

#### 3.1 Container Hardening

**Current Implementation**:

1. **Non-Root User** ✅
   ```dockerfile
   # Switch to non-root user
   USER node  # uid 1000
   ```

2. **Minimal Base Image** ✅
   ```dockerfile
   FROM node:20-alpine  # 180MB vs 1GB for full node image
   ```

3. **Multi-Stage Build** ✅
   ```dockerfile
   FROM node:20-alpine AS builder
   # ... build steps ...

   FROM node:20-alpine
   # ... runtime only ...
   ```

**Additional Hardening** (Recommended):

1. **Read-Only Filesystem** (where possible)
   ```yaml
   # docker-compose.yml
   services:
     maproom-mcp:
       read_only: true
       tmpfs:
         - /tmp
         - /app/logs  # Writable volume
   ```

2. **Drop Capabilities**
   ```yaml
   # docker-compose.yml
   services:
     maproom-mcp:
       cap_drop:
         - ALL
       cap_add:
         - NET_BIND_SERVICE  # Only if needed
   ```

3. **Security Options**
   ```yaml
   # docker-compose.yml
   services:
     maproom-mcp:
       security_opt:
         - no-new-privileges:true  # Prevent privilege escalation
   ```

**Risk Level**: Low (good hardening)

#### 3.2 Vulnerability Scanning

**Trivy Integration** (in GitHub Actions):

```yaml
- name: Run Trivy security scan
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: crewchief/maproom-mcp:${{ steps.version.outputs.full }}
    format: 'sarif'
    output: 'trivy-results.sarif'
    severity: 'CRITICAL,HIGH'
    exit-code: 1  # Fail build on findings
```

**Vulnerability Response Process**:

1. **Critical Vulnerabilities**:
   - Block release immediately
   - Investigate and patch within 24 hours
   - Release hotfix if in production

2. **High Vulnerabilities**:
   - Document and plan fix
   - Patch within 1 week
   - Include in next regular release

3. **Medium/Low Vulnerabilities**:
   - Track in issue tracker
   - Fix in regular update cycle
   - Monitor for exploitation

**Risk Level**: Low (automated scanning)

#### 3.3 Image Signing and Verification

**Docker Content Trust** (Future Enhancement):

1. **Enable DCT**
   ```bash
   # Sign images on push
   export DOCKER_CONTENT_TRUST=1
   docker push crewchief/maproom-mcp:1.1.10
   ```

2. **Require Signed Images**
   ```bash
   # Users verify signature on pull
   export DOCKER_CONTENT_TRUST=1
   docker pull crewchief/maproom-mcp:1.1.10
   ```

**Cosign** (Alternative, Recommended):

```yaml
# GitHub Actions workflow
- name: Install Cosign
  uses: sigstore/cosign-installer@main

- name: Sign image
  run: |
    cosign sign --key cosign.key \
      crewchief/maproom-mcp:${{ steps.version.outputs.full }}

# Users verify
# cosign verify --key cosign.pub crewchief/maproom-mcp:1.1.10
```

**Status**: Not implemented (P2 - nice to have)

**Risk Level**: Medium (no signing currently)

### 4. Access Control

#### 4.1 GitHub Repository Access

**Branch Protection**:

1. **Protect main branch**
   - ✅ Require pull request reviews
   - ✅ Require status checks to pass
   - ✅ Enforce branch up to date before merge
   - ✅ Restrict who can push to main

2. **Protect version tags**
   ```
   GitHub Settings → Branches → Tag protection
   - Pattern: v*.*.*
   - Protected: Yes
   - Allow force pushes: No
   - Allow deletions: No
   ```

3. **Require 2FA for collaborators**
   - All collaborators must enable 2FA
   - Enforce via organization policy

**Risk Level**: Low

#### 4.2 Release Authorization

**Who Can Trigger Releases**:

1. **Automatic (on tag push)**:
   - Only repository maintainers can push tags
   - Tags must follow pattern `v*.*.*`
   - Protected tags cannot be overwritten

2. **Manual (workflow_dispatch)**:
   - Only repository maintainers can trigger
   - Requires explicit approval
   - Logs all manual triggers

**Release Checklist** (manual):
- [ ] Code review completed
- [ ] Tests pass in CI
- [ ] Security scan clean
- [ ] Version number correct
- [ ] CHANGELOG updated
- [ ] Documentation updated
- [ ] Authorized by maintainer

**Risk Level**: Low

### 5. Runtime Security

#### 5.1 Container Isolation

**Network Isolation**:
```yaml
# docker-compose.yml
networks:
  maproom-network:
    driver: bridge
    internal: false  # Allow external access (for Docker Hub pulls)
```

**Volume Isolation**:
```yaml
volumes:
  maproom-data:
    driver: local
    # Data persists but isolated from host filesystem
```

**Process Isolation**:
- Containers run in separate namespaces
- PID, network, mount, UTS namespaces isolated
- User namespaces (UID mapping) available

**Risk Level**: Low

#### 5.2 Secrets Management

**Environment Variables**:
```yaml
environment:
  DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
  OPENAI_API_KEY: ${OPENAI_API_KEY:-}  # From host environment, not in image
```

**Best Practices**:
1. Never bake secrets into images
2. Pass secrets via environment variables
3. Use Docker secrets (Swarm mode) or external secrets manager
4. Rotate database passwords regularly

**Risk Level**: Low (no secrets in image)

#### 5.3 Logging and Monitoring

**Log Collection**:
```yaml
volumes:
  maproom-logs:/app/logs
```

**What to Log**:
- Container start/stop events
- Health check failures
- Authentication attempts
- Errors and exceptions
- Resource usage metrics

**What NOT to Log**:
- Passwords or API keys
- User code content (privacy)
- Full database queries (may contain sensitive data)

**Risk Level**: Low

## Security Testing

### Automated Security Tests

1. **Dependency Scanning** (npm audit)
   - Runs on every build
   - Fails on critical/high vulnerabilities
   - Reports to GitHub Security tab

2. **Container Scanning** (Trivy)
   - Scans base image and layers
   - Detects OS and application vulnerabilities
   - Generates SARIF report for GitHub

3. **Secrets Scanning** (GitHub Secret Scanning)
   - Automatically scans commits
   - Alerts if secrets detected in code
   - Prevents accidental credential exposure

4. **Static Analysis** (CodeQL - optional)
   ```yaml
   # .github/workflows/codeql.yml
   - uses: github/codeql-action/init@v2
     with:
       languages: javascript, typescript
   ```

### Manual Security Testing

**Pre-Release Security Checklist**:

- [ ] Run manual security audit
- [ ] Verify no hardcoded secrets in code
- [ ] Test image with non-root user
- [ ] Verify file permissions restrictive
- [ ] Test with minimal capabilities
- [ ] Check for unnecessary network exposure
- [ ] Verify logs don't contain secrets
- [ ] Test with read-only filesystem
- [ ] Scan for malware (ClamAV)
- [ ] Review SBOM for suspicious packages

### Penetration Testing

**Scope** (for future):
1. Attempt to escape container
2. Try to access Docker socket
3. Test privilege escalation
4. Network-based attacks (XSS, CSRF, etc.)
5. Supply chain attack simulation

**Frequency**: Annual or after major changes

**Risk Level**: Low (not critical for MVP)

## Incident Response Plan

### Security Incident Classification

**P0 - Critical**:
- Confirmed malware in published image
- Docker Hub account compromised
- Active exploitation of vulnerability

**P1 - High**:
- Critical vulnerability discovered
- Unauthorized access to GitHub repository
- Secrets leaked publicly

**P2 - Medium**:
- High vulnerability discovered
- Suspicious activity detected
- Supply chain risk identified

**P3 - Low**:
- Medium/low vulnerability
- Security best practice violation
- Audit finding

### Incident Response Steps

**Detection**:
1. Monitor GitHub Security alerts
2. Monitor Docker Hub activity logs
3. Watch for user reports
4. Review Trivy scan results

**Response** (P0/P1):
1. **Contain**: Immediately unpublish affected images
2. **Investigate**: Determine scope and impact
3. **Notify**: Alert users via GitHub issue and npm advisory
4. **Remediate**: Fix vulnerability and publish clean image
5. **Document**: Post-mortem analysis

**Recovery**:
1. Publish fixed version (e.g., v1.1.11)
2. Document fix in security advisory
3. Update CHANGELOG and README
4. Conduct retrospective

**Communication**:
```markdown
## Security Advisory: Vulnerability in v1.1.10

**Severity**: High
**Affected Versions**: 1.1.10
**Fixed In**: 1.1.11

### Description
[Describe vulnerability]

### Impact
[Describe potential impact]

### Remediation
Update to v1.1.11 immediately:
\`\`\`bash
MAPROOM_VERSION=1.1.11 npx @crewchief/maproom-mcp start
\`\`\`

### Timeline
- 2025-10-29 10:00 UTC: Vulnerability discovered
- 2025-10-29 11:00 UTC: Fix released
- 2025-10-29 12:00 UTC: Advisory published
```

## Compliance Considerations

### Open Source Licensing

**Image Licenses**:
- Maproom MCP: MIT License
- Node.js: MIT License
- Alpine Linux: MIT-style licenses
- Dependencies: Various (checked in SBOM)

**Compliance**:
- All licenses compatible with MIT
- No GPL/AGPL dependencies (copyleft)
- License file included in image

### Privacy Considerations

**Data Processing**:
- Code indexed locally (not sent to cloud)
- Embeddings generated locally (Ollama)
- Database stored in local Docker volume
- No telemetry or analytics sent

**User Privacy**:
- No user data collected
- No phone-home behavior
- No external network calls (except npm/Docker Hub)

**GDPR/Privacy Compliance**: Not applicable (no personal data processed)

## Security Roadmap

### Implemented (v1.1.10)

- [x] Docker Hub access tokens (not passwords)
- [x] GitHub Secrets for credentials
- [x] Non-root user in container
- [x] Multi-stage build (minimal image)
- [x] Trivy vulnerability scanning
- [x] npm audit on build
- [x] Branch protection
- [x] Tag protection
- [x] 2FA on accounts

### Planned (v1.2.0)

- [ ] Pin base image to digest
- [ ] SBOM generation (Syft)
- [ ] Cosign image signing
- [ ] Renovate/Dependabot for updates
- [ ] Read-only filesystem
- [ ] Drop capabilities
- [ ] Security options (no-new-privileges)

### Future (v2.0.0+)

- [ ] Docker Content Trust (DCT)
- [ ] SLSA provenance
- [ ] Annual penetration testing
- [ ] Security bug bounty program
- [ ] SOC 2 compliance (if enterprise)

## Security Metrics

### Key Performance Indicators

| Metric | Target | Current |
|--------|--------|---------|
| Critical vulns in published images | 0 | 0 |
| High vulns in published images | <5 | TBD |
| Days to patch critical vuln | <1 | N/A |
| Days to patch high vuln | <7 | N/A |
| % images with signatures | 100% | 0% (planned) |
| Security incidents per year | 0 | 0 |
| Token rotation frequency | 12 months | TBD |

### Monitoring

**GitHub Actions**:
- Workflow success rate: >95%
- Security scan failures: 0
- Secrets access anomalies: 0

**Docker Hub**:
- Unauthorized push attempts: 0
- Account login failures: <10/month
- API rate limit hits: 0

**User Reports**:
- Security issues reported: Track in GitHub Issues
- False positive rate: <10%

## Conclusion

**Overall Security Posture**: Good

**Key Strengths**:
- Secure credential management
- Automated vulnerability scanning
- Non-root containers
- Minimal attack surface

**Key Weaknesses**:
- No image signing (planned)
- Base image not pinned to digest
- No SBOM generation yet

**Recommendation**: Proceed with release. Security measures are adequate for v1.1.10. Implement planned enhancements in v1.2.0.

---

**Status**: Security review complete, approved for implementation
