# Security Review: VSCode Extension Release Workflow

## Threat Model

### Assets

1. **Secrets**:
   - `VSCE_PAT`: VS Code Marketplace Personal Access Token
   - `OVSX_PAT`: Open VSX Registry Personal Access Token
   - `GITHUB_TOKEN`: Automatic token for release creation

2. **Code**:
   - VSCode extension source code
   - Compiled extension (.vsix package)
   - Workflow definitions

3. **Accounts**:
   - manifoldlogic publisher account (VS Code Marketplace)
   - manifoldlogic namespace (Open VSX Registry)
   - GitHub repository

4. **Users**:
   - Extension users (installs from marketplace)
   - Repository maintainers (trigger workflow)

### Threat Actors

**External**:
- Malicious contributors (PR attacks)
- Compromised dependencies (supply chain)
- Marketplace account takeover

**Internal**:
- Accidental misconfiguration
- Overly permissive workflow

### Attack Vectors

1. **Secret Exfiltration**: Logs expose PATs
2. **Malicious Extension**: Compromised build publishes malware
3. **Account Takeover**: Stolen PATs used elsewhere
4. **Unauthorized Publish**: Workflow triggered without authorization
5. **Supply Chain**: Compromised npm packages

## Security Architecture

### Secret Management

#### Current Design

```yaml
publish-extension:
  steps:
    - name: Publish to VS Code Marketplace
      if: ${{ env.VSCE_PAT != '' }}
      env:
        VSCE_PAT: ${{ secrets.VSCE_PAT }}
      run: vsce publish -p "$VSCE_PAT"
```

**Security Properties**:
- ✅ Secrets only in environment variables (not parameters)
- ✅ GitHub auto-masks secret values in logs
- ✅ Secrets scoped to step, not job
- ✅ Check for existence before use

**Vulnerabilities**:
- ⚠️ If command fails, partial secret might appear in error
- ⚠️ PATs have broad permissions (all extensions in namespace)

**Risk Level**: **LOW**
- GitHub masking prevents accidental exposure
- Error messages unlikely to reveal useful secret fragments
- PAT scope limited to publisher account

#### Mitigations

**M1: Environment Variable Pattern**
```yaml
env:
  VSCE_PAT: ${{ secrets.VSCE_PAT }}
run: vsce publish -p "$VSCE_PAT"  # Not -p ${{ secrets.VSCE_PAT }}
```
**Benefit**: Reduces risk of secret in error messages

**M2: Secret Rotation**
- PATs expire every 90 days
- Documented renewal process
- Alerts before expiration

**M3: Minimal Permissions**
- `contents: write` only for release creation
- No push permissions
- No workflow permissions

### Access Control

#### Workflow Trigger Protection

**Current**: `workflow_dispatch` (manual trigger only)

**Security Properties**:
- ✅ Requires GitHub authentication
- ✅ Only users with write access can trigger
- ✅ Audit trail in Actions logs
- ✅ No automatic triggers (no tag-based)

**Risk Level**: **LOW**
- Trusted users only (repository maintainers)
- Manual approval for each release

#### Future: Tag Trigger

**If Enabled**:
```yaml
on:
  push:
    tags:
      - 'vscode-maproom-v*'
```

**Additional Risks**:
- Anyone with push access can create tags
- Automated publish without review

**Mitigation**:
- Protected tags (GitHub branch protection for tags)
- Require signed commits for tags
- Add approval step before publish

**Decision**: Keep manual-only for MVP

### Supply Chain Security

#### Dependencies

**Direct**:
- `@vscode/vsce` (official Microsoft tool)
- `ovsx` (official Open VSX tool)

**Transitive**: Unknown (npm dependency tree)

**Risks**:
1. **Compromised package**: Malicious vsce/ovsx version
2. **Dependency confusion**: Private package name conflict
3. **Typosquatting**: Similar package names

**Mitigations**:

**M1: Install from npm registry**
```yaml
run: npm install -g @vscode/vsce  # Official registry
```
**Benefit**: Reduces risk vs git install

**M2: Lock file** (Future)
```yaml
run: npm ci -g  # Use package-lock.json
```
**Benefit**: Reproducible builds
**Status**: Not implemented (global install)

