# Ticket: NPMDEP-1003: Validate Deprecation Package Locally Before Publishing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - validation tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Build the npm package locally, extract and verify contents, test executable functionality including --help flag, and validate package size before publishing. This is the final quality gate before irreversible npm publishing.

## Background
Phase 1.3 - Local Testing and Validation. This is a critical quality gate since npm publishes are irreversible. Before pushing to npm registry, we must verify that:
- Package builds correctly with `npm pack`
- Contains exactly the correct files in the correct structure
- Executable works and shows deprecation message
- --help flag works correctly
- Exit codes are correct (code 1 for both normal and --help cases)
- Package size is reasonable (< 50 KB)
- All JSON is valid
- Permissions are correct

This ticket implements Phase 1.3 from the NPMDEP_PLAN.md. If validation fails at any point, we return to NPMDEP-1002 to fix issues before attempting publishing again.

## Acceptance Criteria
- [ ] `npm pack` succeeds in `/tmp/maproom-mcp-deprecated/`
- [ ] Package file `maproom-mcp-2.0.0.tgz` created and size < 50 KB
- [ ] Extracted package contains exactly 3 files: package.json, index.js, README.md
- [ ] `node index.js` shows deprecation message and exits with code 1
- [ ] `node index.js --help` shows help-specific message and exits with code 1
- [ ] package.json is valid JSON with required fields (name, version, deprecated, bin)
- [ ] index.js has executable permissions (755)
- [ ] README.md content is correct and complete
- [ ] Validation report created at `.agents/projects/NPMDEP_npm-deprecation/validation-report.md`

## Technical Requirements
- Use `npm pack` to create the tarball
- Use `tar -xzf` to extract and verify structure
- Test exit codes with `$?` variable or `|| echo` patterns
- Validate JSON structure with `jq` or JSON parser
- Check file permissions with `ls -la`
- Document all test results with timestamps
- Generate validation report with pass/fail status for each check

## Implementation Notes
- This is the last chance to catch errors before publishing to npm
- All validation checks must pass before proceeding to Phase 2 (NPMDEP-2001)
- If any validation fails, document the failure and return to NPMDEP-1002 to fix
- Use `/tmp/maproom-mcp-deprecated/` as the working directory (should already exist from previous tickets)
- Create a comprehensive validation-report.md documenting all tests run, results, and any issues found
- Test both normal invocation and --help flag to ensure both paths work correctly
- Verify exit codes specifically (code 1 expected in both cases)
- Check that the deprecation message is clear and directs users appropriately

## Dependencies
- **Blocks on**: NPMDEP-1002 (Content Preparation and Package Building)
- **Blocks**: NPMDEP-2001 (Publish to npm Registry)

## Risk Assessment
- **Risk**: Validation reveals missing or incorrect files in package
  - **Mitigation**: Fix the issue in NPMDEP-1002 and rebuild; re-run validation
- **Risk**: Executable flag not set on index.js
  - **Mitigation**: Add `chmod +x` step in NPMDEP-1002 build process
- **Risk**: package.json is malformed or missing required fields
  - **Mitigation**: Validate package.json schema before packing; fix in NPMDEP-1002
- **Risk**: Package size exceeds limits
  - **Mitigation**: Remove unnecessary files; verify only needed files are included
- **Risk**: Exit codes not correct (should be 1, not 0)
  - **Mitigation**: Update index.js error handling; verify with `process.exit(1)`

## Files/Packages Affected
- `/tmp/maproom-mcp-deprecated/maproom-mcp-2.0.0.tgz` (created during `npm pack`)
- `/tmp/maproom-mcp-deprecated/package/` (temporary extraction for validation)
- `.agents/projects/NPMDEP_npm-deprecation/validation-report.md` (new file created)
