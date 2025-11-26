# Ticket: SQLINFRA-1005: Update Docker Compose Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only); YAML syntax validated
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This is a documentation ticket; "Tests pass - N/A" applies
- Verification is visual inspection of comments and link validation

## Agents
- general-purpose
- unit-test-runner (N/A - documentation only)
- verify-ticket
- commit-ticket

## Summary
Add explanatory header comments to Docker compose files explaining when PostgreSQL is needed vs when SQLite is sufficient.

## Background
Docker compose files currently provide PostgreSQL infrastructure without context about when it's needed. Users may spin up Docker containers unnecessarily when SQLite would suffice for their use case.

This ticket adds minimal but clarifying documentation directly in the compose files, linking to SQLite as the default alternative for individual use.

Reference: [SQLINFRA Plan - Phase 3](../planning/plan.md#phase-3-docker-documentation)

## Acceptance Criteria
- [ ] `config/docker-compose.yml` has explanatory header comment
- [ ] `packages/vscode-maproom/config/docker-compose.yml` has comment linking to SQLite option
- [ ] Comments clearly explain when Docker/PostgreSQL is needed
- [ ] Comments link to relevant SQLite documentation
- [ ] No functional changes to Docker configurations
- [ ] Docker compose files remain valid YAML

## Technical Requirements
- **`config/docker-compose.yml`**:
  - Add YAML comment block at top explaining:
    - PostgreSQL is for team sharing / multi-user scenarios
    - For individual use, SQLite works without Docker
    - Link to README Quick Start for SQLite path
  - Example:
    ```yaml
    # PostgreSQL for Maproom - Team Sharing Setup
    #
    # This Docker Compose provides PostgreSQL with pgvector for shared team indices.
    #
    # For individual use, SQLite works without Docker:
    #   crewchief maproom:scan /path/to/repo
    #   (Database auto-created at ~/.maproom/maproom.db)
    #
    # See README.md "Quick Start" for SQLite setup.
    # See docs/architecture/DATABASE_ARCHITECTURE.md for backend comparison.
    ```

- **`packages/vscode-maproom/config/docker-compose.yml`**:
  - Add similar header comment
  - Reference that VSCode extension supports SQLite-first activation
  - Link to extension documentation

- **No Functional Changes**:
  - Only add comments
  - Do not modify service definitions, ports, volumes, etc.

## Implementation Notes
- YAML comments start with `#`
- Place comments at the very top of file, before `version:` or `services:`
- Keep comments concise (5-10 lines max)
- Ensure links are relative to repo root or use full documentation paths
- Test that docker compose still parses correctly after changes:
  ```bash
  docker compose -f config/docker-compose.yml config
  docker compose -f packages/vscode-maproom/config/docker-compose.yml config
  ```

## Dependencies
- **SQLINFRA-1003**: Should reference README patterns established in that ticket

## Risk Assessment
- **Risk**: Comments break YAML parsing
  - **Mitigation**: Validate with `docker compose config` after changes
- **Risk**: Comments become outdated
  - **Mitigation**: Keep comments high-level; link to docs rather than duplicating details

## Files/Packages Affected
- `config/docker-compose.yml` - Add header comments
- `packages/vscode-maproom/config/docker-compose.yml` - Add header comments
