# Ticket: CIFIX-1001: Remove explicit pnpm version from test.yml

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (workflow configuration change)
- [x] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Remove the explicit `version: 10` configuration from `.github/workflows/test.yml` to eliminate the pnpm version conflict error that is currently blocking CI.

## Background
The test workflow currently fails with "Multiple versions of pnpm specified" because:
1. Workflow explicitly sets `version: 10` in pnpm/action-setup@v4 (lines 57-59)
2. Root package.json declares `packageManager: "pnpm@10.12.1+sha512..."`
3. The pnpm/action-setup@v4 action detects both and rejects the configuration as ambiguous

The pnpm/action-setup@v4 action is designed to auto-detect pnpm version from the packageManager field (npm RFC standard). Removing the explicit version makes package.json the single source of truth.

This ticket implements the test workflow fix from Phase 1 of the CIFIX project plan.

## Acceptance Criteria
- [x] The `with: version: 10` block is removed from test.yml (lines 57-59)
- [x] Explanatory comment added explaining auto-detection behavior
- [x] YAML syntax validation passes (yamllint)
- [x] packageManager field verified to exist and be valid in package.json
- [ ] Post-commit: CI run succeeds without "Multiple versions" error

## Technical Requirements
- **File to modify**: `.github/workflows/test.yml`
- **Lines to change**: 56-59
- **Change type**: Remove the `with:` block entirely
- **Add comment**: Explain that pnpm version is auto-detected from package.json packageManager field
- **Validation**: Run yamllint to ensure YAML syntax remains valid
- **Verification**: Confirm packageManager field exists in root package.json

## Implementation Notes
The change is straightforward:

```yaml
# BEFORE (lines 56-59):
- name: Setup pnpm
  uses: pnpm/action-setup@v4
  with:
    version: 10

# AFTER (lines 56-57):
- name: Setup pnpm
  uses: pnpm/action-setup@v4
  # Auto-detects pnpm version from package.json packageManager field
```

The pnpm/action-setup@v4 action will automatically detect the version from the packageManager field as per the Corepack specification. This is the recommended approach per the action's documentation.

**Validation Commands**:
```bash
# Pre-commit validation
yamllint .github/workflows/test.yml
jq -r '.packageManager' /workspace/package.json  # Should show: pnpm@10.12.1+sha512...

# Post-commit validation (check CI logs)
# Verify "Setup pnpm" step shows version 10.12.1 was detected and installed
```

## Dependencies
None - this is an independent workflow configuration fix.

## Risk Assessment
- **Risk**: Auto-detection could fail if packageManager field is malformed
  - **Mitigation**: Validate packageManager field syntax before commit. If auto-detection fails in CI, fast rollback via git revert
- **Risk**: Workflow might not run on older branches without packageManager field
  - **Mitigation**: This change is only for the gemini-revival branch. Main branch already has proper configuration

## Files/Packages Affected
- `.github/workflows/test.yml` (lines 56-59)
