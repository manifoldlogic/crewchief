# Ticket: MCPSTART-5003: Document security best practices in README

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
Add comprehensive security documentation to the Maproom MCP README, covering credential handling, network exposure, diagnostic logging, and security reporting. Help users make informed security decisions.

## Background
From MCPSTART_SECURITY_REVIEW.md - users need guidance on secure configuration practices. While the codebase implements security features (credential redaction, localhost binding, etc.), users need to understand these features and how to configure the system securely.

This implements Phase 5 (Security Hardening) of the MCPSTART project plan and should be completed after MCPSTART-5001 and MCPSTART-5002 so the documentation reflects the current security posture.

## Acceptance Criteria
- [ ] Add "Security Considerations" section to README.md
- [ ] Document credential management best practices (never commit, use env vars, rotation)
- [ ] Explain localhost binding and how to expose services if needed
- [ ] Document diagnostic mode redaction behavior for sensitive values
- [ ] Include security reporting contact information
- [ ] Link to external security best practices resources
- [ ] Document the npm audit prepublish check
- [ ] Add warnings about exposing services to network

## Technical Requirements

Add a comprehensive security section to `packages/maproom-mcp/README.md` covering:

### 1. Credentials Management
- Never commit credentials to version control
- Use `.env` files (git-ignored by default)
- Rotate credentials regularly
- Use unique credentials per environment
- Consider secret management tools for production

### 2. Network Security
- Services bound to localhost (127.0.0.1) by default
- How to expose services if needed (with warnings)
- Firewall considerations
- SSH tunneling for remote access

### 3. Diagnostic Logging
- Sensitive values redacted in logs
- What gets redacted (passwords, tokens, API keys)
- How to safely share diagnostic logs

### 4. Supply Chain Security
- npm audit runs before publish
- How to check for vulnerabilities manually
- Process for handling security advisories

### 5. Security Reporting
- Contact: security@crewchief.dev
- What to include in reports
- Expected response time

Use the template from MCPSTART_SECURITY_REVIEW.md lines 191-206 as a starting point.

## Implementation Notes
- Place the security section prominently in the README (before or after "Getting Started")
- Use clear, actionable language - focus on what users should do, not just what to avoid
- Include code examples for secure configuration
- Link to relevant Docker security documentation
- Consider adding a SECURITY.md file for detailed security policy
- Use warning callouts or emoji to highlight critical security information
- Keep the tone helpful, not alarmist - security should be approachable

## Dependencies
- MCPSTART-5001 (localhost binding should be documented)
- MCPSTART-5002 (npm audit should be documented)

## Risk Assessment
- **Risk**: None - documentation-only change
  - **Mitigation**: N/A
- **Risk**: Documentation may become outdated as features change
  - **Mitigation**: Add security documentation review to release checklist

## Files/Packages Affected
- `packages/maproom-mcp/README.md`
- Optionally: `packages/maproom-mcp/SECURITY.md` (if creating separate security policy file)
