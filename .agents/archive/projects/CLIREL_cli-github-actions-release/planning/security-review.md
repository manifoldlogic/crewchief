# Security Review: CLI GitHub Actions Release Automation

## Threat Model

This project involves automated publishing of npm packages with native binaries. The attack surface includes:

1. **GitHub Actions workflow** - CI/CD pipeline with secrets
2. **npm registry publication** - Package distribution
3. **Git tag triggers** - Workflow activation mechanism
4. **Binary compilation** - Native code execution
5. **Package installation** - End-user execution

## Security Risks by Category

### Supply Chain Security

**Risk 1: Compromised npm token**
- **Severity**: Critical
- **Impact**: Attacker publishes malicious package to `@crewchief/cli`
- **Likelihood**: Low (requires GitHub repo compromise + secrets access)
- **Mitigation**:
  - Store NPM_TOKEN as GitHub secret (encrypted at rest)
  - Limit secret access to specific workflows
  - Use environment protection rules (manual approval for production)
  - Enable npm 2FA on package (publish requires OTP)
  - Monitor npm downloads for anomalies
- **MVP**: Store as secret, document 2FA setup
- **Enterprise**: Environment protection + publish approval

**Risk 2: Malicious code in release**
- **Severity**: Critical
- **Impact**: Users execute compromised binaries
- **Likelihood**: Low (requires compromised developer account or PR approval bypass)
- **Mitigation**:
  - Require signed commits (git commit signing)
  - Branch protection on main (require reviews)
  - Tag protection (restrict who can create release tags)
  - Binary validation in workflow (size, content checks)
  - Reproducible builds (same input → same output)
- **MVP**: Tag protection, validation checks
- **Enterprise**: Signed commits, full reproducibility

**Risk 3: Dependency confusion**
- **Severity**: Medium
- **Impact**: Workflow installs malicious dependency instead of legitimate one
- **Likelihood**: Low (scoped package reduces risk)
- **Mitigation**:
  - Use scoped packages (`@crewchief/*` owned by org)
  - Lock file in repository (pnpm-lock.yaml)
  - Dependency review in PRs
  - npm audit in workflow
- **MVP**: Scoped package, lock file
- **Enterprise**: Private registry for internal dependencies

**Risk 4: Typosquatting during deprecation**
- **Severity**: Medium
- **Impact**: Users install malicious `@crewchief/cli` impersonator
- **Likelihood**: Medium (common attack during migrations)
- **Mitigation**:
  - Pre-register package name before announcement
  - Clear deprecation message with exact new package name
  - Verify package ownership on npm
  - Monitor for similar package names
- **MVP**: Pre-register, clear messaging
- **Enterprise**: Proactive typosquat monitoring service

### Build Security

**Risk 5: Compromised build environment**
- **Severity**: High
- **Impact**: Malicious binaries injected during build
- **Likelihood**: Very low (requires GitHub infrastructure compromise)
- **Mitigation**:
  - Use GitHub-hosted runners (managed infrastructure)
  - Pin action versions (not `@latest`)
  - Minimize third-party actions
  - Validate artifacts before publish
- **MVP**: GitHub-hosted runners, pinned actions
- **Enterprise**: Self-hosted runners in isolated network

**Risk 6: Malicious GitHub Action**
- **Severity**: High
- **Impact**: Workflow compromise, secret exfiltration
- **Likelihood**: Low (using official GitHub actions)
- **Mitigation**:
  - Use only official actions (`actions/*`)
  - Pin to specific commit SHA (not version tag)
  - Review action source code for third-party actions
  - Limit action permissions (GITHUB_TOKEN scopes)
- **MVP**: Official actions, recent versions
- **Enterprise**: SHA pinning, action allow-list

**Risk 7: Binary tampering between build and publish**
- **Severity**: High
- **Impact**: Modified binaries published
- **Likelihood**: Very low (requires workflow compromise)
- **Mitigation**:
  - Immutable artifacts (upload → download flow)
  - Checksum validation (hash artifacts)
  - Same-workflow publish (no external trigger)
  - Artifact retention limits (auto-delete after 1 day)
- **MVP**: Workflow-internal artifact passing
- **Enterprise**: Signed artifacts, provenance attestation

### Access Control

**Risk 8: Unauthorized releases**
- **Severity**: High
- **Impact**: Malicious or broken version published
- **Likelihood**: Low (requires write access to repository)
- **Mitigation**:
  - Tag protection (only maintainers create tags)
  - Branch protection (prevent direct main commits)
  - Require PR reviews for release script changes
  - Audit log monitoring (track who creates tags)
