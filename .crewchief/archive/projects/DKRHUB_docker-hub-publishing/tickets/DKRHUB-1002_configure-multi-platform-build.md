# Ticket: DKRHUB-1002: Configure Multi-Platform Build

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Set up Docker Buildx with QEMU emulation to enable building images for both AMD64 and ARM64 platforms in the GitHub Actions workflow.

## Background
Modern deployments require multi-platform support: AMD64 (Linux servers, Intel Macs) and ARM64 (Apple Silicon Macs, AWS Graviton). Docker Buildx with QEMU allows GitHub Actions to build images for both platforms from a single AMD64 runner.

This ticket adds the necessary build infrastructure steps to the workflow created in DKRHUB-1001.

Reference: DKRHUB_PLAN.md Phase 1, Task DKRHUB-1002 (lines 110-142)

## Acceptance Criteria
- [x] QEMU setup step added using `docker/setup-qemu-action@v3` with platforms `linux/amd64,linux/arm64`
- [x] Docker Buildx setup step added using `docker/setup-buildx-action@v3`
- [x] GitHub Actions cache will be configured in build step (DKRHUB-1005)
- [x] Workflow includes checkout step with `fetch-depth: 0` for full git history
- [x] All steps positioned correctly in workflow (after checkout, before build)

## Technical Requirements
- Actions versions:
  - `actions/checkout@v4` with `fetch-depth: 0`
  - `docker/setup-qemu-action@v3` with platforms: `linux/amd64,linux/arm64`
  - `docker/setup-buildx-action@v3` with driver-opts for buildkit
- Step names must match:
  - "Checkout code"
  - "Set up QEMU"
  - "Set up Docker Buildx"
- QEMU platforms: `linux/amd64,linux/arm64` (exactly these two)
- Buildx driver-opts:
  - `image=moby/buildkit:latest`
  - `network=host`

## Implementation Notes
QEMU (Quick Emulator) enables cross-platform builds by emulating ARM64 architecture on AMD64 runners. This is slower than native builds (12-15 min for ARM64 vs 8-10 min for AMD64) but eliminates the need for separate ARM64 runners.

GitHub Actions cache (type=gha) dramatically improves build times on subsequent runs:
- First build (cold): ~15 minutes total
- Subsequent builds (warm): ~5 minutes total

The cache-to mode=max ensures maximum layer caching including intermediate build stages.

**Multi-Stage Caching with Dockerfile.combined**:
GitHub Actions cache works with combined Dockerfile's Rust and Node.js build stages. Each stage (Rust builder, Node.js builder, runtime) benefits from layer caching, significantly reducing rebuild times when dependencies haven't changed.

Reference DKRHUB_ARCHITECTURE.md lines 130-150 and DKRHUB_QUALITY_STRATEGY.md lines 692-705 for performance expectations.

## Dependencies
- **DKRHUB-1000**: Dockerfile.combined must exist
- **DKRHUB-1001**: Workflow file must exist before adding these steps

## Risk Assessment
- **Risk**: QEMU emulation could be slow or unstable
  - **Mitigation**: This is standard practice in GitHub Actions, well-tested and reliable
- **Risk**: Cache could grow too large
  - **Mitigation**: GitHub automatically prunes old caches, 10GB limit per repository

## Files/Packages Affected
- `.github/workflows/publish-maproom-mcp-image.yml` (add steps to existing file)
