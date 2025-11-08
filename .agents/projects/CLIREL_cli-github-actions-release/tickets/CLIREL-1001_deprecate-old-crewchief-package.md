# Ticket: CLIREL-1001: Deprecate old crewchief package with migration warnings

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Publish final version of the `crewchief` npm package (v1.0.0) with deprecation warnings to guide users to the new `@crewchief/cli` scoped package. This is a one-time manual operation to provide a responsible migration path before the new package launches.

## Background
The CLI package is being renamed from `crewchief` to `@crewchief/cli` to follow org naming conventions. Since this is a breaking change (users must update their package.json), we need to:
1. Signal the breaking change with v1.0.0 (semantic versioning)
2. Display clear migration warnings on install
3. Mark the old package as deprecated on npm registry
4. Provide migration instructions

This ensures existing users (if any) have a clear path forward and aren't left with a silent, abandoned package.

This ticket implements Phase 1 of the CLIREL project: "Old Package Deprecation" - Publish `crewchief@1.0.0` with deprecation warnings.

## Acceptance Criteria
- [ ] `crewchief@1.0.0` published to npm registry
- [ ] postinstall script displays deprecation warning with new package name
- [ ] npm package page shows deprecation notice
- [ ] Warning message is clear and actionable (includes migration steps)
- [ ] Package marked as deprecated via `npm deprecate` command

## Technical Requirements

1. **Create deprecation package**:
   - Copy current CLI package to temporary location
   - Update package.json:
     ```json
     {
       "name": "crewchief",
       "version": "1.0.0",
       "deprecated": "This package has been renamed to @crewchief/cli"
     }
     ```

2. **Add postinstall warning**:
   - Create `postinstall.js`:
     ```javascript
     console.warn('\n⚠️  DEPRECATION WARNING ⚠️');
     console.warn('The "crewchief" package has been renamed to "@crewchief/cli"');
     console.warn('Please update your package.json:');
     console.warn('  npm uninstall crewchief');
     console.warn('  npm install @crewchief/cli\n');
     ```
   - Add to package.json: `"postinstall": "node postinstall.js"`

3. **Publish and deprecate**:
   ```bash
   npm version 1.0.0
   npm publish
   npm deprecate crewchief@1.0.0 "Package renamed to @crewchief/cli. Install @crewchief/cli instead."
   ```

## Implementation Notes
- This is a manual, one-time operation
- No automation needed (not worth the complexity)
- Keep it simple - just publish, deprecate, verify
- Low risk since likely no production users yet
- The deprecation package should be minimal - just enough to show the warning
- Use a temporary working directory outside the main codebase
- After publishing, verify with `npm view crewchief` and `npm install crewchief` (in a test directory)

## Dependencies
None (can be done immediately)

## Risk Assessment
- **Risk**: Users confused by deprecation message
  - **Mitigation**: Clear, actionable migration instructions with step-by-step commands
- **Risk**: Old package still being used long-term
  - **Mitigation**: Deprecation notice on npm registry prevents new usage and warns existing users
- **Risk**: Publish fails due to permissions
  - **Mitigation**: Verify npm authentication and package ownership before attempting publish

## Files/Packages Affected
- Temporary working directory for deprecation package (outside main codebase)
- npm registry (crewchief package)
- No files in the main CrewChief repository will be modified
