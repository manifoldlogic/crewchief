# Ticket: NPMDEP-2001: Publish maproom-mcp Version 2.0.0 to npm Registry

## Status
- [ ] **Task completed** - BLOCKED: requires user authentication and approval
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## ⚠️ BLOCKER: USER INTERACTION REQUIRED

This ticket cannot be completed autonomously. The user must:
1. Run `npm login` with credentials for daniel.bushman account
2. Explicitly approve publishing (irreversible action)
3. Execute `npm publish` from /tmp/maproom-mcp-deprecated/
4. Verify publication succeeded

See publish-verification.md for complete instructions.

## Agents
- general-purpose (user interaction required)
- verify-ticket
- commit-ticket

## Summary
Perform final pre-publish verification, publish maproom-mcp v2.0.0 to npm registry, and immediately verify the package appears correctly in the registry and on the npm website.

## Background
Phase 2 - Publishing. This is the **irreversible step** that makes the deprecation package publicly available. Must verify npm authentication, publish rights, then publish and validate. After this completes, version 2.0.0 cannot be unpublished after 72 hours per npm policy.

This ticket requires **user interaction** for npm credentials and publish approval.

From NPMDEP_PLAN.md Phase 2: Publish v2.0.0 to npm registry with full pre-publish verification and immediate registry validation.

## Acceptance Criteria
- [ ] `npm whoami` succeeds (user authenticated)
- [ ] `npm owner ls maproom-mcp` confirms user has publish rights
- [ ] User explicitly approves proceeding with publish
- [ ] `npm publish` command succeeds from `/tmp/maproom-mcp-deprecated/`
- [ ] `npm view maproom-mcp@2.0.0` returns package metadata
- [ ] npm website https://www.npmjs.com/package/maproom-mcp shows v2.0.0
- [ ] README.md visible and rendered correctly on npm website
- [ ] `deprecated` field visible in package metadata
- [ ] Publish output captured and documented

## Technical Requirements
**Pre-publish verification:**
```bash
npm whoami  # Verify authentication
npm owner ls maproom-mcp  # Verify publish rights
```

**Publish command:**
```bash
cd /tmp/maproom-mcp-deprecated
npm publish 2>&1 | tee publish-output.log
```

**Immediate verification:**
```bash
npm view maproom-mcp@2.0.0  # Verify appears in registry
npm view maproom-mcp@2.0.0 version  # Should show: 2.0.0
npm view maproom-mcp@2.0.0 deprecated  # Should show deprecation message
```

**Web verification:**
- Visit https://www.npmjs.com/package/maproom-mcp
- Verify v2.0.0 shows as latest
- Verify README displays deprecation notice
- Verify "DEPRECATED" badge visible

## Implementation Notes
- **USER INTERACTION REQUIRED** - This ticket cannot be fully automated
- User must be present for npm login if needed
- User must explicitly approve the publish action
- Capture all command output for documentation
- If publish fails, document error and do not retry without analysis
- Cannot undo publish after 72 hours - verify everything carefully
- Save publish-output.log to project directory for audit trail

## Dependencies
- **Blocks on:** NPMDEP-1003 (local testing must pass first)
- **Required:** User must be available for interaction
- **Blocks:** NPMDEP-3001 (can't deprecate until published)

## Risk Assessment
- **Risk**: User lacks npm credentials
  - **Mitigation**: Verified in NPMDEP-1001, have user run `npm login` now
  - **Impact**: High - blocks publish
- **Risk**: Publish fails due to network/registry issues
  - **Mitigation**: Check https://status.npmjs.org/ first
  - **Fallback**: Wait and retry
- **Risk**: Wrong content published
  - **Mitigation**: NPMDEP-1003 validation reduces this risk
  - **Fallback**: Publish 2.0.1 with corrections (cannot unpublish)
- **Risk**: Version 2.0.0 already exists
  - **Mitigation**: Checked in NPMDEP-1001
  - **Fallback**: Use 2.0.1 or 3.0.0 instead

## Files/Packages Affected
- `/tmp/maproom-mcp-deprecated/` (publish from here)
- `.agents/projects/NPMDEP_npm-deprecation/publish-output.log` (new, captured output)
- `.agents/projects/NPMDEP_npm-deprecation/publish-verification.md` (new, verification results)
