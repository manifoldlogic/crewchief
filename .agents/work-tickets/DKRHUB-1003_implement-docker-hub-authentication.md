# Ticket: DKRHUB-1003: Implement Docker Hub Authentication

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Configure Docker Hub login in the GitHub Actions workflow using credentials stored in GitHub Secrets (DOCKERHUB_USERNAME and DOCKERHUB_TOKEN).

## Background
Before pushing images to Docker Hub, the workflow must authenticate using an access token (not password). GitHub Secrets provide secure storage for credentials, ensuring they are never exposed in logs or code.

This ticket implements the authentication step required before image push operations.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1003 (lines 144-173)

## Acceptance Criteria
- [ ] Docker login step added using `docker/login-action@v3`
- [ ] Username configured from `${{ secrets.DOCKERHUB_USERNAME }}`
- [ ] Password/token configured from `${{ secrets.DOCKERHUB_TOKEN }}`
- [ ] Authentication succeeds when workflow runs
- [ ] No credentials visible in workflow logs (verified by checking log output)

## Technical Requirements
- Action: `docker/login-action@v3`
- Step name: "Login to Docker Hub"
- Inputs:
  - username: `${{ secrets.DOCKERHUB_USERNAME }}`
  - password: `${{ secrets.DOCKERHUB_TOKEN }}`
- Step position: After Docker Buildx setup, before version extraction
- No conditional logic required (always run on workflow execution)

## Implementation Notes
The docker/login-action automatically:
- Logs into Docker Hub registry
- Stores credentials in Docker config
- Redacts sensitive values from logs
- Logs out at the end of the job

GitHub Secrets (DOCKERHUB_USERNAME and DOCKERHUB_TOKEN) are already configured in the repository settings (prerequisite completed). These secrets:
- Are encrypted at rest
- Only accessible to workflows in the same repository
- Never exposed in forks or pull requests from forks

Security best practices from DKRHUB_SECURITY_REVIEW.md (lines 105-176):
- Use access tokens, not passwords
- Token has limited scope (read/write only)
- Token should be rotated annually
- 2FA enabled on Docker Hub account

## Dependencies
- DKRHUB-1002: Buildx must be configured before authentication
- Prerequisite: GitHub Secrets DOCKERHUB_USERNAME and DOCKERHUB_TOKEN must exist (already completed)

## Risk Assessment
- **Risk**: Secrets not configured or invalid
  - **Mitigation**: Verify secrets exist in repository settings before testing workflow
- **Risk**: Token expired or revoked
  - **Mitigation**: Test authentication in manual workflow_dispatch run before tagging release
- **Risk**: Credentials leaked in logs
  - **Mitigation**: docker/login-action automatically redacts secrets; verify by inspecting logs

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml` (add login step)