**M3: Dependency scanning**
```yaml
run: npm audit
```
**Benefit**: Alerts on known vulnerabilities
**Status**: Not implemented (defer to CI)

**Risk Level**: **MEDIUM**
- Official tools generally trustworthy
- npm ecosystem has occasional compromises
- Global install (no lock file) increases risk

### Code Integrity

#### Build Artifact Verification

**Current**:
```yaml
- name: Verify dist structure
  run: test -d packages/vscode-maproom/dist

- name: Smoke tests
  run: unzip -l "$VSIX_FILE" | grep "extension.js"
```

**Security Properties**:
- ✅ Verifies extension.js exists
- ✅ Checks dist/ directory structure
- ❌ No hash verification
- ❌ No signature verification

**Vulnerabilities**:
- ⚠️ No protection against compromised build
- ⚠️ Smoke tests check presence, not integrity

**Risk Level**: **MEDIUM**
- Trusts build process completely
- No defense against malicious build artifact

**Mitigations**:

**M1: Artifact Hashing** (Future)
```yaml
- name: Hash artifact
  run: |
    sha256sum "$VSIX_FILE" > vsix.sha256
    cat vsix.sha256
```
**Benefit**: Audit trail for artifact integrity
**Status**: Not implemented

**M2: Code Scanning**
```yaml
- name: Scan extension
  run: codeql analyze dist/
```
**Benefit**: Detect malicious patterns
**Status**: Out of scope (requires setup)

**M3: Manual Review**
**Process**: Review changes before triggering workflow
**Benefit**: Human oversight
**Status**: Implicit (manual trigger)

#### Package Signing

**Current**: None

**Options**:
1. **VS Code Marketplace**: Supports verified publishers
2. **Open VSX**: No signing mechanism
3. **Manual**: GPG sign .vsix file

**Decision**: Not implemented
**Rationale**:
- Marketplace verification pending (account setup)
- MVP doesn't require signing
- Can add later if needed

### Marketplace Account Security

#### VS Code Marketplace

**Account**: manifoldlogic
**Authentication**: VSCE_PAT (Personal Access Token)

**Security Controls**:
- ✅ PAT scoped to Marketplace only
- ✅ 90-day expiration
- ✅ Stored in GitHub Secrets
- ❌ No 2FA enforcement check

**Risks**:
- PAT theft allows unauthorized publishes
- Account compromise affects all extensions

**Mitigations**:
- M1: Regular PAT rotation (every 90 days)
- M2: Monitor marketplace activity
- M3: Enable 2FA on Microsoft account

**Risk Level**: **MEDIUM**

#### Open VSX Registry

**Namespace**: manifoldlogic
**Authentication**: OVSX_PAT

**Security Controls**:
- ✅ Namespace-scoped token
- ✅ Stored in GitHub Secrets
- ❌ Unknown expiration policy

**Mitigations**:
- M1: Verify token expiration policy
- M2: Rotate periodically (quarterly)
- M3: Monitor namespace activity

**Risk Level**: **MEDIUM**

### Workflow Permissions

#### Current Configuration

```yaml
permissions:
  contents: write  # For creating releases
```

**Risks**:
- `contents: write` allows pushing to repository
- Could create malicious tags
- Could modify workflow files

