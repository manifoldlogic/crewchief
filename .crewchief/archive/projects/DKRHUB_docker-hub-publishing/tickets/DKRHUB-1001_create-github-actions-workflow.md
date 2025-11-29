# Ticket: DKRHUB-1001: Create GitHub Actions Workflow File

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create `.github/workflows/publish-maproom-mcp-image.yml` with complete workflow configuration for automated Docker image publishing to Docker Hub.

## Background
The v1.1.9 release of @crewchief/maproom-mcp is broken because docker-compose.yml attempts to build images from source using a build context that doesn't exist in deployed npm packages. This ticket implements the foundation of the Docker Hub publishing solution by creating the GitHub Actions workflow file that will automate multi-platform image builds and pushes.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1001 (lines 85-108)

## Acceptance Criteria
- [x] File created at `.github/workflows/publish-maproom-mcp-image.yml`
- [x] Workflow triggers on version tags matching pattern `v*.*.*`
- [x] Manual trigger available via workflow_dispatch with version and push_to_registry inputs
- [x] Environment variables defined: DOCKER_HUB_REPO, DOCKERFILE_PATH (Dockerfile.combined), BUILD_CONTEXT (workspace root)
- [x] Permissions correctly scoped: contents: read, packages: write, security-events: write

## Technical Requirements
- Workflow name: "Publish Maproom MCP Docker Image"
- Trigger patterns:
  - push.tags: `v*.*.*`
  - workflow_dispatch with inputs: version (required), push_to_registry (default: false)
- Environment variables:
  - DOCKER_HUB_REPO: `crewchief/maproom-mcp`
  - DOCKERFILE_PATH: `packages/maproom-mcp/config/Dockerfile.combined`
  - BUILD_CONTEXT: `.` (workspace root, required for Rust + Node.js builds)
- Jobs: Single job named `build-and-push` running on `ubuntu-latest`
- Initial steps placeholder (will be filled in subsequent tickets)

## Implementation Notes
This is the foundation ticket that creates the workflow file structure. Subsequent tickets (1002-1006) will add specific build steps, authentication, version extraction, and image pushing logic.

The workflow_dispatch trigger is essential for testing - it allows manual runs without pushing to production, enabling validation before the actual v1.1.10 release.

Reference DKRHUB_ARCHITECTURE.md lines 93-250 for complete workflow specification.

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist before workflow can reference it
- **DKRHUB-1007**: Local Dockerfile testing should pass before GitHub Actions implementation
- GitHub repository must have DOCKERHUB_USERNAME and DOCKERHUB_TOKEN secrets configured (prerequisite - already completed)

## Risk Assessment
- **Risk**: Workflow syntax errors could prevent execution
  - **Mitigation**: Validate YAML syntax before committing, test with workflow_dispatch
- **Risk**: Incorrect permissions could block security scan uploads
  - **Mitigation**: Use minimal required permissions as specified in DKRHUB_SECURITY_REVIEW.md

## Files/Packages Affected
- NEW: `.github/workflows/publish-maproom-mcp-image.yml`
