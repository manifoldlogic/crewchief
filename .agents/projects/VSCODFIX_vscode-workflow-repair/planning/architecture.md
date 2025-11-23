# Architecture: VSCode Extension Release Workflow

## Design Principles

1. **Validation First**: Workflow must pass GitHub Actions validation on every push
2. **Fail Fast**: Use step-level checks, not job-level conditionals with secrets
3. **Explicit Over Implicit**: Prefer workflow_dispatch over complex trigger patterns
4. **Testable**: Support dry-run mode for all publishing operations
5. **Resilient**: Handle partial failures (one marketplace succeeds, other fails)
6. **Simple Structure**: Avoid complex job dependencies that complicate validation

## Architecture Decision

### Primary Trigger: workflow_dispatch

**Rationale**:
- Avoids tag pattern validation issues
- Provides explicit control over when releases happen
- Allows testing without publishing
- Simpler to debug and maintain
- No validation errors on regular pushes

**Trade-off**: Manual trigger vs automatic on tag push
- **Acceptable**: VSCode extensions are released infrequently
- **Benefit**: More control, less risk of accidental publishes
- **Future**: Can add tag trigger later if needed

### Job Structure: Linear with Conditional Steps

```
build-extension
    ↓
package-extension
    ↓
publish (single job with conditional steps)
    ├─ publish to vscode (if VSCE_PAT exists)
    ├─ publish to ovsx (if OVSX_PAT exists)
    └─ create release (if any publish succeeded)
```

**Rationale**:
- Simpler than parallel jobs with complex dependencies
- Secret checks moved to step level (not job level)
- Single publish job easier to validate
- Linear flow easier to understand and debug

**Trade-off**: Sequential vs parallel publishing
- **Impact**: ~30-60s longer workflow (acceptable for infrequent releases)
- **Benefit**: Simpler logic, better validation, easier debugging

## Detailed Design

### Workflow Configuration

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
  contents: write  # For creating releases
```

**Key Decisions**:
- `workflow_dispatch` only (no tag trigger initially)
- Version input for explicit control
- `dry_run` flag for testing
- `pre_release` for beta releases
- Minimal permissions (write only for releases)

### Job 1: Build Extension

```yaml
build-extension:
  name: Build TypeScript
  uses: ./.github/workflows/reusable-typescript-build.yml
  with:
    workspace_filter: '@crewchief/vscode-maproom...'
    artifact_name: 'vscode-extension-dist'
```

**Purpose**: Compile TypeScript using proven reusable workflow
**Dependencies**: None
**Outputs**: Artifact `vscode-extension-dist`
**Validation Risk**: Low (proven pattern from CICDOPT)

### Job 2: Package Extension

```yaml
package-extension:
  name: Package Extension
  runs-on: ubuntu-latest
  needs: build-extension

  outputs:
    version: ${{ steps.version.outputs.version }}
    vsix_filename: ${{ steps.package.outputs.vsix_filename }}
    vsix_path: ${{ steps.package.outputs.vsix_path }}

  steps:
    - checkout
    - setup node/pnpm
    - download build artifact
    - verify dist structure
    - verify version matches input
    - install vsce
    - package extension
    - smoke tests (verify .vsix contents)
    - upload .vsix artifact