- **MVP**: Tag protection, branch protection
- **Enterprise**: CODEOWNERS enforcement, approval workflows

**Risk 9: Workflow modification attack**
- **Severity**: Critical
- **Impact**: Attacker modifies workflow to skip validation or exfiltrate secrets
- **Likelihood**: Low (requires PR approval)
- **Mitigation**:
  - Protect .github/workflows/ with CODEOWNERS
  - Require security team review for workflow changes
  - GitHub Actions workflow read-only by default
  - Audit workflow changes in PRs
- **MVP**: CODEOWNERS for workflows directory
- **Enterprise**: Dedicated security review process

**Risk 10: Secret exposure in logs**
- **Severity**: High
- **Impact**: NPM_TOKEN or other secrets leaked
- **Likelihood**: Low (GitHub masks secrets automatically)
- **Mitigation**:
  - Never echo secrets in scripts
  - Use `secrets.*` context, not env vars
  - GitHub's automatic secret masking
  - Review workflow logs before making public
- **MVP**: Standard secret handling practices
- **Enterprise**: Secret scanning tools, audit log analysis

### Runtime Security

**Risk 11: Binary execution vulnerabilities**
- **Severity**: Medium
- **Impact**: Users run vulnerable binary on their systems
- **Likelihood**: Medium (inherited from Rust dependencies)
- **Mitigation**:
  - Cargo audit in CI (check Rust dependencies)
  - Update dependencies regularly
  - Follow Rust security advisories
  - Statically link dependencies (reduce runtime deps)
- **MVP**: Document dependency update process
- **Enterprise**: Automated cargo audit in CI, vulnerability scanning

**Risk 12: Insufficient binary validation**
- **Severity**: Medium
- **Impact**: Corrupted or malicious binary ships to users
- **Likelihood**: Low (caught by validation)
- **Mitigation**:
  - Size checks (corrupted binaries often wrong size)
  - Execution test on native platform
  - File type verification (`file` command)
  - Checksum comparison (build to build)
- **MVP**: Size and execution checks
- **Enterprise**: Binary signing, SBOM generation

### Denial of Service

**Risk 13: Workflow resource exhaustion**
- **Severity**: Low
- **Impact**: GitHub Actions quota consumed, delays releases
- **Likelihood**: Low (builds are time-bounded)
- **Mitigation**:
  - Timeout limits on jobs (max 60 minutes)
  - Cache Cargo builds (reduce build time)
  - Abort on build failure (fail fast)
  - Monitor GitHub Actions usage
- **MVP**: Job timeouts
- **Enterprise**: Dedicated runner pools

**Risk 14: Tag flooding attack**
- **Severity**: Low
- **Impact**: Many workflow runs triggered, quota exhaustion
- **Likelihood**: Very low (requires write access)
- **Mitigation**:
  - Tag protection (only maintainers)
  - Workflow concurrency limits (cancel in-progress)
  - Manual tag cleanup process
- **MVP**: Tag protection
- **Enterprise**: Automated tag validation, rate limiting

## Security Baseline for MVP

**Implemented protections**:
1. ✅ NPM_TOKEN stored as GitHub secret
2. ✅ Scoped package name (`@crewchief/cli`)
3. ✅ Tag protection rules (maintainers only)
4. ✅ Branch protection on main (require reviews)
5. ✅ Binary validation (size, execution)
6. ✅ GitHub-hosted runners (managed infrastructure)
7. ✅ Official GitHub actions only
8. ✅ Workflow-internal artifact passing
9. ✅ pnpm lock file committed
10. ✅ Job timeout limits (60 minutes max)

**Documented but not automated**:
- npm 2FA setup instructions
- Dependency update process
- Security incident response

**Explicitly not implemented** (acceptable for MVP):
- Signed commits or binaries
- SBOM generation
- Vulnerability scanning automation
- Environment protection rules
- SHA-pinned actions
- Reproducible builds

## Security Checklist

### Pre-Deployment

- [ ] NPM_TOKEN secret configured in repository
- [ ] npm account has 2FA enabled
- [ ] Package `@crewchief/cli` owned by correct org
- [ ] Tag protection enabled (main branch)
- [ ] Branch protection enabled (require 1 review)
- [ ] CODEOWNERS file created for `.github/workflows/`
- [ ] Workflow uses official actions only
- [ ] Secrets never echoed in scripts

### Post-Deployment

