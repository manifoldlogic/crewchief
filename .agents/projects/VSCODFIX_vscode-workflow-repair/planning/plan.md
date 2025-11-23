# Execution Plan: VSCode Workflow Repair

## Project Goal

Replace the failing `release-vscode-maproom.yml` workflow with a robust, validated implementation that successfully publishes the VSCode extension to both marketplaces and creates GitHub releases.

## Success Criteria

1. ✅ Workflow passes GitHub Actions validation on every push
2. ✅ Dry-run mode creates valid .vsix package
3. ✅ Publishing to VS Code Marketplace succeeds
4. ✅ Publishing to Open VSX Registry succeeds
5. ✅ GitHub release created with .vsix attachment
6. ✅ Graceful handling of missing secrets
7. ✅ Clear documentation for future releases

## Phase 1: Implementation (Day 1)

### Task 1.1: Create Workflow File

**Agent**: github-actions-specialist
**Deliverable**: `.github/workflows/release-vscode-maproom.yml`

**Implementation Details**:

```yaml
name: Release VSCode Extension

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release (must match package.json)'
        required: true
        type: string
      pre_release:
        description: 'Mark as pre-release'
        type: boolean
        default: false
      dry_run:
        description: 'Build and package only (skip publishing)'
        type: boolean
        default: false

permissions:
  contents: write

jobs:
  build-extension:
    name: Build TypeScript
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/vscode-maproom...'
      artifact_name: 'vscode-extension-dist'

  package-extension:
    name: Package Extension
    runs-on: ubuntu-latest
    needs: build-extension
    outputs:
      version: ${{ steps.version.outputs.version }}
      vsix_filename: ${{ steps.package.outputs.vsix_filename }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - uses: pnpm/action-setup@v4
      - uses: actions/download-artifact@v4
        with:
          name: vscode-extension-dist
          path: .

      # Verify dist structure
      - run: |
          if [ ! -d "packages/vscode-maproom/dist" ]; then
            echo "ERROR: dist not found"
            exit 1
          fi

      # Verify version
      - id: version
        run: |
          PACKAGE_VERSION=$(node -p "require('./packages/vscode-maproom/package.json').version")
          EXPECTED="${{ inputs.version }}"
          if [ "$PACKAGE_VERSION" != "$EXPECTED" ]; then
            echo "ERROR: Version mismatch"
            exit 1
          fi
          echo "version=$PACKAGE_VERSION" >> $GITHUB_OUTPUT

      # Package
      - run: npm install -g @vscode/vsce
      - id: package
        working-directory: packages/vscode-maproom
        run: |
          vsce package --out vscode-maproom-${{ steps.version.outputs.version }}.vsix
          echo "vsix_filename=vscode-maproom-${{ steps.version.outputs.version }}.vsix" >> $GITHUB_OUTPUT

      # Smoke tests
      - run: |
          cd packages/vscode-maproom
          VSIX="${{ steps.package.outputs.vsix_filename }}"
          unzip -l "$VSIX" | grep -q "extension/package.json" || exit 1
          unzip -l "$VSIX" | grep -q "extension/dist/extension.js" || exit 1

      # Upload
      - uses: actions/upload-artifact@v4
        with:
          name: vscode-extension-vsix
          path: packages/vscode-maproom/*.vsix
          retention-days: 90

  publish-extension:
    name: Publish Extension
    runs-on: ubuntu-latest
    needs: package-extension
    if: ${{ !inputs.dry_run }}
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: vscode-extension-vsix

      - uses: actions/setup-node@v4
        with:
          node-version: 'lts/*'

      # VS Code Marketplace
      - name: Publish to VS Code Marketplace
        if: ${{ env.VSCE_PAT != '' }}
        env:
          VSCE_PAT: ${{ secrets.VSCE_PAT }}
        run: |
          npm install -g @vscode/vsce
          VSIX="${{ needs.package-extension.outputs.vsix_filename }}"
          if [ "${{ inputs.pre_release }}" = "true" ]; then
            vsce publish --packagePath "$VSIX" --pre-release -p "$VSCE_PAT"
          else
            vsce publish --packagePath "$VSIX" -p "$VSCE_PAT"
          fi
        continue-on-error: true
        id: publish_vscode

      # Open VSX Registry
      - name: Publish to Open VSX
        if: ${{ env.OVSX_PAT != '' }}
        env:
          OVSX_PAT: ${{ secrets.OVSX_PAT }}
        run: |
          npm install -g ovsx
          VSIX="${{ needs.package-extension.outputs.vsix_filename }}"
          ovsx publish "$VSIX" -p "$OVSX_PAT"
        continue-on-error: true
        id: publish_ovsx

      # GitHub Release
      - name: Create GitHub Release
        if: ${{ steps.publish_vscode.outcome == 'success' || steps.publish_ovsx.outcome == 'success' }}
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          VERSION="${{ inputs.version }}"
          TAG="vscode-maproom-v${VERSION}"
          VSIX="${{ needs.package-extension.outputs.vsix_filename }}"

          PRERELEASE=""
          [ "${{ inputs.pre_release }}" = "true" ] && PRERELEASE="--prerelease"

          gh release create "$TAG" \
            --title "VSCode Maproom v${VERSION}" \
            --notes "Extension release. See [CHANGELOG.md](https://github.com/danielbushman/crewchief/blob/main/packages/vscode-maproom/CHANGELOG.md)." \
            $PRERELEASE \
            "$VSIX"
```

