# Ticket: NPMDEP-3001: Apply npm Deprecation Warning to All Versions

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Execute npm deprecate command to apply deprecation warning to all versions of maproom-mcp package, then verify the deprecation message is properly displayed to users installing the old package.

## Background
Phase 3 - After publishing v2.0.0 under the new @crewchief/maproom-mcp namespace (NPMDEP-2001), apply deprecation warning to all versions of the old maproom-mcp package. This ensures existing users are notified when they install or use the old package. User specifically requested the --help reference and @crewchief/maproom-mcp package name in the deprecation message.

## Acceptance Criteria
- [ ] npm deprecate command executes successfully with exact message
- [ ] Message includes @crewchief/maproom-mcp package reference
- [ ] Message includes --help command reference
- [ ] npm view maproom-mcp deprecated returns the deprecation message
- [ ] npm install maproom-mcp shows deprecation warning to users
- [ ] Command output and verification results documented

## Technical Requirements
- Execute exact npm deprecate command: `npm deprecate maproom-mcp "This package has been replaced by @crewchief/maproom-mcp. Please use the new package: npx @crewchief/maproom-mcp --help"`
- Verify deprecation using `npm view maproom-mcp deprecated`
- Test deprecation warning in fresh install: `cd /tmp/test && npm install maproom-mcp`
- Document all output to deprecation-output.log
- Record verification steps in deprecation-verification.md

## Implementation Notes
- Use copy-paste for the exact deprecation message to avoid typos
- Command can be re-run if corrections needed
- No user interaction required
- Deprecation applies to all existing versions of maproom-mcp
- Message should guide users to both the new package name and basic usage

## Dependencies
- Blocks on NPMDEP-2001 (Publish v2.0.0 to npm)
- Blocks NPMDEP-4001 (Phase 4 work - if applicable)

## Risk Assessment
- **Risk**: Typo in deprecation message makes message unclear or unprofessional
  - **Mitigation**: Copy-paste the exact message provided, do not type manually. Verify output matches expected message exactly.

## Files/Packages Affected
- `deprecation-output.log` (new file - command output)
- `deprecation-verification.md` (new file - verification documentation)
- npm registry entry for `maproom-mcp` (all versions)
