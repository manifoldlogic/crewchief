# Ticket: MCPSTART-5002: Add npm audit check to prepublishOnly script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Add npm audit to the prepublishOnly script to prevent publishing the Maproom MCP package with known high or critical vulnerabilities. This implements supply chain security best practices.

## Background
From MCPSTART_SECURITY_REVIEW.md Section 6 (Supply Chain Security) - the package currently has no automated vulnerability checking before publish. Running npm audit before publishing catches vulnerable dependencies before they reach users, preventing the distribution of packages with known security issues.

This implements Phase 5 (Security Hardening) of the MCPSTART project plan.

## Acceptance Criteria
- [ ] Add `npm audit --audit-level=high` to prepublishOnly script
- [ ] Script fails the publish process if high or critical vulnerabilities are found
- [ ] Create separate `security-check` script for manual audits (moderate+ level)
- [ ] Document the audit process in README.md
- [ ] Verify that current dependencies pass the high-level audit
- [ ] Test that prepublishOnly script correctly blocks publish on vulnerability

## Technical Requirements

Update `packages/maproom-mcp/package.json` scripts section:

```json
{
  "scripts": {
    "prepublishOnly": "tsc && npm audit --audit-level=high",
    "security-check": "npm audit --audit-level=moderate",
    "build": "tsc",
    "start": "node dist/index.js"
  }
}
```

## Implementation Notes
- `npm audit --audit-level=high` will exit with non-zero status if high or critical vulnerabilities exist, preventing publish
- The `security-check` script uses moderate level for more thorough manual checks during development
- The prepublishOnly hook runs automatically before `npm publish`
- This prevents accidental publication of vulnerable packages
- Developers can still override in emergency situations using `npm publish --force`, but this should be documented as dangerous
- Consider adding `npm audit fix` guidance to documentation for resolving vulnerabilities

## Dependencies
None - this is an independent security hardening change

## Risk Assessment
- **Risk**: Low - prevents bad publishes without affecting development workflow
  - **Mitigation**: Developers can run `npm audit fix` to resolve issues, or temporarily remove check in genuine emergencies
- **Risk**: False positives may block legitimate publishes
  - **Mitigation**: Use `--audit-level=high` to focus on critical issues; document override process for false positives
- **Risk**: Audit may fail due to transitive dependencies outside our control
  - **Mitigation**: Document process for handling unavoidable vulnerabilities (e.g., checking for exploitability, filing issues with upstream)

## Files/Packages Affected
- `packages/maproom-mcp/package.json`
- `packages/maproom-mcp/README.md` (documentation of audit process)