**Acceptance Criteria**:
- [ ] File created at correct location
- [ ] YAML validates with `python3 -c "import yaml..."`
- [ ] All jobs defined
- [ ] Step-level secret checks implemented
- [ ] continue-on-error on publish steps
- [ ] Outcome-based release conditional

---

### Task 1.2: Local Validation

**Agent**: unit-test-runner (or manual)
**Deliverable**: Validation proof

**Steps**:
```bash
# Step 1: Syntax validation
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-vscode-maproom.yml')); print('✓ Valid')"

# Step 2: Check for common issues
grep -n 'secrets\.' .github/workflows/release-vscode-maproom.yml
# Verify only in env: blocks, not if: conditions

# Step 3: Verify job structure
grep -A 1 'needs:' .github/workflows/release-vscode-maproom.yml
```

**Acceptance Criteria**:
- [ ] YAML parses successfully
- [ ] No job-level secret checks
- [ ] All dependencies correct

---

### Task 1.3: Push and Validate

**Agent**: Developer (manual)
**Deliverable**: Workflow committed to main

**Steps**:
```bash
git checkout -b fix-vscode-workflow
git add .github/workflows/release-vscode-maproom.yml
git commit -m "fix(ci): repair VSCode extension release workflow

Re-implement workflow with robust structure:
- workflow_dispatch trigger only (no tag pattern issues)
- Step-level secret checks (not job-level)
- Continue-on-error for marketplace publishes
- Outcome-based conditionals (not result-based)
- Simpler tag format (vscode-maproom-vX.Y.Z)

Fixes validation errors from previous implementation.

Related: VSCODFIX project"

git push origin fix-vscode-workflow
gh pr create --title "fix(ci): repair VSCode workflow" --body "See commit message"
```

**Acceptance Criteria**:
- [ ] PR created
- [ ] GitHub Actions validates workflow
- [ ] No "workflow file issue" errors
- [ ] Test workflow passes (if any)

---

## Phase 2: Testing (Day 1-2)

### Task 2.1: Dry Run Test

**Agent**: Developer (manual trigger)
**Deliverable**: Successful dry-run execution

**Steps**:
```bash
# Merge PR to main first
gh pr merge --squash

# Trigger dry-run
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=true \
  --field pre_release=false

# Monitor
gh run watch

# Verify
gh run view --log | grep "Package created"
gh run view --log | grep "Skipped" # Publishing should be skipped
```

**Acceptance Criteria**:
- [ ] Build job completes successfully
- [ ] Package job creates .vsix
- [ ] Smoke tests pass
- [ ] Publish job skipped (dry_run=true)
- [ ] .vsix artifact uploaded

---

### Task 2.2: Download and Inspect .vsix

**Agent**: Developer (manual)
**Deliverable**: Verified .vsix package