- [ ] First release published successfully
- [ ] npm package page shows correct ownership
- [ ] Deprecation of old package completed
- [ ] Monitor npm downloads for anomalies
- [ ] Document security incident response
- [ ] Set up dependency update reminders

## Compliance Considerations

**This project does not require**:
- SOC 2 compliance (developer tool)
- HIPAA compliance (no PHI)
- PCI DSS compliance (no payment data)
- GDPR considerations (no user data collection)

**Industry standards that apply**:
- SLSA (Supply Chain Levels for Software Artifacts)
  - Current level: ~SLSA 2 (signed provenance would reach SLSA 3)
- OpenSSF Best Practices
  - Follows most basic practices
  - Could adopt scorecard in future

## Incident Response

**If NPM_TOKEN compromised**:
1. Immediately revoke token on npmjs.com
2. Generate new token with 2FA
3. Update GitHub secret
4. Review all published versions for tampering
5. Publish security advisory if needed
6. Rotate any other potentially exposed secrets

**If malicious version published**:
1. Unpublish version immediately (`npm unpublish @crewchief/cli@<version>`)
2. Publish patched version with next patch number
3. Issue security advisory on GitHub
4. Notify users via deprecation message
5. Investigate root cause (compromised token, account, or workflow)

**If workflow compromised**:
1. Disable workflow immediately
2. Review all workflow runs for suspicious activity
3. Check GitHub audit log for unauthorized changes
4. Rotate NPM_TOKEN and any other secrets
5. Review and fix workflow before re-enabling
6. Consider additional protections (environment rules, approvals)

## Security Documentation

**README.md should include**:
- Security policy (how to report vulnerabilities)
- Supported versions (what gets security updates)
- Known limitations

**SECURITY.md** (create in repo root):
```markdown
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

Please report security vulnerabilities by emailing [security email].
Do not open public issues for security vulnerabilities.

We will respond within 48 hours and provide a fix timeline.

## Security Measures

- npm packages published from GitHub Actions only
- Binaries built on GitHub-hosted runners
- NPM_TOKEN stored as encrypted GitHub secret
- Tag protection prevents unauthorized releases
```

## Enterprise-Grade Enhancements (Future)

Not in scope for MVP, but worth noting for future consideration:

1. **SLSA Level 3**: Generate signed build provenance
2. **Binary signing**: Code sign macOS binaries, sign Linux binaries with GPG
3. **SBOM**: Generate Software Bill of Materials for each release
4. **Vulnerability scanning**: Automated cargo audit, Snyk/Dependabot
5. **Environment protection**: Require manual approval for production deploys
6. **Signed commits**: Require GPG-signed commits for releases
7. **Private registry**: Mirror dependencies in private npm registry
8. **Reproducible builds**: Deterministic builds for audit trail
9. **Security scorecard**: OpenSSF Scorecard integration
10. **Penetration testing**: Third-party security audit

## Risk Acceptance Statement

For MVP release of `@crewchief/cli`, we accept the following residual risks:

**Accepted Risks**:
1. **No binary signing**: Users can't cryptographically verify binaries came from us
   - **Rationale**: Developer tool, low user count, npm package integrity provides basic verification

2. **No environment protection**: No manual approval before publish
   - **Rationale**: Tag protection + branch protection provide sufficient access control

3. **No vulnerability scanning automation**: Dependency vulnerabilities not automatically caught
   - **Rationale**: Manual `cargo audit` during releases, low attack surface

4. **No SLSA provenance**: Can't prove build chain integrity cryptographically
   - **Rationale**: GitHub-hosted runners + open workflow provide transparency

5. **No signed commits**: Can't verify commit author cryptographically
   - **Rationale**: GitHub account security + branch protection sufficient

**Unacceptable Risks** (must mitigate):
1. ❌ NPM_TOKEN in plaintext → Use GitHub secrets
2. ❌ No tag protection → Enable tag protection
3. ❌ No validation → Implement validation checks
4. ❌ Unknown package ownership → Pre-register `@crewchief/cli`
5. ❌ Third-party actions without review → Use official actions only

## Security Review Sign-Off

**Reviewer**: [Name]
**Date**: [Date]
**Risk Assessment**: Low-Medium
**Recommendation**: Approve for MVP release with documented security baseline

**Conditions for approval**:
- All "Unacceptable Risks" mitigated before first production release
- Security checklist completed
- SECURITY.md published in repository
- Incident response plan documented

**Follow-up actions**:
- Review security posture after 3 months
- Consider enterprise enhancements if user base grows
- Monitor npm security advisories for Rust dependencies
