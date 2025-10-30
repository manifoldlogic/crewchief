# Ticket: MCPSTART-6004: Publish v1.1.9 to npm with 2FA

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer (human input required for 2FA)
- verify-ticket

## Summary
Build, test, and publish the fixed package to npm registry as version 1.1.9, making the Ollama startup fix available to all users via `npx @crewchief/maproom-mcp@latest`.

## Background
After all implementation and testing is complete, this ticket publishes v1.1.9 to npm so users can immediately benefit from the critical Ollama startup fix. This is the final step in the MCPSTART project, delivering the solution to production.

## Acceptance Criteria
- [ ] All Phase 1-5 tickets completed and committed to the repository
- [ ] Integration tests pass (MCPSTART-4002)
- [ ] Manual test with real MCP client passes (Claude Desktop or Cursor)
- [ ] Version bumped to 1.1.9 in package.json
- [ ] CHANGELOG updated with v1.1.9 entry (MCPSTART-6003)
- [ ] README updated with troubleshooting guide (MCPSTART-6001)
- [ ] Configuration examples file created (MCPSTART-6002)
- [ ] npm audit passes with no high/critical vulnerabilities
- [ ] Package built successfully (pnpm build)
- [ ] Package published to npm registry with 2FA verification
- [ ] Verified published package works via npx in clean environment

## Technical Requirements

### Pre-Publish Checklist

```bash
cd packages/maproom-mcp

# 1. Verify all tests pass
bash tests/startup-integration.sh

# 2. Verify build succeeds
pnpm build

# 3. Verify audit passes (should already pass via prepublishOnly hook)
npm audit --audit-level=high

# 4. Manual test with actual MCP client
# Test each provider configuration:
EMBEDDING_PROVIDER=ollama npx . # Should start Ollama
EMBEDDING_PROVIDER=google GOOGLE_CLOUD_PROJECT=test npx . # Should NOT start Ollama
EMBEDDING_PROVIDER=openai OPENAI_API_KEY=sk-test npx . # Should NOT start Ollama

# 5. Verify config auto-update works
rm -rf ~/.maproom-mcp
npx . # Should create fresh configs
# Verify configs are at latest version
```

### Publishing Steps

```bash
# 6. Bump version in package.json
npm version 1.1.9

# This will:
# - Update package.json version
# - Create git commit: "1.1.9"
# - Create git tag: "v1.1.9"

# 7. Push commit and tag
git push origin maproom-vamp
git push origin v1.1.9

# 8. Publish to npm (requires 2FA)
npm publish --otp=<2FA_CODE>

# Note: If 2FA code expires during publish, re-run with a fresh code
```

### Post-Publish Verification

```bash
# 9. Test the published package in clean environment
rm -rf ~/.maproom-mcp

# Test default (Ollama)
npx -y @crewchief/maproom-mcp@1.1.9
# Verify: Ollama starts

# Test Google provider
EMBEDDING_PROVIDER=google npx -y @crewchief/maproom-mcp@1.1.9
# Verify: Ollama does NOT start (this is the critical fix!)

# Test OpenAI provider
EMBEDDING_PROVIDER=openai npx -y @crewchief/maproom-mcp@1.1.9
# Verify: Ollama does NOT start

# 10. Verify package metadata on npm
# Visit: https://www.npmjs.com/package/@crewchief/maproom-mcp
# Check: Version shows 1.1.9, README displays correctly
```

## Implementation Notes

- **Human interaction required**: 2FA code must be entered during `npm publish`
- **Pre-publish hook**: npm audit runs automatically via prepublishOnly script (MCPSTART-5002)
- **Git workflow**: Publishing creates a git commit and tag, must be pushed to remote
- **Rollback plan**: If issues are discovered post-publish, can publish 1.1.10 as a patch
- **Testing scope**: Test both "works correctly" (Ollama starts when selected) and "bug is fixed" (Ollama doesn't start when not selected)
- **Environment**: Use `npx -y` to force download of latest version and avoid cached installs
- **Timing**: Allow a few minutes after publish for npm CDN to propagate the new version

## Dependencies
- **ALL previous MCPSTART tickets must be complete** (Phases 1-6)
  - Phase 1: MCPSTART-1001, 1002, 1003, 1004
  - Phase 2: MCPSTART-2001, 2002, 2003
  - Phase 3: MCPSTART-3001, 3002, 3003
  - Phase 4: MCPSTART-4001, 4002
  - Phase 5: MCPSTART-5001, 5002, 5003
  - Phase 6: MCPSTART-6001, 6002, 6003

## Risk Assessment
- **Risk**: Medium - publishing is irreversible (cannot un-publish after 72 hours)
  - **Mitigation 1**: Thorough testing before publish (integration tests + manual tests)
  - **Mitigation 2**: Test published package immediately after publish
  - **Mitigation 3**: Monitor for issues in first 24 hours
  - **Rollback**: Can publish 1.1.10 patch if critical issues are found
  - **Communication**: Update CHANGELOG and GitHub issues after successful publish

- **Risk**: Low - 2FA code expiration during publish
  - **Mitigation**: Have authenticator app ready, publish during low-distraction time

- **Risk**: Low - npm registry issues or downtime
  - **Mitigation**: Check npm status page before publishing, retry if transient failure

## Files/Packages Affected
- `packages/maproom-mcp/package.json` (version bump)
- npm registry (new version published)
- Git tags (v1.1.9 tag created)

## Post-Publish Tasks
- [ ] Announce release in relevant channels (GitHub, Discord, etc.)
- [ ] Close MCP-008 and MCP-011 GitHub issues if they exist
- [ ] Monitor for user reports of issues in first 24-48 hours
- [ ] Update any external documentation that references the package
