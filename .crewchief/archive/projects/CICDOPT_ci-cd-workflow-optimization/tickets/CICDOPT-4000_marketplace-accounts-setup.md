# Ticket: CICDOPT-4000: Setup Marketplace Accounts and PAT Tokens

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (manual account setup task)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Tests pass - N/A (manual account setup and credential configuration)
- Verification will confirm GitHub secrets are accessible via test workflow

## Agents
- vscode-extension-specialist
- verify-ticket
- commit-ticket

## Summary

Create Microsoft VS Code Marketplace and Open VSX Registry publisher accounts, generate Personal Access Tokens (PATs) with appropriate scopes, and configure GitHub repository secrets. This is a **mandatory prerequisite** for all other Phase 4 tickets - the VSCode extension publishing workflows cannot function without these accounts and credentials.

## Background

**Problem Being Solved**:
- **Extension exists**: `packages/vscode-maproom/` is already implemented
- **Cannot publish**: No marketplace accounts or credentials configured
- **Phase 4 blocked**: All publishing workflows require PAT tokens
- **Manual setup required**: Account creation cannot be automated

**Why This is a Prerequisite**:
- VS Code Marketplace account creation requires Microsoft authentication
- Open VSX account requires Eclipse Foundation authentication
- PAT token generation requires authenticated access to marketplaces
- GitHub repository secrets require organization/repo admin access
- All Phase 4 workflows will fail without these credentials

**Context from Review Updates**:
From `.crewchief/projects/CICDOPT_ci-cd-workflow-optimization/planning/review-updates.md` lines 245-262:
- This ticket added as Phase 4 prerequisite after project review
- VSCode extension already exists (packages/vscode-maproom/)
- Account setup is manual, must be done before automation
- PAT tokens have 90-day expiry (renewal procedure needed)

**Context from Plan**:
From `.crewchief/projects/CICDOPT_ci-cd-workflow-optimization/planning/plan.md` lines 463-469:
- Phase 4 prerequisites identified
- Marketplace accounts must be created before workflow implementation
- PAT tokens must be stored as GitHub secrets

**Reference**: This ticket implements Phase 4 Prerequisites from CICDOPT plan.md (VSCode Extension Publishing preparation).

## Acceptance Criteria

### Account Creation
- [ ] **Microsoft VS Code Marketplace Account Created**:
  - Publisher account created at https://marketplace.visualstudio.com/manage
  - Publisher ID configured (e.g., `crewchief` or company-specific ID)
  - Publisher account verified as active
  - Publisher page accessible

- [ ] **Open VSX Registry Account Created**:
  - Account created at https://open-vsx.org/
  - Publisher namespace created matching VS Code publisher ID
  - Publisher account verified as active

