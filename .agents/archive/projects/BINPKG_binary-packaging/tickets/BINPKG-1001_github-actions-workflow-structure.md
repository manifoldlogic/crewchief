# Ticket: BINPKG-1001: Create GitHub Actions workflow structure and trigger configuration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Set up the foundational GitHub Actions workflow file that will orchestrate multi-platform binary builds and npm publishing for the maproom-mcp package. This workflow replaces the manual build process and ensures all 4 platform binaries are built before every release.

## Background
The current release process is unreliable - version 1.3.0 was published without linux-x64 binaries, causing production failures. The manual `scripts/build-and-package.sh` only builds for the current platform. We need automated CI builds for all 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64) that trigger automatically on version tags.

This ticket creates the workflow file structure with proper triggers, but individual platform builds will be implemented in subsequent tickets (BINPKG-1002-1005).

## Acceptance Criteria
- [x] Workflow file `.github/workflows/build-and-publish-maproom-mcp.yml` exists
- [x] Workflow triggers on git tags matching pattern `v*.*.*` (e.g., v1.2.3)
- [x] Workflow supports manual trigger (workflow_dispatch) with dry-run option
- [x] Workflow contains job structure: `build-binaries` (matrix), `validate-and-publish` (depends on build)
- [x] Matrix strategy defined with 4 platforms (implementation details in later tickets)
- [x] Workflow file follows GitHub Actions best practices (clear names, concurrency control)

## Technical Requirements

### File Location
- Create: `.github/workflows/build-and-publish-maproom-mcp.yml`

### Trigger Configuration
- **Automatic trigger**: Git tags matching `v*.*.*` pattern (e.g., v1.2.3, v1.4.0)
- **Manual trigger**: `workflow_dispatch` with boolean `dry_run` input

### Job Structure

**Job 1: `build-binaries`**
- Uses matrix strategy
- 4 platform configurations (implementation placeholders for now)
- Each matrix item defines:
  - `os`: GitHub runner OS (ubuntu-latest, macos-13, macos-latest)
  - `target`: Rust target triple
  - `platform`: Our naming convention (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
  - `use_cross`: Boolean indicating whether to use cross-compilation tool

**Job 2: `validate-and-publish`**
- Depends on: `build-binaries`
- Runs after all matrix builds complete
- Implementation details in BINPKG-1006-1007

### Matrix Configuration
Define 4 platform configurations:

1. **linux-x64**:
   - os: ubuntu-latest
   - target: x86_64-unknown-linux-gnu
   - platform: linux-x64
   - use_cross: true

2. **linux-arm64**:
   - os: ubuntu-latest
   - target: aarch64-unknown-linux-gnu
   - platform: linux-arm64
   - use_cross: true

3. **darwin-x64**:
   - os: macos-13
   - target: x86_64-apple-darwin
   - platform: darwin-x64
   - use_cross: false

4. **darwin-arm64**:
   - os: macos-latest
   - target: aarch64-apple-darwin
   - platform: darwin-arm64
   - use_cross: false

### Best Practices
- Add concurrency control to prevent multiple runs for same tag
- Use clear job and step names
- Add comments explaining workflow structure
- Group related steps logically
- Use environment variables for repeated values

## Implementation Notes

### Reference Materials
- **Existing workflow**: `.github/workflows/publish-maproom-mcp-image.yml` shows patterns used in this project
- **GitHub docs**: https://docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs
- **Planning doc**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 11-62)
- **Architecture doc**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 139-212)

### Workflow Structure Template
```yaml
name: Build and Publish Maproom MCP

on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (skip publish)'
        required: false
        type: boolean
        default: false

concurrency:
  group: publish-${{ github.ref }}
  cancel-in-progress: false

jobs:
  build-binaries:
    name: Build ${{ matrix.platform }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          # Matrix items defined as per Technical Requirements
          # Build steps are PLACEHOLDERS - will be implemented in BINPKG-1002-1005
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      # TODO: Add build steps in BINPKG-1002-1005

  validate-and-publish:
    name: Validate and Publish to npm
    runs-on: ubuntu-latest
    needs: build-binaries
    steps:
      # TODO: Add validation and publish steps in BINPKG-1006-1007
```