**Mitigations**:
- M1: Minimize permission scope (only what's needed)
- M2: No `workflows: write` permission
- M3: Protected branches prevent direct pushes

**Risk Level**: **LOW**
- Manual trigger reduces abuse risk
- Protected branches limit damage

**Alternative** (More Restrictive):
```yaml
permissions:
  contents: write  # Releases only
  # Future: Use GITHUB_TOKEN with minimal scope
```

### Audit and Monitoring

#### Logging

**Current**:
- GitHub Actions logs (all steps)
- Workflow run history
- Secret access audit (GitHub audit log)

**Gaps**:
- No alerting on workflow failures
- No monitoring of marketplace publishes
- No verification of extension downloads

**Recommendations**:

**R1: Failure Alerts**
```yaml
- name: Notify on Failure
  if: failure()
  run: # Send alert to Slack/email
```
**Status**: Future enhancement

**R2: Marketplace Monitoring**
- Check download counts after publish
- Alert on suspicious activity
- Monitor ratings/reviews

**Status**: Manual process

**R3: Audit Log Review**
- Monthly review of workflow runs
- Verify all triggers were authorized
- Check for unexpected secret access

**Status**: Not scheduled

### Incident Response

#### Scenario: PAT Compromised

**Detection**:
- Unusual marketplace activity
- Unexpected workflow runs
- External notification

**Response**:
1. Revoke compromised PAT immediately
2. Review marketplace publish history
3. Unpublish any unauthorized versions
4. Generate new PAT
5. Update GitHub secret
6. Notify users if malicious version published

**Recovery Time**: <1 hour (revoke + new PAT)

#### Scenario: Malicious Extension Published

**Detection**:
- User reports
- Marketplace review
- Security scan alerts

**Response**:
1. Unpublish version from all marketplaces
2. Create GitHub release with security advisory
3. Publish patched version
4. Notify affected users
5. Investigate how malicious code entered build

**Recovery Time**: <4 hours (unpublish + patch)

#### Scenario: Workflow Abuse

**Detection**:
- Unexpected workflow runs
- GitHub notifications
- Audit log review

**Response**:
1. Disable workflow (rename file)
2. Review workflow changes
3. Revert unauthorized modifications
4. Investigate access logs
5. Re-enable with fixes

**Recovery Time**: <2 hours

### Compliance and Best Practices

#### GitHub Actions Security Best Practices

- ✅ Pin action versions to SHA (where possible)
- ✅ Use minimal permissions
- ✅ Don't echo secrets
- ✅ Use environment variables for secrets
- ⚠️ Review third-party actions (none used currently)

#### Marketplace Publishing Best Practices

- ✅ Review extension before publishing
- ✅ Test in pre-release mode
- ⚠️ Verified publisher badge (pending)
- ❌ Code signing (not implemented)

#### General CI/CD Security

- ✅ Manual approval for publishes
- ✅ Audit trail of all runs
- ⚠️ Dependency scanning (deferred)
- ❌ SAST/DAST (out of scope)

## Risk Assessment Summary

| Risk | Severity | Likelihood | Current Mitigation | Residual Risk |
|------|----------|------------|-------------------|---------------|
| Secret exposure in logs | High | Low | GitHub masking | **LOW** |
| PAT theft/misuse | High | Low | Secret rotation | **MEDIUM** |
| Compromised dependency | Medium | Medium | Official packages | **MEDIUM** |
| Malicious build | Medium | Low | Manual review | **MEDIUM** |
| Account takeover | High | Low | PAT expiration | **MEDIUM** |
| Unauthorized publish | Medium | Low | Manual trigger | **LOW** |
| Workflow abuse | Medium | Low | Protected branches | **LOW** |

## Security Recommendations

### MVP (Must Have)

1. ✅ **Use environment variables for secrets** (Implemented)
2. ✅ **Manual workflow trigger only** (Implemented)
3. ✅ **Minimal permissions** (Implemented)
4. ✅ **PAT rotation process** (Documented)

### Phase 2 (Should Have)

1. ⏳ **Enable 2FA on marketplace accounts**
2. ⏳ **Artifact hash verification**
3. ⏳ **Failure alerting**
4. ⏳ **Monthly audit log review**

### Phase 3 (Nice to Have)

1. 🔮 **Dependency lock files for global installs**
2. 🔮 **Code signing**
3. 🔮 **Automated security scanning**
4. 🔮 **Marketplace activity monitoring**

## Acceptance Criteria

**Security MVP is acceptable if**:
1. ✅ Secrets never appear in logs
2. ✅ Only authorized users can trigger workflow
3. ✅ PAT rotation process documented
4. ✅ Manual review before each publish
5. ✅ Incident response plan documented

**Decision**: **APPROVED FOR MVP**

The current design provides adequate security for a manually-triggered, infrequently-used publishing workflow. The main risks (PAT compromise, supply chain) are mitigated through rotation, official packages, and manual oversight. Additional hardening can be added incrementally.
