# Ticket: CIFIX-1002: Update workflow documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only change)
- [x] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Add troubleshooting documentation to `.github/CLAUDE.md` explaining the pnpm auto-detection behavior and how to maintain it.

## Background
After fixing the pnpm version conflict in CIFIX-1001, developers need clear documentation on:
1. How pnpm version is now managed (packageManager field)
2. How to update pnpm version (change package.json only, not workflow)
3. Troubleshooting if "Multiple versions" error returns

This prevents future regressions and helps onboard new developers. This ticket implements documentation improvements identified in the CIFIX project plan.

## Acceptance Criteria
- [x] `.github/CLAUDE.md` has new "Troubleshooting Workflows" section
- [x] Documentation explains pnpm auto-detection mechanism
- [x] Clear instructions on how to update pnpm version
- [x] Troubleshooting guide for "Multiple versions of pnpm" error
- [x] Explains that workflow YAML should never specify pnpm version

## Technical Requirements
- **File**: `.github/CLAUDE.md`
- **Section to add**: "Troubleshooting Workflows"
- **Format**: Markdown documentation with code examples
- **Content**: Must cover pnpm version management and common issues

## Implementation Notes
Add a new "Troubleshooting Workflows" section to `.github/CLAUDE.md` with the following content:

```markdown
## Troubleshooting Workflows

### Test Workflow

**pnpm Version Management:**
- pnpm version is auto-detected from `package.json` packageManager field
- To change pnpm version: Update `package.json` ONLY (not workflow YAML)
- Do NOT add explicit `version:` to `pnpm/action-setup@v4`

**Common Issues:**

#### "Multiple versions of pnpm specified"
- **Cause**: Explicit version in workflow + packageManager in package.json
- **Fix**: Remove explicit `with: version:` from `.github/workflows/test.yml`
- **Prevention**: Never add version field to pnpm/action-setup step

#### "pnpm command not found"
- **Cause**: packageManager field missing or malformed
- **Fix**: Verify `jq -r '.packageManager' package.json` returns valid value
- **Format**: Must be `pnpm@<version>+sha512...`
```

**Validation**:
```bash
# Verify documentation is clear and renders correctly
cat .github/CLAUDE.md | grep -A 20 "Troubleshooting Workflows"

# Verify markdown syntax
markdownlint .github/CLAUDE.md || echo "No markdown linter available"
```

## Dependencies
- **Requires**: CIFIX-1001 (provides context for documentation)

## Risk Assessment
- **Risk**: None - documentation-only change
  - **Mitigation**: N/A

## Files/Packages Affected
- `.github/CLAUDE.md`