```

**Purpose**: Create .vsix package with validation
**Dependencies**: build-extension
**Outputs**: Version, filename, path
**Key Validations**:
- dist/ exists and contains extension.js
- package.json version matches input
- .vsix created successfully
- .vsix contains required files

**Validation Risk**: Low (uses standard vsce package command)

### Job 3: Publish Extension

```yaml
publish-extension:
  name: Publish Extension
  runs-on: ubuntu-latest
  needs: package-extension
  if: ${{ !inputs.dry_run }}  # Simple conditional

  steps:
    - download .vsix artifact
    - setup node

    # VS Code Marketplace
    - name: Publish to VS Code Marketplace
      if: ${{ env.VSCE_PAT != '' }}  # Step-level secret check
      env:
        VSCE_PAT: ${{ secrets.VSCE_PAT }}
      run: |
        npm install -g @vscode/vsce
        if [ "${{ inputs.pre_release }}" = "true" ]; then
          vsce publish --packagePath "$VSIX_FILE" --pre-release -p "$VSCE_PAT"
        else
          vsce publish --packagePath "$VSIX_FILE" -p "$VSCE_PAT"
        fi
      continue-on-error: true  # Don't fail workflow if one marketplace fails
      id: publish_vscode

    # Open VSX Registry
    - name: Publish to Open VSX Registry
      if: ${{ env.OVSX_PAT != '' }}  # Step-level secret check
      env:
        OVSX_PAT: ${{ secrets.OVSX_PAT }}
      run: |
        npm install -g ovsx
        ovsx publish "$VSIX_FILE" -p "$OVSX_PAT"
      continue-on-error: true  # Don't fail workflow if one marketplace fails
      id: publish_ovsx

    # Create GitHub Release
    - name: Create GitHub Release
      if: ${{ steps.publish_vscode.outcome == 'success' || steps.publish_ovsx.outcome == 'success' }}
      env:
        GH_TOKEN: ${{ github.token }}
      run: |
        TAG="vscode-maproom-v${{ inputs.version }}"
        PRERELEASE_FLAG=""
        if [ "${{ inputs.pre_release }}" = "true" ]; then
          PRERELEASE_FLAG="--prerelease"
        fi

        gh release create "$TAG" \
          --title "VSCode Maproom v${{ inputs.version }}" \
          --notes "See [CHANGELOG.md](https://github.com/danielbushman/crewchief/blob/main/packages/vscode-maproom/CHANGELOG.md) for details." \
          $PRERELEASE_FLAG \
          "$VSIX_FILE"
```

**Purpose**: Publish to marketplaces and create release
**Dependencies**: package-extension
**Conditional**: Skipped if dry_run=true

**Key Design Choices**:

1. **Step-level secret checks**: `if: ${{ env.VSCE_PAT != '' }}`
   - Moves secret validation to step level
   - Uses environment variable, not `secrets` context in conditional
   - Avoids job-level secret reference

2. **continue-on-error**: Allows partial success
   - One marketplace can fail without blocking the other
   - Release created if at least one succeeds

3. **Outcome-based release conditional**: `steps.publish_vscode.outcome == 'success'`
   - Uses step outcomes, not job results
   - Simpler than `always()` with complex boolean logic

4. **Simpler tag format**: `vscode-maproom-v1.0.0`
   - Avoids double `@` characters
   - Still unique and descriptive
   - Easier to validate

**Validation Risk**: Medium (but mitigated by step-level checks)

## Alternative Considered: Composite Action

```yaml
# .github/actions/publish-vscode/action.yml
name: Publish VSCode Extension
description: Publish extension to marketplaces

inputs:
  vsix_file:
    required: true
  version:
    required: true
  vsce_pat:
    required: false
  ovsx_pat:
    required: false

runs:
  using: composite
  steps:
    - if: ${{ inputs.vsce_pat != '' }}
      run: vsce publish ...
    - if: ${{ inputs.ovsx_pat != '' }}
      run: ovsx publish ...
```

**Rejected Because**:
- Adds complexity without solving validation issue
- Still need workflow with conditionals
- Harder to debug
- Over-engineering for this use case

**Future Consideration**: If we add more extensions, composite action makes sense

## Error Handling Strategy

### Marketplace Publishing Failures

**Scenario**: VSCE_PAT expired or invalid
**Handling**:
```yaml
continue-on-error: true
id: publish_vscode
```
**Outcome**: Step fails, but workflow continues
**Result**: Other marketplace publishes, release created, logs show error

### Version Mismatch

**Scenario**: Input version doesn't match package.json
**Handling**:
```bash
PACKAGE_VERSION=$(node -p "require('./package.json').version")
if [ "$PACKAGE_VERSION" != "$EXPECTED_VERSION" ]; then
  echo "ERROR: Version mismatch!"
  exit 1
