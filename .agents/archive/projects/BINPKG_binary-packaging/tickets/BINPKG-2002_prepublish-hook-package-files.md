# Ticket: BINPKG-2002: Add prepublishOnly hook and update package.json files array

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update `packages/maproom-mcp/package.json` to run validation before any publish and simplify the files array. This integrates the validation script (BINPKG-2001) into the publish workflow.

## Background
The prepublishOnly hook is npm's standard mechanism for pre-publish checks. It runs automatically before `npm publish` or `pnpm publish`, providing a safety net against publishing incomplete packages. This ticket integrates the validation script created in BINPKG-2001 into the standard npm/pnpm publish workflow, ensuring that all platform binaries are validated before any package publication. We also simplify the files array from `"bin/**/*"` to just `"bin"` for cleaner configuration with equivalent behavior.

## Acceptance Criteria
- [ ] `prepublishOnly` script added to package.json scripts section with value: `"node ../../scripts/validate-binaries.js"`
- [ ] Files array updated to use `"bin"` instead of `"bin/**/*"`
- [ ] Hook runs automatically when executing `npm publish` or `pnpm publish`
- [ ] Hook blocks publish if validation fails (exit code 1)
- [ ] Hook allows publish if validation passes (exit code 0)
- [ ] Manual test completed: Delete one platform binary, attempt publish, verify blocked
- [ ] Manual test completed: With all binaries present, attempt publish (dry run), verify passes

## Technical Requirements
- File: `packages/maproom-mcp/package.json`
- Add to scripts section:
  ```json
  "prepublishOnly": "node ../../scripts/validate-binaries.js"
  ```
- Update files array to:
  ```json
  "files": [
    "bin",
    "config/docker-compose.yml",
    "config/Dockerfile.mcp-server",
    "config/init.sql",
    "dist/",
    "src/",
    "tsconfig.json",
    "README.md",
    "LICENSE"
  ]
  ```
- Script path must be relative to package.json location (../../ goes to repo root)
- Hook must exit with code 1 on validation failure to block publish
- Hook must exit with code 0 on validation success to allow publish

## Implementation Notes
- **prepublishOnly lifecycle**: Runs before both `npm publish` and `pnpm publish` (also runs for `npm pack`)
- **Script path resolution**: Path is relative to package.json location - `../../` navigates from `packages/maproom-mcp/` to repo root
- **Files array change**: `"bin"` includes all subdirectories automatically, equivalent to `"bin/**/*"` but cleaner
- **Testing approach**: Use `npm publish --dry-run` to test without actual publish (note: prepublishOnly still runs)
- **Validation behavior**: Script checks for all required platform binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64)

## Dependencies
- **BINPKG-2001** - Validation script must exist at `scripts/validate-binaries.js`
- Validation script must be executable and return proper exit codes

## Risk Assessment
- **Risk**: Relative path to validation script could break if package structure changes
  - **Mitigation**: Test script execution from package directory, document path requirements in README
- **Risk**: Hook might not run in all publish scenarios (CI/CD, different package managers)
  - **Mitigation**: Verify with npm/pnpm documentation, test manually with both package managers
- **Risk**: Hook could block legitimate publishes if validation script has bugs
  - **Mitigation**: Ensure validation script (BINPKG-2001) is thoroughly tested before integration

## Files/Packages Affected
- MODIFY: `packages/maproom-mcp/package.json`
  - Add prepublishOnly script
  - Update files array
