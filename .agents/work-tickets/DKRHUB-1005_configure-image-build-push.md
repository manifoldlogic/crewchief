# Ticket: DKRHUB-1005: Configure Image Build and Push

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Configure the docker/build-push-action to build multi-platform images and push them to Docker Hub with proper caching, build arguments, and conditional push logic.

## Background
This is the core build step that:
1. Builds Docker images for linux/amd64 and linux/arm64
2. Tags images with all version variants
3. Pushes to Docker Hub (conditional on trigger type)
4. Uses GitHub Actions cache for performance

This completes the critical path for automated image publishing.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1005 (lines 208-245)

## Acceptance Criteria
- [ ] Build and push step added using `docker/build-push-action@v5`
- [ ] Build context set to workspace root (`.`)
- [ ] Dockerfile path set to `packages/maproom-mcp/config/Dockerfile.combined`
- [ ] Platforms configured: `linux/amd64,linux/arm64`
- [ ] Build arguments passed: VERSION, COMMIT_SHA, BUILD_DATE
- [ ] Tags from metadata action applied
- [ ] Labels from metadata action applied
- [ ] Cache configured: `cache-from: type=gha` and `cache-to: type=gha,mode=max`
- [ ] Push conditional: true for tag triggers, respects workflow_dispatch input

## Technical Requirements
- Action: `docker/build-push-action@v5`
- Step name: "Build and push Docker image"
- Inputs:
  - context: `${{ env.BUILD_CONTEXT }}` (workspace root: `.`)
  - file: `${{ env.DOCKERFILE_PATH }}` (packages/maproom-mcp/config/Dockerfile.combined)
  - platforms: `linux/amd64,linux/arm64`
  - push: `${{ github.event_name != 'workflow_dispatch' || github.event.inputs.push_to_registry == 'true' }}`
  - tags: `${{ steps.meta.outputs.tags }}`
  - labels: `${{ steps.meta.outputs.labels }}`
  - cache-from: `type=gha`
  - cache-to: `type=gha,mode=max`
  - build-args:
    - `VERSION=${{ steps.version.outputs.full }}`
    - `COMMIT_SHA=${{ github.sha }}`
    - `BUILD_DATE=${{ github.event.head_commit.timestamp }}`

## Implementation Notes
**Conditional Push Logic**:
- Tag triggers (`v*.*.*`): Always push to Docker Hub
- Manual workflow_dispatch: Only push if `push_to_registry` input is "true"
- This allows testing builds without publishing

**Build Arguments**:
These are passed to the Dockerfile and used in LABEL directives:
- VERSION: Semantic version (1.1.10)
- COMMIT_SHA: Full git commit hash for traceability
- BUILD_DATE: ISO 8601 timestamp of commit

**Caching Strategy**:
- `cache-from: type=gha`: Reuse layers from previous builds
- `cache-to: type=gha,mode=max`: Cache all layers including intermediate stages
- Reduces build time from 15min (cold) to 5min (warm)

**Combined Dockerfile Build**:
Dockerfile.combined builds both Rust and Node.js components in multi-stage build:
- Stage 1: Rust binary compilation (cargo build)
- Stage 2: TypeScript compilation (npx tsc)
- Stage 3: Runtime image with both components
- Build time: ~12-15 min (cold), ~5 min (warm with cache)

**Performance Expectations**:
- First build: ~12-15 minutes total
- Subsequent builds: ~5 minutes with cache
- Image size: ~350-400MB (includes both Rust and Node.js runtimes)

Reference DKRHUB_QUALITY_STRATEGY.md lines 42-76 for build validation test cases.

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist and be tested
- **DKRHUB-1007**: Local Dockerfile testing must pass before GitHub Actions build
- **DKRHUB-1004**: Version extraction and metadata must be configured
- **DKRHUB-1003**: Docker Hub authentication must be configured
- **DKRHUB-1002**: Buildx and QEMU must be set up

## Risk Assessment
- **Risk**: Multi-platform build failures (especially ARM64)
  - **Mitigation**: QEMU is stable; test with workflow_dispatch before release
- **Risk**: Build timeout (GitHub Actions 6-hour limit)
  - **Mitigation**: Caching keeps builds under 15 minutes; well within limits
- **Risk**: Cache eviction causing slow builds
  - **Mitigation**: GitHub retains caches for 7 days; regular releases keep cache warm

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml` (add build-push step)
