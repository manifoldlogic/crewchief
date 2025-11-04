# Ticket: BINPKG-1906: Install dependencies before npm publish for prepublishOnly hook

## Status
- [x] **Task completed** - added npm install --ignore-scripts + fixed prepublishOnly to use npm audit
- [x] **Tests pass** - workflow change only, no code tests
- [x] **Verified** - by the verify-ticket agent

## Agents
- github-actions-engineer
- verify-ticket
- commit-ticket

## Summary
Add npm install step before npm publish to ensure dependencies are available for the prepublishOnly hook which runs TypeScript compilation and security audit.

## Background
Discovered in workflow run #19054236628 after BINPKG-1905 fix. All 4 platform builds succeeded, validation passed, and tarball was created successfully. However, the npm publish step failed with TypeScript compilation errors:

```
error TS2307: Cannot find module 'pg' or its corresponding type declarations
error TS2580: Cannot find name 'process'. Do you need to install type definitions for node?
error TS2307: Cannot find module 'node:child_process' or its corresponding type declarations
```

**Root Cause**: The package.json includes a prepublishOnly script:
```json
"prepublishOnly": "tsc && pnpm audit --audit-level=high --prod"
```

This script runs automatically before `npm publish`, but the workflow doesn't install dependencies before publishing. When npm tries to run `tsc`, it fails because:
1. TypeScript isn't installed
2. Type definitions (@types/node, @types/pg) aren't installed
3. Runtime dependencies (pg, pino, zod, execa) aren't installed

The tarball already includes the compiled `dist/` directory from the local build, but the prepublishOnly hook tries to rebuild from source as a safety check.

## Acceptance Criteria
- [x] npm install step added before npm publish
- [x] Step runs only when dry_run is false (same condition as publish)
- [x] Dependencies installed in packages/maproom-mcp directory
- [x] prepublishOnly hook runs successfully with dependencies available (v1.3.1 confirmed)
- [x] npm publish succeeds in dry-run mode (v1.3.1 Run ID: 19055680204)
- [x] Workflow completes successfully (v1.3.1 published to npm)

## Technical Requirements

### Current Problematic Flow
1. Create tarball with `npm pack` (works - includes prebuilt dist/)
2. Verify tarball contents (works - all binaries present)
3. Run `npm publish` → triggers prepublishOnly hook
4. prepublishOnly runs `tsc` → FAILS (no dependencies)

### Fixed Flow
1. Create tarball with `npm pack` (works - includes prebuilt dist/)
2. Verify tarball contents (works - all binaries present)
3. **Run `npm install --ignore-scripts` → installs dependencies, skips prepare hook**
4. Run `npm publish` → triggers prepublishOnly hook
5. prepublishOnly runs `tsc` → SUCCESS (dependencies available)
6. prepublishOnly runs `npm audit` → SUCCESS (changed from pnpm to npm)
7. Publish completes

### Additional Issue #1: Root workspace prepare hook (Workflow Run #19055002462)

After adding `npm install`, a new issue appeared:
```
> prepare
> husky

sh: 1: husky: not found
npm error code 127
```

**Root Cause**: The monorepo's root package.json has a "prepare" script that runs "husky" (git hooks tool). When running `npm install` in the packages/maproom-mcp directory within the workspace, npm triggers the root's prepare script, but husky isn't installed in GitHub Actions.

**Solution**: Use `npm install --ignore-scripts` to:
- Skip the "prepare" hook during dependency installation
- Still allow "prepublishOnly" to run during publish (publish-specific hooks aren't affected)

### Additional Issue #2: prepublishOnly uses pnpm audit (Workflow Run #19055207167)

After fixing the prepare hook issue, a third issue appeared:
```
> @crewchief/maproom-mcp@1.3.0 prepublishOnly
> tsc && pnpm audit --audit-level=high --prod

sh: 1: pnpm: not found
npm error code 127
```

**Root Cause**: The package.json prepublishOnly script uses `pnpm audit` but the GitHub Actions workflow uses npm, not pnpm. pnpm is not installed in the workflow runner.

**Solution**: Change the prepublishOnly script in package.json to use npm audit:
- Changed from: `"prepublishOnly": "tsc && pnpm audit --audit-level=high --prod"`
- Changed to: `"prepublishOnly": "tsc && npm audit --audit-level=high --production"`
- Note: npm uses `--production` instead of `--prod`

### Solution Implemented

Added Step 9 before the publish step:

```yaml
# Step 9: Install dependencies for prepublishOnly hook
- name: Install dependencies
  if: inputs.dry_run != 'true'
  working-directory: packages/maproom-mcp
  run: npm install --ignore-scripts
```

The `--ignore-scripts` flag:
- Skips "prepare", "install", and "postinstall" hooks during dependency installation
- Prevents the root workspace's "prepare" hook (husky) from running
- Still allows "prepublishOnly" to run during `npm publish` (publish hooks are separate)
- Ensures TypeScript, type definitions, and all dependencies are available for prepublishOnly

## Implementation Notes

### Why prepublishOnly Exists
The prepublishOnly hook serves two purposes:
1. **Rebuild from source** - Ensures the published package includes the latest compiled code
2. **Security audit** - Runs `pnpm audit` to catch known vulnerabilities before publishing

In our workflow, the first purpose is redundant (we already built and included dist/), but the second is valuable. The simplest fix is to install dependencies so both checks can run.

### Alternative Solutions Considered

**Option 1**: Remove tsc from prepublishOnly
```json
"prepublishOnly": "pnpm audit --audit-level=high --prod"
```
- Pro: No need to install dependencies
- Con: Loses rebuild safety check

**Option 2**: Use npm pack + npm publish from tarball
```bash
TARBALL=$(npm pack)
npm publish "$TARBALL"
```
- Pro: Skips prepublishOnly entirely
- Con: Loses security audit check

**Selected Option**: Install dependencies (implemented)
- Pro: Keeps all safety checks
- Pro: Minimal change to workflow
- Con: Adds ~10 seconds to workflow (acceptable)

## Dependencies
- BINPKG-1902 (dead code fix) - COMPLETED
- BINPKG-1903 (vendored OpenSSL) - COMPLETED
- BINPKG-1904 (cross-arch validation) - COMPLETED
- BINPKG-1905 (tarball verification) - COMPLETED

## Blocks
- BINPKG-1901 (canary release test)
- BINPKG-5002 (production release)

## Risk Assessment
- **Risk**: Very low - standard npm workflow pattern
- **Impact**: Unblocks publishing completely

## Files to Modify
- `.github/workflows/build-and-publish-maproom-mcp.yml` (added Step 9, renumbered Steps 10-12)
- `packages/maproom-mcp/package.json` (changed prepublishOnly to use npm audit instead of pnpm audit)

## Verification
After implementing:
1. Trigger workflow with manual dispatch (dry_run: true)
2. Verify "Install dependencies" step runs and succeeds
3. Verify "Publish to npm" step runs prepublishOnly successfully
4. Verify TypeScript compilation succeeds
5. Verify pnpm audit runs
6. Verify workflow completes successfully in dry-run mode

## Priority
**CRITICAL** - Final blocker for automated release pipeline. All previous issues resolved, this is the last fix needed.