**Steps**:
```bash
# Download artifact
gh run download <run-id>

# Inspect
unzip -l vscode-extension-vsix/vscode-maproom-0.1.0.vsix
# Verify:
# - extension/package.json exists
# - extension/dist/extension.js exists
# - Size reasonable (<2MB)

# Test locally
code --install-extension vscode-extension-vsix/vscode-maproom-0.1.0.vsix
# Open VS Code, verify extension loads
```

**Acceptance Criteria**:
- [ ] .vsix contains required files
- [ ] Package size reasonable
- [ ] Extension installs locally
- [ ] Extension loads without errors

---

### Task 2.3: Pre-release Test (Optional Staging)

**Agent**: Developer (manual trigger)
**Deliverable**: Extension published to marketplaces in pre-release mode

**Prerequisites**:
- Secrets configured (VSCE_PAT, OVSX_PAT)
- Willing to publish test version

**Steps**:
```bash
# Publish pre-release
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0-rc.1 \
  --field dry_run=false \
  --field pre_release=true

# Monitor
gh run watch

# Verify VS Code Marketplace
open "https://marketplace.visualstudio.com/items?itemName=manifoldlogic.vscode-maproom"
# Check version shows 0.1.0-rc.1 with pre-release badge

# Verify Open VSX
open "https://open-vsx.org/extension/manifoldlogic/vscode-maproom"

# Verify GitHub Release
gh release view vscode-maproom-v0.1.0-rc.1

# Clean up
gh release delete vscode-maproom-v0.1.0-rc.1 --yes
# Manually unpublish from marketplaces if needed
```

**Acceptance Criteria**:
- [ ] Both marketplace publish steps succeed
- [ ] GitHub release created
- [ ] Pre-release flag visible
- [ ] Extension installable

---

## Phase 3: Documentation (Day 2)

### Task 3.1: Update VSCODE_PUBLISHING.md

**Agent**: Developer
**Deliverable**: Updated documentation

**Changes**:
```markdown
## Releasing a New Version

### Preparation
1. Update version in `packages/vscode-maproom/package.json`
2. Update CHANGELOG.md
3. Commit changes
4. Push to main

### Trigger Release
```bash
gh workflow run release-vscode-maproom.yml \
  --field version=X.Y.Z \
  --field dry_run=false \
  --field pre_release=false
```

### Verify
1. Check workflow run: `gh run watch`
2. Verify VS Code Marketplace: https://marketplace.visualstudio.com/...
3. Verify Open VSX: https://open-vsx.org/...
4. Verify GitHub Release: `gh release view vscode-maproom-vX.Y.Z`

### Troubleshooting
- If one marketplace fails: Check logs, manually publish if needed
- If both fail: Check PAT expiration, verify version number
- If build fails: Check TypeScript compilation errors
```

**Acceptance Criteria**:
- [ ] Release process documented
- [ ] Troubleshooting guide added
- [ ] Examples provided

---

### Task 3.2: Create Runbook

**Agent**: Developer
**Deliverable**: `.agents/projects/VSCODFIX_vscode-workflow-repair/RUNBOOK.md`

**Contents**:
- Pre-flight checklist
- Step-by-step release instructions
- Verification steps
- Rollback procedure
- Common failure scenarios

**Acceptance Criteria**:
- [ ] Runbook created
- [ ] Covers all release scenarios
- [ ] Includes examples

---

## Phase 4: Production Release (Day 3+)

### Task 4.1: Release v0.1.0

**Agent**: Developer (when ready)
**Deliverable**: Extension published to production

**Prerequisites**:
- All tests passed
- Documentation complete
- Version bumped in package.json
- CHANGELOG updated

**Steps**:
```bash
# Final check
cat packages/vscode-maproom/package.json | grep version
# Verify: "version": "0.1.0"

# Trigger production release
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=false \
  --field pre_release=false

# Monitor closely
gh run watch

# Verify
gh release view vscode-maproom-v0.1.0
code --install-extension manifoldlogic.vscode-maproom
```

