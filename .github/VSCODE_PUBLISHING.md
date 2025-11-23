# VSCode Extension Publishing Guide

Documentation for publishing the `vscode-maproom` extension to Microsoft VS Code Marketplace and Open VSX Registry.

## Table of Contents

- [Publisher Accounts](#publisher-accounts)
- [Authentication Setup](#authentication-setup)
- [PAT Token Management](#pat-token-management)
- [Publishing Workflows](#publishing-workflows)
- [Troubleshooting](#troubleshooting)
- [Maintenance Schedule](#maintenance-schedule)

---

## Publisher Accounts

### Microsoft VS Code Marketplace

**Publisher ID**: `manifoldlogic`
**Marketplace URL**: https://marketplace.visualstudio.com/publishers/manifoldlogic
**Management Portal**: https://marketplace.visualstudio.com/manage/publishers/manifoldlogic

**Account Details**:
- Publisher display name: Manifold Logic
- Verified publisher: Pending (verify after first publish)
- Contact email: (configured via marketplace account settings)

### Open VSX Registry

**Publisher Namespace**: `manifoldlogic`
**Registry URL**: https://open-vsx.org/namespace/manifoldlogic
**Management Portal**: https://open-vsx.org/user-settings/namespaces

**Account Details**:
- Namespace: manifoldlogic
- Access level: Owner (publisher account)

---

## Authentication Setup

### GitHub Secrets Configuration

The following secrets are configured in the repository for CI/CD publishing:

| Secret Name | Purpose | Marketplace | Configured |
|------------|---------|-------------|-----------|
| `VSCE_PAT` | Microsoft Marketplace publishing | VS Code Marketplace | ✓ (2025-11-23) |
| `OVSX_PAT` | Open VSX publishing | Open VSX Registry | ✓ (2025-11-23) |

**Verification**:
```bash
# List all configured secrets
gh secret list

# Expected output should include:
# VSCE_PAT    [timestamp]
# OVSX_PAT    [timestamp]
```

---

## PAT Token Management

### Token Expiration

Personal Access Tokens (PATs) have limited lifespans and require periodic renewal:

| Token | Expiration Period | Next Renewal Date |
|-------|------------------|-------------------|
| `VSCE_PAT` | 90 days (max) | 2026-02-21 (created 2025-11-23) |
| `OVSX_PAT` | 90+ days | 2026-02-21 (created 2025-11-23, verify actual expiration in marketplace) |

### Renewal Procedure

#### Renewing VS Code Marketplace PAT (`VSCE_PAT`)

1. **Generate New PAT**:
   - Go to Azure DevOps: https://dev.azure.com/
   - Navigate to User Settings → Personal Access Tokens
   - Click "New Token"
   - Configure:
     - Name: `VSCE_PAT_CICD_[date]` (e.g., `VSCE_PAT_CICD_20260221`)
     - Organization: Select your organization
     - Expiration: 90 days (maximum allowed)
     - Scopes: **Marketplace** → **Manage** (full permissions)
   - Click "Create"
   - **CRITICAL**: Copy the token immediately (cannot be viewed again)

2. **Update GitHub Secret**:
   ```bash
   # Update the VSCE_PAT secret
   gh secret set VSCE_PAT
   # Paste the new PAT when prompted
   ```

3. **Verify Update**:
   ```bash
   gh secret list
   # Verify VSCE_PAT timestamp is updated
   ```

4. **Test Publishing** (optional):
   ```bash
   # Trigger a test publish workflow
   gh workflow run publish-vscode-extension.yml \
     --ref main \
     --field dry_run=true
   ```

5. **Revoke Old Token**:
   - Return to Azure DevOps → Personal Access Tokens
   - Find the old token
   - Click "Revoke"
   - Confirm revocation

#### Renewing Open VSX PAT (`OVSX_PAT`)

1. **Generate New PAT**:
   - Go to Open VSX: https://open-vsx.org/user-settings/tokens
   - Click "Generate New Token"
   - Configure:
     - Description: `OVSX_PAT_CICD_[date]`
     - Expiration: 90 days or longer (if available)
   - Click "Generate"
   - Copy the token immediately

2. **Update GitHub Secret**:
   ```bash
   gh secret set OVSX_PAT
   # Paste the new PAT when prompted
   ```

3. **Verify and Test**: (same as VSCE_PAT steps 3-4)

### Calendar Reminders

**Set up renewal reminders** 1-2 weeks before expiration:

#### Google Calendar
1. Create event: "Renew VSCE_PAT and OVSX_PAT"
2. Set date: [Expiration date - 14 days]
3. Set reminder: 1 week before, 3 days before, 1 day before
4. Add description: Link to this document section

#### GitHub Issues
```bash
# Create reminder issue 2 weeks before expiration
gh issue create \
  --title "🔐 PAT Token Renewal Required" \
  --body "VSCE_PAT and OVSX_PAT expire on [date]. Follow renewal procedure in .github/VSCODE_PUBLISHING.md" \
  --label "maintenance,security" \
  --assignee @me
```

---

## Publishing Workflows

### Automated Publishing (CI/CD)

The repository includes automated workflows for VSCode extension publishing:

**Workflow**: `.github/workflows/publish-vscode-extension.yml`
- **Triggers**: Version tags (`vscode-maproom@v*.*.*`), manual dispatch
- **Marketplaces**: Both VS Code Marketplace and Open VSX Registry
- **Authentication**: Uses `VSCE_PAT` and `OVSX_PAT` secrets
- **Dry-run mode**: Available via `workflow_dispatch` for testing

**Manual Trigger** (Dry-run):
```bash
# Test publishing without actual release
gh workflow run publish-vscode-extension.yml \
  --ref main \
  --field dry_run=true
```

**Production Release**:
```bash
# Create version tag to trigger automatic publish
VERSION="1.0.0"
git tag "vscode-maproom@v${VERSION}"
git push origin "vscode-maproom@v${VERSION}"

# Monitor workflow
gh run watch
```

### Manual Publishing (Fallback)

If automated workflows fail, publish manually:

#### VS Code Marketplace
```bash
cd packages/vscode-maproom

# Install vsce CLI
npm install -g @vscode/vsce

# Package extension
vsce package

# Publish (requires VSCE_PAT)
vsce publish -p $VSCE_PAT
```

#### Open VSX Registry
```bash
cd packages/vscode-maproom

# Install ovsx CLI
npm install -g ovsx

# Package extension (if not already packaged)
vsce package

# Publish (requires OVSX_PAT)
ovsx publish -p $OVSX_PAT
```

---

## Troubleshooting

### Authentication Errors

#### "401 Unauthorized" - VS Code Marketplace

**Symptom**: Publishing fails with "401 Unauthorized" or "Invalid PAT"

**Causes**:
1. PAT expired (90-day limit)
2. PAT revoked or deleted
3. Incorrect PAT in GitHub secret
4. Insufficient scopes on PAT

**Resolution**:
```bash
# 1. Verify secret exists
gh secret list | grep VSCE_PAT

# 2. Test PAT manually
cd packages/vscode-maproom
vsce publish -p [paste-pat-here]

# 3. If PAT works manually but not in CI, update secret
gh secret set VSCE_PAT
# Paste the working PAT

# 4. If PAT doesn't work, generate new PAT
# Follow "Renewing VS Code Marketplace PAT" above
```

#### "403 Forbidden" - Open VSX Registry

**Symptom**: Publishing fails with "403 Forbidden" or "Namespace not found"

**Causes**:
1. PAT doesn't have access to `manifoldlogic` namespace
2. Namespace ownership changed
3. PAT expired

**Resolution**:
```bash
# 1. Verify namespace access
# Visit https://open-vsx.org/user-settings/namespaces
# Confirm "manifoldlogic" is listed

# 2. Test PAT manually
cd packages/vscode-maproom
ovsx publish -p [paste-pat-here]

# 3. If namespace access lost, request access
# Email open-vsx-admin@eclipse.org with namespace request

# 4. Generate new PAT with correct permissions
# Follow "Renewing Open VSX PAT" above
```

### Publishing Workflow Failures

#### "Extension package not found"

**Symptom**: Workflow fails at publish step with "*.vsix not found"

**Cause**: Extension packaging step failed

**Resolution**:
```bash
# Check workflow logs for packaging errors
gh run view [run-id] --log

# Common packaging issues:
# - Missing dependencies in package.json
# - Incorrect package.json version
# - Missing required files (README.md, LICENSE, icon)

# Test packaging locally
cd packages/vscode-maproom
vsce package --no-yarn
```

#### "Version already exists"

**Symptom**: Publishing fails with "Version X.Y.Z already exists"

**Cause**: Attempting to publish same version twice

**Resolution**:
```bash
# Check current published version
vsce show manifoldlogic.vscode-maproom

# Increment version in package.json
cd packages/vscode-maproom
npm version patch  # or minor, or major

# Create new tag
VERSION=$(node -p "require('./package.json').version")
git tag "vscode-maproom@v${VERSION}"
git push origin "vscode-maproom@v${VERSION}"
```

### Secret Rotation Issues

#### "Secret update not reflected in workflow"

**Symptom**: Updated PAT secret but workflow still fails with old PAT

**Cause**: GitHub Actions cache or timing issue

**Resolution**:
```bash
# 1. Verify secret was updated
gh secret list | grep -E "(VSCE_PAT|OVSX_PAT)"

# 2. Trigger new workflow run (don't re-run failed run)
gh workflow run publish-vscode-extension.yml --ref main

# 3. If still failing, delete and recreate secret
gh secret delete VSCE_PAT
gh secret set VSCE_PAT
# Paste new PAT
```

---

## Maintenance Schedule

### Monthly Tasks
- [ ] Verify extension is published and accessible on both marketplaces
- [ ] Check for security vulnerabilities: `npm audit`
- [ ] Review extension analytics (downloads, ratings, reviews)

### Quarterly Tasks (Every 3 Months)
- [ ] **PAT Token Renewal** (critical - 90-day expiration)
  - Generate new VSCE_PAT
  - Generate new OVSX_PAT
  - Update GitHub secrets
  - Test publishing workflows
- [ ] Review and update extension metadata (README, changelog)
- [ ] Update dependencies: `npm update` in packages/vscode-maproom

### Annual Tasks
- [ ] Review publisher account access and permissions
- [ ] Update publisher profile information
- [ ] Review and optimize extension size
- [ ] Consider verified publisher badge application

### Checklist Template

```markdown
## PAT Renewal - [Date]

**VSCE_PAT**:
- [ ] Generated new PAT in Azure DevOps
- [ ] Updated GitHub secret
- [ ] Verified secret in `gh secret list`
- [ ] Tested dry-run publish
- [ ] Revoked old PAT

**OVSX_PAT**:
- [ ] Generated new PAT in Open VSX
- [ ] Updated GitHub secret
- [ ] Verified secret in `gh secret list`
- [ ] Tested dry-run publish
- [ ] Revoked old PAT (if applicable)

**Next Renewal Date**: [Date + 90 days]
**Calendar Reminder Set**: Yes/No
```

---

## Emergency Contacts

**Marketplace Support**:
- VS Code Marketplace: https://github.com/microsoft/vscode/issues
- Open VSX Registry: https://github.com/eclipse/openvsx/issues

**Repository Admins**:
- Repository owner/admins (configured via GitHub repository settings)

---

## Additional Resources

- [VS Code Publishing Guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [Open VSX Publishing Guide](https://github.com/eclipse/openvsx/wiki/Publishing-Extensions)
- [VSCE CLI Documentation](https://github.com/microsoft/vscode-vsce)
- [OVSX CLI Documentation](https://github.com/eclipse/openvsx/wiki/Publishing-Extensions#publishing-from-the-command-line)
- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

---

**Last Updated**: 2025-11-23
**Document Version**: 1.0
**Maintained By**: CI/CD Team