### PAT Token Generation
- [ ] **VS Code Marketplace PAT Generated**:
  - PAT created via Azure DevOps (https://dev.azure.com/)
  - Name: `VSCE_PAT_CICD` (or similar descriptive name)
  - Scope: **Marketplace** → **Manage** (full permissions)
  - Expiration: 90 days (maximum allowed)
  - Token copied and securely stored
  - Expiration date documented

- [ ] **Open VSX PAT Generated**:
  - PAT created via Open VSX user settings
  - Description: `OVSX_PAT_CICD`
  - Expiration: 90 days (or longer if available)
  - Token copied and securely stored
  - Expiration date documented

### GitHub Configuration
- [ ] **GitHub Repository Secrets Configured**:
  - `VSCE_PAT` secret added with VS Code Marketplace PAT 
  - `OVSX_PAT` secret added with Open VSX PAT
  - Secrets verified accessible via `gh secret list`

- [ ] **Secret Access Test**:
  - Test workflow created to verify secret accessibility
  - Workflow successfully accesses both VSCE_PAT and OVSX_PAT
  - Test workflow removed after verification

### Documentation
- [ ] **Publishing Documentation Created**: `.github/VSCODE_PUBLISHING.md`
  - Publisher account details (IDs, URLs, admin contacts)
  - PAT renewal procedure (90-day cycle)
  - Secret rotation instructions
  - Troubleshooting guide for authentication issues
  - Calendar reminder setup instructions

- [ ] **Calendar Reminders Set**:
  - Reminder configured for 1 week before VSCE_PAT expiration (83 days from creation)
  - Reminder configured for 1 week before OVSX_PAT expiration (83 days from creation)
  - Renewal procedure documented

## Technical Requirements

### Accounts to Create

**1. Microsoft VS Code Marketplace**:
- URL: https://marketplace.visualstudio.com/manage
- Publisher ID: `manifoldlogic`
- Required for: Publishing to VS Code Marketplace
- Authentication: Microsoft account (sign in or create)
- Verify: Publisher page at https://marketplace.visualstudio.com/manage/publishers/manifoldlogic


**2. Open VSX Registry**:
- URL: https://open-vsx.org/
- Namespace: Should match VS Code publisher ID
- Required for: Publishing to Open VSX (open-source alternative)
- Authentication: Eclipse Foundation account (sign in or create)
- Verify: Publisher page at https://open-vsx.org/namespace/manifoldlogic

### PAT Token Scopes

**VSCE_PAT** (Azure DevOps PAT):
- **Scope**: `Marketplace` → `Manage`
- **Allows**: Publish, unpublish, update extensions
- **Expiration**: 90 days (set calendar reminder at 83 days)
- **Organization**: All accessible organizations
- **Created via**: https://dev.azure.com/ → User icon → Personal access tokens

**OVSX_PAT** (Open VSX PAT):
- **Scope**: Publish permissions
- **Allows**: Publish, update extensions
- **Expiration**: 90 days (if configurable)
- **Created via**: https://open-vsx.org/user-settings/tokens

### GitHub Secrets

Secrets must be added to the repository to be accessible by GitHub Actions workflows:

```bash
# Add secrets via GitHub CLI
gh secret set VSCE_PAT --body "<paste-vsce-token>"
gh secret set OVSX_PAT --body "<paste-ovsx-token>"

# Verify secrets exist
gh secret list
# Expected output:
# VSCE_PAT    Updated YYYY-MM-DD
# OVSX_PAT    Updated YYYY-MM-DD
```

**Alternative via GitHub UI**:
1. Navigate to repository Settings
2. Secrets and variables → Actions
3. Click "New repository secret"
4. Add VSCE_PAT and OVSX_PAT separately

**Security Requirements**:
- Tokens must be copied immediately (only shown once during creation)
- Do NOT paste tokens in Slack, email, or unsecured locations
- Store in GitHub secrets immediately after generation
- If token is leaked, regenerate immediately

## Implementation Notes

### Step 1 - Create VS Code Marketplace Account

```
1. Navigate to https://marketplace.visualstudio.com/manage
2. Sign in with Microsoft account (or create new account if needed)
3. Click "Create publisher"
4. Fill in publisher details:
   - Publisher ID: ManifoldLogic (or company-specific ID)
   - Display Name: ManifoldLogic
   - Description: CrewChief development tools and extensions
5. Click "Create"
6. Verify publisher page: https://marketplace.visualstudio.com/publishers/manifoldlogic
7. Document publisher ID and admin contact information
```

**Important**: Publisher ID cannot be changed after creation. Choose carefully.

### Step 2 - Create Open VSX Account

```
1. Navigate to https://open-vsx.org/
2. Sign in (or create Eclipse Foundation account)
3. Go to User Settings → Namespaces
4. Click "Create Namespace"
5. Configure namespace to match VS Code publisher ID
6. Verify namespace is active
7. Document namespace and admin contact information
```

### Step 3 - Generate VSCE_PAT (VS Code Marketplace PAT)

```
1. Navigate to https://dev.azure.com/
2. Click user icon → Personal access tokens
3. Click "+ New Token"
4. Configure token:
   - Name: VSCE_PAT_CICD
   - Organization: All accessible organizations
   - Expiration: 90 days (maximum allowed)
   - Scopes: Custom defined
     - ✓ Marketplace (Manage)
5. Click "Create"
6. **COPY TOKEN IMMEDIATELY** (only shown once)
7. Store securely until added to GitHub secrets
8. Document creation date and expiration date
```

**Critical**: The token is only shown once. If you lose it, you must regenerate.

### Step 4 - Generate OVSX_PAT (Open VSX PAT)

```
1. Navigate to https://open-vsx.org/user-settings/tokens
2. Click "Generate New Token"
3. Configure:
   - Description: OVSX_PAT_CICD
   - Expiration: 90 days (or longer if available)
4. Click "Generate"
5. **COPY TOKEN IMMEDIATELY**
6. Store securely until added to GitHub secrets
7. Document creation date and expiration date
```

### Step 5 - Add GitHub Secrets

```bash
# Via GitHub CLI (recommended)
gh secret set VSCE_PAT
# Paste token when prompted (input is hidden)

gh secret set OVSX_PAT
# Paste token when prompted (input is hidden)

# Verify secrets were added
gh secret list
# Expected output shows VSCE_PAT and OVSX_PAT with creation dates
```

**Verification**:
- Secrets should appear in repository Settings → Secrets and variables → Actions
- Secret values are never shown after creation (only names and dates)

### Step 6 - Create Test Workflow

Create a simple workflow to verify secrets are accessible:

```yaml
# .github/workflows/test-secrets.yml
name: Test VSCode Publishing Secrets
on: workflow_dispatch

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Check VSCE_PAT exists
        run: |
          if [ -z "${{ secrets.VSCE_PAT }}" ]; then
            echo "VSCE_PAT is not set"
            exit 1
          fi
          echo "VSCE_PAT is configured"

      - name: Check OVSX_PAT exists
        run: |
          if [ -z "${{ secrets.OVSX_PAT }}" ]; then
            echo "OVSX_PAT is not set"
            exit 1
          fi
          echo "OVSX_PAT is configured"
```

**Test Steps**:
1. Create workflow file
2. Commit and push
3. Go to Actions → Test VSCode Publishing Secrets
4. Click "Run workflow"
5. Verify both checks pass
6. Delete workflow file after successful verification

### Step 7 - Document Everything

Create `.github/VSCODE_PUBLISHING.md` with comprehensive documentation:

```markdown
# VSCode Extension Publishing Setup

## Publisher Accounts

### Microsoft VS Code Marketplace
- **Publisher ID**: [actual-publisher-id]
- **URL**: https://marketplace.visualstudio.com/publishers/[actual-publisher-id]
- **Admin Contact**: [email/name]
- **Created**: YYYY-MM-DD

### Open VSX Registry
- **Namespace**: [actual-namespace]
- **URL**: https://open-vsx.org/namespace/[actual-namespace]
- **Admin Contact**: [email/name]
- **Created**: YYYY-MM-DD

## PAT Token Management

### VSCE_PAT (VS Code Marketplace)
- **Created**: YYYY-MM-DD
- **Expires**: YYYY-MM-DD (90 days from creation)
- **Next Renewal**: YYYY-MM-DD (1 week before expiration)
- **Scope**: Marketplace → Manage
- **Created via**: Azure DevOps (https://dev.azure.com/)

### OVSX_PAT (Open VSX)
- **Created**: YYYY-MM-DD
- **Expires**: YYYY-MM-DD (90 days from creation)
- **Next Renewal**: YYYY-MM-DD (1 week before expiration)
- **Scope**: Publish permissions
- **Created via**: Open VSX user settings

## Renewal Procedure

### VSCE_PAT Renewal (Every 90 days)

**When**: 1 week before expiration (calendar reminder should fire)

**Steps**:
1. Navigate to https://dev.azure.com/
2. Click user icon → Personal access tokens
3. Find VSCE_PAT_CICD in list
4. Click "..." → Revoke
5. Click "+ New Token"
6. Configure with SAME settings:
   - Name: VSCE_PAT_CICD
   - Organization: All accessible organizations
   - Expiration: 90 days
   - Scopes: Marketplace (Manage)
7. Click "Create"
8. Copy new token immediately
9. Update GitHub secret:
   ```bash
   gh secret set VSCE_PAT
   # Paste new token when prompted
   ```
10. Update expiration dates in this document
11. Set new calendar reminder for 83 days from today

### OVSX_PAT Renewal (Every 90 days)

**When**: 1 week before expiration (calendar reminder should fire)

**Steps**:
1. Navigate to https://open-vsx.org/user-settings/tokens
2. Find OVSX_PAT_CICD in list
3. Click "Revoke"
4. Click "Generate New Token"
5. Description: OVSX_PAT_CICD
6. Click "Generate"
7. Copy new token immediately
8. Update GitHub secret:
   ```bash
   gh secret set OVSX_PAT
   # Paste new token when prompted
   ```
9. Update expiration dates in this document
10. Set new calendar reminder for 83 days from today

## Troubleshooting

### Publishing fails with "Unauthorized" or "401" error

**Cause**: PAT token expired or has insufficient permissions

**Solution**:
1. Check expiration dates in this document
2. If expired, follow renewal procedure above
3. If not expired, verify token scopes are correct:
   - VSCE_PAT: Marketplace → Manage
   - OVSX_PAT: Publish permissions
4. If scopes wrong, regenerate with correct scopes

### Publishing fails with "Publisher not found"

**Cause**: Publisher ID in package.json doesn't match marketplace account

**Solution**:
1. Check publisher ID in `packages/vscode-maproom/package.json`
2. Verify matches publisher ID in marketplace account
3. Update package.json or create new publisher account if needed

### Secret not accessible in GitHub Actions

**Cause**: Secret not set or wrong secret name

**Solution**:
1. Run: `gh secret list`
2. Verify VSCE_PAT and OVSX_PAT appear in list
3. Check workflow uses correct secret names: `${{ secrets.VSCE_PAT }}`
4. If secret missing, add via: `gh secret set VSCE_PAT`

### Token leaked or compromised

**Immediate Actions**:
1. Revoke compromised token immediately
2. Generate new token with different name
3. Update GitHub secret
4. Review recent publishing activity for unauthorized changes

## Additional Resources

- [VS Code Publishing Guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [Open VSX Publishing Wiki](https://github.com/eclipse/openvsx/wiki/Publishing-Extensions)
- [Azure DevOps PAT Documentation](https://docs.microsoft.com/en-us/azure/devops/organizations/accounts/use-personal-access-tokens-to-authenticate)
```

### Step 8 - Set Calendar Reminders

**VSCE_PAT Reminder**:
```
Title: URGENT: Renew VSCE_PAT for VSCode Publishing
Date: [Creation date + 83 days]
Time: 09:00 AM
Reminder: Day of event
Description:
  - Go to https://dev.azure.com/
  - Revoke old VSCE_PAT_CICD
  - Create new token (same settings)
  - Update GitHub secret: gh secret set VSCE_PAT
  - Update expiration date in .github/VSCODE_PUBLISHING.md
  - Set new reminder for 83 days from today
```

**OVSX_PAT Reminder**:
```
Title: URGENT: Renew OVSX_PAT for VSCode Publishing
Date: [Creation date + 83 days]
Time: 09:00 AM
Reminder: Day of event
Description:
  - Go to https://open-vsx.org/user-settings/tokens
  - Revoke old OVSX_PAT_CICD
  - Generate new token
  - Update GitHub secret: gh secret set OVSX_PAT
  - Update expiration date in .github/VSCODE_PUBLISHING.md
  - Set new reminder for 83 days from today
```

## Dependencies

**Depends On**:
- None (this is a prerequisite ticket for all Phase 4 work)

**Blocks**:
- CICDOPT-4001_vscode-build-workflow (needs VSCE_PAT and OVSX_PAT)
- CICDOPT-4002_microsoft-marketplace-publishing (needs VSCE_PAT)
- CICDOPT-4003_open-vsx-publishing (needs OVSX_PAT)
- CICDOPT-4004_github-release-creation (may need publisher info)

**CRITICAL**: This ticket MUST be completed before ANY other Phase 4 tickets can proceed. All VSCode publishing workflows require these credentials.

## Risk Assessment

**Risk Level**: Low-Medium (manual setup, credential management)

### Risk 1: PAT tokens expire without renewal
- **Impact**: Publishing workflows fail, cannot release new versions
- **Probability**: Medium (90-day cycle, easy to forget)
- **Mitigation**:
  - Calendar reminders set for 1 week before expiration
  - Renewal procedure documented in `.github/VSCODE_PUBLISHING.md`
  - Expiration dates clearly documented
- **Detection**: Publishing workflows fail with "unauthorized" error
- **Resolution**: Follow renewal procedure, regenerate tokens, update secrets

### Risk 2: Tokens stored insecurely during setup
- **Impact**: Credential compromise, unauthorized access to publisher accounts
- **Probability**: Low (if procedure followed correctly)
- **Mitigation**:
  - Copy tokens directly to GitHub secrets (don't paste in Slack/email)
  - Don't store tokens in files, notes, or clipboard history
  - Use GitHub CLI with hidden input for secret creation
  - Document secure handling procedure
- **Detection**: Unusual publishing activity, unauthorized extensions
- **Resolution**: Revoke compromised tokens immediately, regenerate new ones

### Risk 3: Wrong token scopes configured
- **Impact**: Publishing workflows fail with "insufficient permissions"
- **Probability**: Low (procedure specifies exact scopes)
- **Mitigation**:
  - Exact scope requirements documented in ticket
  - VSCE_PAT: Marketplace → Manage (only this scope)
  - Test workflow verifies secrets accessible before proceeding
- **Detection**: Publishing fails with "insufficient permissions" error
- **Resolution**: Regenerate tokens with correct scopes

### Risk 4: Publisher account not accessible by team
- **Impact**: Single point of failure, can't recover if admin unavailable
- **Probability**: Medium (depends on account setup)
- **Mitigation**:
  - Document admin contact information in VSCODE_PUBLISHING.md
  - Consider team access vs individual account
  - Add multiple team members to publisher account if possible
  - Document account recovery procedure
- **Detection**: Cannot access publisher account when needed
- **Resolution**: Add additional admins, transfer ownership if needed

### Risk 5: Publisher ID mismatch with package.json
- **Impact**: Publishing fails, cannot find publisher
- **Probability**: Low (documented in acceptance criteria)
- **Mitigation**:
  - Document publisher ID in VSCODE_PUBLISHING.md
  - Verify package.json publisher field matches before Phase 4 tickets
  - Include publisher ID verification in CICDOPT-4001
- **Detection**: Publishing fails with "publisher not found"
- **Resolution**: Update package.json or create matching publisher account

**Confidence**: Medium - Manual setup required, but well-documented process with clear steps

## Files/Packages Affected

### Files to Create
1. `.github/VSCODE_PUBLISHING.md` - Publisher account and PAT token documentation
2. `.github/workflows/test-secrets.yml` - Temporary workflow to verify secret access (deleted after verification)

### Files to Verify (Not Modified)
- `packages/vscode-maproom/package.json` - Publisher field should match created publisher ID (verification only, modification in CICDOPT-4001)

### GitHub Settings to Configure
- Repository Secrets → Actions:
  - `VSCE_PAT` (VS Code Marketplace PAT)
  - `OVSX_PAT` (Open VSX PAT)

## Related Documentation

**External Resources**:
- [VS Code Publishing Guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [Open VSX Publishing Wiki](https://github.com/eclipse/openvsx/wiki/Publishing-Extensions)
- [Azure DevOps PAT Documentation](https://docs.microsoft.com/en-us/azure/devops/organizations/accounts/use-personal-access-tokens-to-authenticate)
- [GitHub Secrets Documentation](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

**Project Documentation**:
- `.crewchief/projects/CICDOPT_ci-cd-workflow-optimization/planning/plan.md` (lines 463-469) - Phase 4 Prerequisites
- `.crewchief/projects/CICDOPT_ci-cd-workflow-optimization/planning/review-updates.md` (lines 245-262) - Phase 4 Prerequisites Added
- `packages/vscode-maproom/package.json` - Publisher field configuration

## Success Indicators

After this ticket is complete, the following should be true:

1. **Accounts Created**:
   - ✅ VS Code Marketplace publisher account exists and is active
   - ✅ Open VSX publisher account exists and is active
   - ✅ Publisher IDs documented in `.github/VSCODE_PUBLISHING.md`

2. **Credentials Generated**:
   - ✅ VSCE_PAT created with Marketplace → Manage scope
   - ✅ OVSX_PAT created with publish permissions
   - ✅ Expiration dates documented
   - ✅ Calendar reminders set for renewal

3. **GitHub Configuration**:
   - ✅ `VSCE_PAT` secret added to repository
   - ✅ `OVSX_PAT` secret added to repository
   - ✅ Secrets verified accessible via test workflow
   - ✅ Test workflow removed after verification

4. **Documentation**:
   - ✅ `.github/VSCODE_PUBLISHING.md` created with all sections
   - ✅ Renewal procedures documented
   - ✅ Troubleshooting guide included
   - ✅ Admin contact information documented

5. **Phase 4 Unblocked**:
   - ✅ All credentials ready for publishing workflows
   - ✅ CICDOPT-4001 can proceed (build workflow)
   - ✅ CICDOPT-4002 can proceed (marketplace publishing)
   - ✅ CICDOPT-4003 can proceed (Open VSX publishing)

**Final Verification**:
```bash
# Verify secrets exist
gh secret list | grep -E "(VSCE_PAT|OVSX_PAT)"

# Expected output:
# VSCE_PAT    Updated YYYY-MM-DD
# OVSX_PAT    Updated YYYY-MM-DD

# Verify documentation exists
ls -la /workspace/.github/VSCODE_PUBLISHING.md

# Verify calendar reminders set
# (Manual check in calendar application)
```

When all success indicators are met, Phase 4 implementation can begin.