fi
```
**Outcome**: Job fails immediately
**Result**: No publishing, clear error message

### .vsix Creation Failure

**Scenario**: vsce package command fails
**Handling**: Natural failure, job stops
**Outcome**: No artifact uploaded
**Result**: publish-extension job skipped (needs artifact)

### Partial Success

**Scenario**: VS Code Marketplace succeeds, Open VSX fails
**Handling**:
```yaml
if: ${{ steps.publish_vscode.outcome == 'success' || steps.publish_ovsx.outcome == 'success' }}
```
**Outcome**: Release created
**Result**: Users can install from successful marketplace, .vsix available for manual publish

## Testing Strategy

### Dry Run Mode

```bash
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=true
```

**Tests**:
- ✅ Build succeeds
- ✅ Package creates .vsix
- ✅ Smoke tests pass
- ✅ No publishing occurs
- ✅ No release created

**Purpose**: Validate workflow logic without publishing

### Test with Missing Secrets

**Setup**: Remove one PAT from repository secrets
**Expected**:
- Other marketplace publishes
- Missing marketplace skipped
- Release still created
- Logs show which was skipped

### Pre-release Test

```bash
gh workflow run release-vscode-maproom.yml \
  --field version=0.2.0-beta.1 \
  --field pre_release=true
```

**Tests**:
- ✅ vsce adds --pre-release flag
- ✅ GitHub release marked as pre-release
- ✅ Marketplace shows pre-release badge

## Performance Considerations

### Workflow Duration

**Estimated**:
- Build: 2-3 minutes (TypeScript compilation)
- Package: 30-60 seconds (vsce package + smoke tests)
- Publish: 1-2 minutes (marketplace API calls)
- **Total**: 4-6 minutes

**Acceptable**: Extensions published infrequently (monthly/quarterly)

### Artifact Size

**.vsix Size**: ~100-500 KB (typical VSCode extension)
**Retention**: 90 days
**Impact**: Negligible

### Cache Strategy

**pnpm cache**: Handled by reusable-typescript-build.yml
**Docker cache**: Not applicable
**Action cache**: Not needed

## Security Considerations

### Secret Exposure

**Risk**: Secrets in logs
**Mitigation**:
- GitHub automatically masks secret values
- Use `-p $VAR` not `-p ${{ secrets.VAR }}` in commands
- Environment variables over direct secret references

### Permission Scope

**Granted**: `contents: write`
**Used For**: Creating releases only
**Risk**: Could push to repository
**Mitigation**: Trust boundary (only runs on manual dispatch)

### Supply Chain

**Dependencies**:
- `@vscode/vsce` (official Microsoft tool)
- `ovsx` (official Open VSX tool)

**Risk**: Compromised packages
**Mitigation**: npm ecosystem trust + infrequent updates

## Deployment Strategy

### Rollout Plan

**Phase 1**: Add workflow with dry_run=true
- Deploy to main
- Test with workflow_dispatch
- Verify no validation errors
- Confirm .vsix creation

**Phase 2**: Test publishing to staging
- Use pre_release=true
- Publish test version (0.1.0-rc.1)
- Verify marketplaces
- Delete test release

**Phase 3**: Production release
- Remove dry_run flag
- Publish actual version
- Monitor for issues
- Document process

### Rollback Plan

If workflow fails:
1. **Disable workflow**: Rename file to `.bak`
2. **Manual publish**: Use vsce/ovsx CLI locally
3. **Fix and retry**: Deploy corrected workflow
4. **Test first**: Use dry_run before production

## Long-term Maintainability

### Evolution Path

**Near-term** (next 3-6 months):
- Add tag trigger back if workflow stable
- Enhance release notes generation
- Add changelog automation

**Medium-term** (6-12 months):
- Consider composite actions if more extensions added
- Automated version bumping
- Release notes from commits

**Long-term** (1+ year):
- Full automated release pipeline
- Multiple extension support
- Marketplace metrics integration

### Documentation Requirements

**Workflow Comments**: Each step explains purpose
**README**: Clear usage instructions
**Troubleshooting**: Common failure scenarios
**Examples**: Sample workflow_dispatch commands

## Success Metrics

1. **Validation**: Workflow passes validation on every push ✅
2. **Reliability**: 95%+ success rate on manual triggers
3. **Performance**: <6 minutes total runtime
4. **Usability**: Clear error messages on failures
5. **Testability**: Dry-run mode works without secrets

## Constraints and Trade-offs

| Constraint | Solution | Trade-off |
|------------|----------|-----------|
| Validation errors | Remove job-level secret checks | More verbose step-level checks |
| Complex conditionals | Step outcomes vs job results | Slightly more code |
| Tag patterns | Simpler tag format | Not using npm-style scoped tags |
| Parallel publishing | Sequential with continue-on-error | 30-60s slower |
| Auto-trigger | Manual workflow_dispatch | Requires human intervention |

**All trade-offs acceptable** for MVP that prioritizes working > perfect.
