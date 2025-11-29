# Ticket: NPMDEP-1002: Create Deprecation Package Content Files

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Create the three files needed for the maproom-mcp v2.0.0 deprecation package: package.json with metadata, index.js executable with migration message, and README.md with detailed deprecation notice.

## Background

This is Phase 1.2 (Content Preparation) of the NPMDEP project. After verifying npm state in NPMDEP-1001, we now create the actual package content that will be published.

The deprecation package needs three files:
1. **package.json** - Version 2.0.0 with deprecated field and bin entry pointing to index.js
2. **index.js** - Executable script showing migration message (with --help flag support as requested)
3. **README.md** - Full deprecation notice for npm package page

Detailed specifications are in `planning/architecture.md` sections "File Specifications" (lines 291-355).

## Acceptance Criteria

- [ ] Directory `/tmp/maproom-mcp-deprecated/` created
- [ ] `package.json` created with version 2.0.0, deprecated field, and correct bin entry
- [ ] `index.js` created with shebang, executable permissions, exit code 1
- [ ] `index.js` handles both normal execution and --help flag correctly
- [ ] `README.md` created from `/workspace/packages/maproom-mcp/README.deprecated.md`
- [ ] All files contain correct content per architecture.md specifications
- [ ] No unexpected files in directory (clean package)

## Technical Requirements

**package.json specifications:**
- Version: `2.0.0` (major bump signals breaking change)
- `type`: `commonjs`
- `name`: `maproom-mcp`
- `description`: `"DEPRECATED: Use @crewchief/maproom-mcp instead"`
- `main`: `index.js`
- `bin`: Maps `maproom-mcp` command to `./index.js`
- `files`: Include only `index.js` and `README.md`
- `keywords`: `["deprecated", "maproom", "mcp", "semantic-search"]`
- `author`: `CrewChief`
- `license`: `MIT`
- `repository`: Points to github.com/danielbushman/crewchief.git
- `bugs`: Points to github.com/danielbushman/crewchief/issues
- `homepage`: github.com/danielbushman/crewchief/tree/main/packages/maproom-mcp
- `deprecated`: `"This package has been renamed to @crewchief/maproom-mcp"`

**index.js specifications:**
- Must have shebang: `#!/usr/bin/env node`
- Must be made executable: `chmod +x index.js`
- Must extract CLI arguments: `process.argv.slice(2)`
- Must detect --help or -h flag: `args.includes('--help') || args.includes('-h')`
- Must show help-specific message for --help flag
- Must show migration instructions for normal execution
- Must use stderr for output: `console.error()` (standard for warnings/errors)
- Must exit with code 1: `process.exit(1)` (signal deprecation/error)
- Exact implementation specified in architecture.md lines 331-351

**README.md:**
- Copy from `/workspace/packages/maproom-mcp/README.deprecated.md`
- No modifications needed (already correct)

## Implementation Notes

**Execution steps:**
1. Create directory: `mkdir -p /tmp/maproom-mcp-deprecated`
2. Create package.json with exact content from architecture.md specifications (lines 295-327)
3. Create index.js with exact content from architecture.md specifications (lines 331-351)
4. Set executable permissions: `chmod +x /tmp/maproom-mcp-deprecated/index.js`
5. Copy README from existing file: `cp /workspace/packages/maproom-mcp/README.deprecated.md /tmp/maproom-mcp-deprecated/README.md`
6. Verify no unexpected files: `ls -la /tmp/maproom-mcp-deprecated/` should show exactly 3 files

**Critical requirement:** User specifically requested --help flag support in index.js. This must be implemented exactly as specified.

**Verification:**
- Verify executable flag: `ls -la /tmp/maproom-mcp-deprecated/index.js` should show `rwxr-xr-x` or similar with execute bit
- Verify package.json is valid JSON: `node -e "require('/tmp/maproom-mcp-deprecated/package.json')"`
- Test execution: `node /tmp/maproom-mcp-deprecated/index.js` (should show migration message on stderr, exit 1)
- Test --help: `node /tmp/maproom-mcp-deprecated/index.js --help` (should show help-specific message on stderr, exit 1)

## Dependencies

- **Blocks on:** NPMDEP-1001 (npm state assessment must complete first)
- **Required files:** `/workspace/packages/maproom-mcp/README.deprecated.md` must exist

## Risk Assessment

- **Risk**: Typo in package.json or index.js
  - **Mitigation**: Copy exact content from architecture.md specifications, verify JSON validity
  - **Impact**: Medium - would require publishing 2.0.1 or 2.0.2 to fix

- **Risk**: Missing executable permissions on index.js
  - **Mitigation**: Explicit `chmod +x` step and verification with `ls -la`
  - **Impact**: High - npx and bin entry won't work without executable bit

- **Risk**: Wrong exit code or output stream
  - **Mitigation**: Follow specifications exactly (exit 1, use stderr with console.error())
  - **Impact**: Low - cosmetic issue, but important for user experience

- **Risk**: README file doesn't exist or can't be copied
  - **Mitigation**: Verify `/workspace/packages/maproom-mcp/README.deprecated.md` exists before creating ticket
  - **Impact**: Medium - would require manual creation

## Files/Packages Affected

- `/tmp/maproom-mcp-deprecated/package.json` (new file)
- `/tmp/maproom-mcp-deprecated/index.js` (new file, executable)
- `/tmp/maproom-mcp-deprecated/README.md` (new file, copied from existing)

## Related Planning Documents

- **Plan**: `.crewchief/projects/NPMDEP_npm-deprecation/planning/plan.md`
- **Architecture**: `.crewchief/projects/NPMDEP_npm-deprecation/planning/architecture.md` (sections "File Specifications" lines 291-355)
