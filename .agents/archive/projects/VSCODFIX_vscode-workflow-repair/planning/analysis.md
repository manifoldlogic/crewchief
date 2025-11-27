# Analysis: VSCode Extension Release Workflow Issues

## Problem Definition

The VSCode extension release workflow (`.github/workflows/release-vscode-maproom.yml`) fails GitHub Actions validation on every push to main, despite being configured to only trigger on version tags. The workflow file was created as part of CICDOPT project but encounters immediate validation failures with "workflow file issue" errors (0s runtime).

## Current State

### What Was Built
Complete 5-job workflow for VSCode extension publishing:
1. **build-extension**: Calls reusable TypeScript build
2. **package-extension**: Creates .vsix with vsce, runs smoke tests
3. **publish-vscode**: Publishes to VS Code Marketplace
4. **publish-ovsx**: Publishes to Open VSX Registry (parallel)
5. **create-release**: Creates GitHub release with .vsix

### What's Failing
- Workflow fails validation immediately (0s runtime) on every push to main
- Error: "This run likely failed because of a workflow file issue"
- Trigger configured for tags `@crewchief/vscode-maproom@v*` but validation fails on non-tag pushes
- File was temporarily removed to stop failures

### Investigation Results

**Attempted Fixes**:
1. ✅ Added `${{ }}` wrappers to all conditional expressions
2. ✅ Simplified multi-line release notes (removed heredoc with markdown)
3. ✅ Changed tag pattern from `v*.*.*` to `v*`
4. ✅ YAML validates locally with Python parser
5. ✅ Minimal test workflow validates successfully

**Key Findings**:
- YAML syntax is valid
- Other tag-based workflows (release-cli.yml) don't trigger on pushes
- GitHub validates ALL workflows on push, even if triggers don't match
- Validation error persists despite local YAML validation passing
- The specific validation error is not visible via gh CLI

## Root Cause Hypothesis

**Primary Issue**: GitHub Actions has stricter workflow validation than standard YAML parsers. The workflow likely contains:
1. **Expression evaluation issues** in conditional contexts
2. **Secret reference problems** when workflow hasn't been triggered
3. **Complex job dependencies** that don't validate well
4. **Tag pattern edge cases** with `@` characters

**Evidence**:
- Workflow validates locally but fails in GitHub
- Minimal workflow (without complex conditionals/secrets) passes
- Error occurs before workflow runs (0s duration = validation failure)
- Other complex workflows in repo don't have this issue

## Industry Solutions

### How Other Projects Handle This

**1. Separate Workflows Pattern**:
```yaml
# build.yml - always runs
on: [push, pull_request]

# publish.yml - only on tags
on:
  push:
    tags: ['v*']
```

**Benefits**:
- Validation errors only affect workflows that actually run
- Simpler conditional logic
- Easier to debug

**2. Path-Based Triggers**:
```yaml
on:
  push:
    paths:
      - 'packages/vscode-maproom/**'
    tags:
      - 'vscode-maproom-v*'
```

**3. Workflow Dispatch Only** (for less frequent releases):
```yaml
on:
  workflow_dispatch:
    inputs:
      version:
        required: true
```

### VS Code Extension Publishing Best Practices

**Microsoft's vsce Official Recommendations**:
- Use `workflow_dispatch` for manual releases
- Test packaging separately from publishing
- Validate package before publishing
- Use environment secrets, not repository secrets

**Open VSX Publishing**:
- Parallel publishing with VS Code Marketplace
- Separate jobs allow partial success
- No pre-release flag (uses semver detection)

## Existing Implementations in Project

### Successful Patterns
**`.github/workflows/release-cli.yml`**:
- Uses same tag pattern with `@`: `@crewchief/cli@v*.*.*`
- Has conditional `if: ${{ !inputs.dry_run }}`
- Calls reusable workflows successfully
- **Doesn't fail validation on pushes**

**`.github/workflows/release-maproom-mcp.yml`**:
- Similar structure to intended vscode workflow
- Uses secrets in conditional: `if: ${{ !inputs.dry_run }}`
- **Doesn't fail validation on pushes**

