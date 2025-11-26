# Docker-in-Docker Workspace Path Detection

**Project Slug:** DINDFX
**Status:** Planning
**Owner:** docker-engineer

## Problem Statement

When users run `npx @crewchief/maproom-mcp setup --provider=openai` inside a devcontainer, the Docker-in-Docker environment causes volume mount failures because:

1. `/workspace` only exists inside the devcontainer, not on the actual Docker host
2. The `WORKSPACE_HOST_PATH` environment variable is not automatically set
3. Volume mount `${WORKSPACE_HOST_PATH:-/workspace}:/workspace:ro` defaults to non-existent `/workspace`
4. The maproom-mcp container starts but cannot access any workspace files

## Proposed Solution

Implement automatic Docker-in-Docker detection and host path discovery in `bin/cli.cjs`:

1. **Detect Docker-in-Docker**: Check for `/.dockerenv` or `/run/.containerenv`
2. **Discover host path**: Use `docker inspect $(hostname)` to find the actual host mount source
3. **Set environment variable**: Export `WORKSPACE_HOST_PATH` before `docker compose up`
4. **Test-driven approach**: Write failing test first, then implement fix

## Success Criteria

- Test proves the problem exists (fails without fix)
- Test passes after implementing the fix
- Normal user flow works: `npx @crewchief/maproom-mcp setup --provider=openai`
- No manual configuration required
- Works in both devcontainer and host environments

## Planning Documents

- [Analysis](planning/analysis.md) - Problem investigation and industry research
- [Architecture](planning/architecture.md) - Solution design and implementation approach
- [Quality Strategy](planning/quality-strategy.md) - Test strategy and validation
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Execution phases and deliverables

## Relevant Agents

- **docker-engineer**: Container configuration and Docker-in-Docker expertise
- **unit-test-runner**: Execute tests to validate solution
- **verify-ticket**: Ensure acceptance criteria are met
