# Ticket: DKRHUB-1004: Implement Version Extraction and Tagging

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Extract semantic version from git tag and generate multiple Docker image tags (full version, minor version, major version, latest) for flexible version pinning.

## Background
When a git tag like `v1.1.10` is pushed, the workflow needs to:
1. Extract the version (1.1.10)
2. Parse semantic version components (major: 1, minor: 1.1, patch: 1.1.10)
3. Generate multiple Docker tags for flexibility

This enables users to pin to specific versions or follow minor/major updates automatically.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1004 (lines 175-206)

## Acceptance Criteria
- [x] Version extraction step added with id `version`
- [x] Full version extracted and stored in `$GITHUB_OUTPUT` as `full` (e.g., "1.1.10")
- [x] Major version extracted and stored as `major` (e.g., "1")
- [x] Minor version extracted and stored as `minor` (e.g., "1.1")
- [x] Supports both tag triggers (`v1.1.10`) and manual workflow_dispatch
- [x] Docker metadata action configured with `docker/metadata-action@v5`
- [x] Metadata generates tags: `{full}`, `{minor}`, `{major}`, `latest`
- [x] OCI labels included: title, description, vendor, version

## Technical Requirements
- Step: "Extract version"
  - id: `version`
  - Shell script to parse GITHUB_REF or workflow_dispatch input
  - Outputs: `full`, `minor`, `major`
  - Logic:
    ```bash
    if workflow_dispatch: VERSION = input.version
    else: VERSION = GITHUB_REF with 'refs/tags/v' prefix removed
    MINOR = first two components (cut -d. -f1-2)
    MAJOR = first component (cut -d. -f1)
    ```

- Step: "Generate Docker metadata"
  - uses: `docker/metadata-action@v5`
  - id: `meta`
  - inputs:
    - images: `${{ env.DOCKER_HUB_REPO }}`
    - tags: type=raw for each version variant
    - labels: OCI image labels

## Implementation Notes
The version extraction handles two trigger scenarios:
1. **Tag push**: Extract from `$GITHUB_REF` (refs/tags/v1.1.10)
2. **Manual dispatch**: Use `${{ github.event.inputs.version }}`

Multiple tags provide flexibility:
- `1.1.10`: Immutable, specific release (recommended for production)
- `1.1`: Moves with patch releases (1.1.10 → 1.1.11)
- `1`: Moves with minor/patch releases
- `latest`: Always points to newest release

OCI labels provide metadata for tools and users:
- `org.opencontainers.image.title`: "Maproom MCP Server"
- `org.opencontainers.image.description`: "Semantic code search MCP server with local LLM embeddings"
- `org.opencontainers.image.vendor`: "CrewChief"
- `org.opencontainers.image.version`: Version from extraction

Reference DKRHUB_ARCHITECTURE.md lines 545-610 for version management strategy.

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1003**: Authentication must be configured before version extraction
- **DKRHUB-1001**: Workflow environment variables must be defined

## Risk Assessment
- **Risk**: Version parsing fails on unexpected tag format
  - **Mitigation**: Use strict pattern matching in workflow trigger (v*.*.*), add validation
- **Risk**: Tag overwriting (e.g., re-pushing 1.1.10)
  - **Mitigation**: Docker Hub allows overwrites, but git tag protection prevents accidental re-tags
- **Risk**: Latest tag points to wrong version
  - **Mitigation**: Always push tags in order; latest is overwritten by each release

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml` (add version extraction and metadata steps)