**Acceptance Criteria**:
- [ ] Workflow completes successfully
- [ ] Extension available on both marketplaces
- [ ] GitHub release created
- [ ] Extension installs and works

---

### Task 4.2: Post-Release Validation

**Agent**: Developer
**Deliverable**: Release verification report

**Checks**:
```markdown
## Release Validation: v0.1.0

### Marketplaces
- [ ] VS Code Marketplace shows v0.1.0
- [ ] Open VSX shows v0.1.0
- [ ] Download counts incrementing
- [ ] No error reports

### GitHub
- [ ] Release created: vscode-maproom-v0.1.0
- [ ] .vsix attached to release
- [ ] Release notes visible

### Installation
- [ ] `code --install-extension manifoldlogic.vscode-maproom` works
- [ ] Extension loads in VS Code
- [ ] Extension functions correctly
- [ ] No errors in extension host log

### Metrics
- Downloads after 24h: ___
- Ratings: ___
- Issues reported: ___
```

**Acceptance Criteria**:
- [ ] All checks passed
- [ ] No critical issues reported
- [ ] Extension publicly available

---

## Rollback Plan

### If Workflow Fails After Merge

**Immediate**:
```bash
# Disable workflow
mv .github/workflows/release-vscode-maproom.yml .github/workflows/release-vscode-maproom.yml.bak
git add .github/workflows/
git commit -m "temp: disable failing workflow"
git push

# Publish manually
cd packages/vscode-maproom
vsce package
vsce publish -p $VSCE_PAT
ovsx publish vscode-maproom-0.1.0.vsix -p $OVSX_PAT
```

**Fix**:
- Analyze failure logs
- Identify issue
- Fix workflow
- Test with dry-run
- Re-enable

### If Wrong Version Published

**Immediate**:
```bash
# Unpublish from marketplaces (if within 24 hours)
vsce unpublish manifoldlogic.vscode-maproom@0.1.0

# Delete GitHub release
gh release delete vscode-maproom-v0.1.0 --yes
```

**Republish**:
- Fix version in package.json
- Run workflow again with correct version

---

## Risk Mitigation

| Risk | Mitigation | Owner |
|------|-----------|-------|
| Validation fails again | Local YAML validation before push | Developer |
| Secrets not working | Test with dry-run first | Developer |
| Marketplace API down | continue-on-error allows partial success | Workflow |
| Wrong version | Version verification step | Workflow |
| Extension broken | Smoke tests catch packaging issues | Workflow |

---

## Timeline

| Phase | Duration | Dependencies | Deliverable |
|-------|----------|--------------|-------------|
| **1. Implementation** | 2-4 hours | None | Working workflow file |
| **2. Testing** | 4-8 hours | Phase 1 | Verified workflow |
| **3. Documentation** | 2-3 hours | Phase 2 | Complete docs |
| **4. Production** | 1 hour + monitoring | Phase 3 | Live release |
| **Total** | **1-2 days** | | Production-ready workflow |

---

## Success Metrics

**Technical**:
- ✅ Zero validation errors on push
- ✅ <6 minute workflow duration
- ✅ 100% smoke test pass rate
- ✅ Graceful secret handling

**Business**:
- ✅ Extension available on both marketplaces
- ✅ Users can install extension
- ✅ No critical post-release issues

**Process**:
- ✅ Clear documentation for future releases
- ✅ Repeatable release process
- ✅ Runbook for troubleshooting

---

## Agents Assignment

| Phase | Primary Agent | Support |
|-------|---------------|---------|
| 1. Implementation | github-actions-specialist | vscode-extension-specialist |
| 2. Testing | unit-test-runner | Developer (manual) |
| 3. Documentation | Developer | - |
| 4. Production | Developer | - |

---

## Completion Criteria

Project complete when:
1. ✅ Workflow passes GitHub validation
2. ✅ Dry-run test successful
3. ✅ Documentation updated
4. ✅ Production release successful (or scheduled)
5. ✅ No outstanding issues

## Future Enhancements

**Not in MVP, but documented for later**:
- Add tag trigger back (once workflow proven stable)
- Automated changelog generation
- Marketplace metrics integration
- Slack/email notifications
- Automated version bumping