### Key Differences in Failed Workflow

1. **More complex conditionals**:
   ```yaml
   if: ${{ secrets.VSCE_PAT != '' }}  # Checking secret existence
   if: ${{ always() && (needs.publish-vscode.result == 'success' || needs.publish-ovsx.result == 'success') }}
   ```

2. **Job-level secret checks**: Other workflows check `inputs.dry_run`, not `secrets.XYZ`

3. **Multi-job dependencies**: create-release depends on `[package-extension, publish-vscode, publish-ovsx]`

## Problem Breakdown

### Critical Issues

**Issue 1: Secret Existence Checks in Job-Level Conditionals**
```yaml
# This pattern may fail validation
publish-vscode:
  if: ${{ secrets.VSCE_PAT != '' }}
```

**Why it fails**:
- GitHub may not allow secret existence checks at job level
- Secrets might not be available during workflow validation
- `secrets` context might be restricted to step level

**Solution**: Move to step-level with environment protection

---

**Issue 2: Complex Result-Based Conditionals**
```yaml
if: ${{ always() && (needs.publish-vscode.result == 'success' || needs.publish-ovsx.result == 'success') }}
```

**Why it might fail**:
- Complex boolean logic in job conditionals
- Referencing job results that may not exist
- `always()` function in job-level conditional

**Solution**: Simplify or restructure job dependencies

---

**Issue 3: Tag Pattern with Special Characters**
```yaml
tags:
  - '@crewchief/vscode-maproom@v*'
```

**Why it might fail**:
- Double `@` characters in pattern
- Package-scoped naming convention
- GitHub may have tag pattern restrictions

**Solution**: Use workflow_dispatch or simpler tag pattern

## Constraints

### Technical Constraints
- Must maintain dual marketplace publishing (VS Code + Open VSX)
- Must support pre-release flag
- Must create GitHub releases automatically
- Must use existing reusable TypeScript build workflow
- Cannot use npm-style versioning (need scoped tags)

### Environmental Constraints
- Secrets configured: `VSCE_PAT`, `OVSX_PAT`
- Publisher accounts: manifoldlogic (both marketplaces)
- Package: `@crewchief/vscode-maproom`
- Current version: 0.1.0

### Validation Constraints
- Must pass GitHub Actions validation on every push
- Should not trigger on non-tag pushes
- Must handle missing secrets gracefully
- Should allow testing without publishing

## Success Criteria

1. **Workflow validates** on push to main (no validation errors)
2. **Doesn't run unnecessarily** (only on tags or manual trigger)
3. **All jobs execute** when triggered properly
4. **Publishes successfully** to both marketplaces
5. **Creates GitHub release** with .vsix attachment
6. **Handles failures gracefully** (partial success scenarios)
7. **Supports testing** without publishing to production

## Research Insights

### GitHub Actions Limitations

**Documented Restrictions**:
- `secrets` context has limited availability in conditionals
- Job-level conditionals must be simple expressions
- Workflow validation is more strict than YAML validation
- Tag patterns with special characters may need quoting

**Undocumented Behaviors**:
- Some workflows fail validation but error isn't exposed via API
- Validation happens even for workflows that won't run
- Secret checks might work in steps but not job-level conditionals

### Recommended Approach

**Split Workflow Strategy**:
1. Keep packaging workflow simple (always runs on dispatch)
2. Use workflow_dispatch for publishing (manual control)
3. Move secret checks to step level with continue-on-error
4. Simplify job dependencies
5. Add explicit testing mode

**Alternative: Composite Actions**:
- Move publishing logic to composite actions
- Call from simpler workflow
- Better testability and reusability

## Next Steps

Based on analysis, the architectural approach should:
1. **Restructure workflow** to avoid job-level secret checks
2. **Simplify conditionals** to basic expressions
3. **Use workflow_dispatch** as primary trigger (tags optional)
4. **Move complexity to steps** rather than job structure
5. **Add explicit test mode** for validation