### Key Design Decisions

1. **Concurrency control**: Use `cancel-in-progress: false` to ensure publishes complete even if new tag is pushed
2. **Matrix naming**: Use `platform` field for our naming convention (linux-x64, etc.) distinct from OS and target
3. **Placeholder steps**: Leave build steps as TODOs with ticket references for traceability
4. **Clear dependencies**: `validate-and-publish` explicitly `needs: build-binaries`

### Future Ticket References
This ticket creates the structure. Implementation happens in:
- **BINPKG-1002**: Linux x64 build steps
- **BINPKG-1003**: Linux ARM64 build steps
- **BINPKG-1004**: macOS x64 build steps
- **BINPKG-1005**: macOS ARM64 build steps
- **BINPKG-1006**: Binary validation job
- **BINPKG-1007**: npm publish job

### Verification Steps
After creating the workflow file:
1. Check YAML syntax is valid (use yamllint or GitHub's syntax checker)
2. Verify trigger patterns match expected tag format
3. Verify matrix includes all 4 platforms with correct parameters
4. Verify job dependencies are correct (validate-and-publish needs build-binaries)
5. Verify concurrency control is configured
6. Verify placeholder comments reference correct future tickets

## Dependencies
**None** - This is the first ticket in the BINPKG project sequence

## Risk Assessment

- **Risk**: Trigger pattern conflicts with existing workflows
  - **Likelihood**: Low
  - **Impact**: Medium (could trigger multiple workflows)
  - **Mitigation**: Use specific tag pattern `v*.*.*`, add concurrency control, review existing workflows for conflicts

- **Risk**: Matrix configuration too complex for maintainers to understand
  - **Likelihood**: Medium
  - **Impact**: Low (slows future modifications)
  - **Mitigation**: Add comprehensive comments explaining each matrix item and why it's configured that way, follow GitHub documentation patterns

- **Risk**: Workflow file structure needs changes when implementing build steps
  - **Likelihood**: Medium
  - **Impact**: Low (requires rework)
  - **Mitigation**: Review architecture documentation before creating structure, leave flexibility for build step implementation, use clear TODOs

- **Risk**: Missing permissions or secrets needed for later steps
  - **Likelihood**: Low
  - **Impact**: Low (can add later)
  - **Mitigation**: Review publish workflow requirements before finalizing, document any needed secrets in future tickets

## Files/Packages Affected

### Files to Create
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Main workflow file

### Files to Reference (Read Only)
- `/workspace/.github/workflows/publish-maproom-mcp-image.yml` - Existing workflow patterns
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/plan.md` - Phase 1 planning
- `/workspace/.agents/projects/BINPKG_binary-packaging/planning/architecture.md` - Technical architecture

### Packages Affected
- `packages/maproom-mcp` - Target package for binary builds (no code changes)

## Estimated Effort
**1-2 hours** - Straightforward workflow structure creation

Breakdown:
- 30 min: Review existing workflows and planning docs
- 30 min: Create workflow YAML with structure and triggers
- 30 min: Configure matrix strategy with 4 platforms
- 15 min: Add comments and documentation
- 15 min: Verify YAML syntax and structure

## Priority
**High** - Foundation for all subsequent binary packaging work. Blocks BINPKG-1002-1007.

## Related Tickets

### Blocks (must be completed before these can start)
- BINPKG-1002: Implement Linux x64 build steps
- BINPKG-1003: Implement Linux ARM64 build steps
- BINPKG-1004: Implement macOS x64 build steps
- BINPKG-1005: Implement macOS ARM64 build steps
- BINPKG-1006: Implement binary validation
- BINPKG-1007: Implement npm publish

### Sequence
This is ticket 1 of 11 in Phase 1 of the BINPKG project:
1. **BINPKG-1001** (this ticket) - Workflow structure
2. BINPKG-1002-1005 - Platform build implementations
3. BINPKG-1006-1007 - Validation and publish

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 11-62)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 139-212)

### External References
- **GitHub Actions matrix strategy**: https://docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs
- **GitHub Actions workflow syntax**: https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions
- **Rust cross-compilation**: https://github.com/cross-rs/cross
