# Ticket: DOCKER-1001: Add Perl to Dockerfile for vendored OpenSSL build

## Status
- [x] **Task completed** - added perl to rust-builder stage dependencies
- [x] **Tests pass** - Dockerfile change only, no code tests
- [ ] **Verified** - awaiting Docker image build workflow run
- [ ] **Committed** - ready to commit

## Agents
- docker-engineer
- github-actions-engineer

## Summary
Add Perl to the Dockerfile rust-builder stage to support vendored OpenSSL builds. The vendored OpenSSL feature (added in BINPKG-1903) builds OpenSSL from source, which requires Perl with core modules to run the Configure script.

## Background
Discovered in Docker image build workflow run #19055680179 when attempting to publish v1.3.1. The build failed with:

```
Can't locate FindBin.pm in @INC (you may need to install the FindBin module)
BEGIN failed--compilation aborted at ./Configure line 15.
Error configuring OpenSSL build: 'perl' reported failure with exit status: 2
```

**Root Cause**:
- BINPKG-1903 added `openssl = { version = "0.10", features = ["vendored"] }` to support cross-compilation
- Vendored feature builds OpenSSL from source instead of linking to system OpenSSL
- OpenSSL's Configure script requires Perl with core modules (FindBin.pm)
- The rustlang/rust:nightly-bookworm-slim base image has minimal Perl installation without all core modules

**Why Vendored Feature Was Added**:
BINPKG-1903 added the vendored feature specifically for GitHub Actions cross-compilation where the `cross` Docker images don't include libssl-dev packages. The feature statically links OpenSSL, making binaries self-contained.

**Why It Breaks Docker Builds**:
The Dockerfile.combined already installs libssl-dev (system OpenSSL), but the vendored feature in Cargo.toml forces building from source, which requires Perl dependencies not present in the slim image.

## Acceptance Criteria
- [x] Perl added to rust-builder stage apt-get install command
- [x] Comment added explaining why Perl is needed
- [ ] Docker image builds successfully
- [ ] Image published to Docker Hub

## Technical Requirements

### Current Problematic Dockerfile (lines 7-11)
```dockerfile
# Install Rust build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
```

### Fixed Dockerfile
```dockerfile
# Install Rust build dependencies
# perl is required for vendored OpenSSL build (BINPKG-1903 cross-compilation support)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    perl \
    && rm -rf /var/lib/apt/lists/*
```

## Implementation Notes

### Why Install Perl Instead of Removing Vendored Feature

**Option 1: Remove vendored feature from Cargo.toml**
- Pro: Faster Docker builds (use system OpenSSL)
- Con: Breaks npm package build workflow (BINPKG-1903)
- Con: Would need conditional features or separate builds

**Option 2: Add Perl to Dockerfile (selected)**
- Pro: Works with both workflows (Docker + npm package)
- Pro: Simple one-line fix
- Pro: Maintains consistency across builds
- Con: Slightly larger build context (~5MB for perl package)
- Con: Slightly longer build time (minimal impact)

The vendored feature is beneficial for both workflows:
- **npm package build**: Required for cross-compilation (no system OpenSSL in cross images)
- **Docker build**: Creates self-contained image (no runtime OpenSSL dependency, though we install libssl3 anyway)

### Package Size Impact
Adding the `perl` package to rust-builder stage:
- Package size: ~5-6MB
- Build time impact: ~2-3 seconds during apt-get install
- Runtime impact: None (perl only in build stage, not in final image)

## Dependencies
- BINPKG-1903 (vendored OpenSSL) - COMPLETED
- Related to build-and-publish-maproom-mcp.yml workflow

## Blocks
- Docker image publishing for v1.3.1
- Future Docker image releases

## Risk Assessment
- **Risk**: Very low - standard dependency installation
- **Impact**: Unblocks Docker image publishing

## Files to Modify
- `packages/maproom-mcp/config/Dockerfile.combined` (added perl to line 12)

## Verification
After implementing:
1. Trigger Docker image build workflow
2. Verify rust-builder stage completes successfully
3. Verify OpenSSL builds from source without Perl errors
4. Verify final image is published to Docker Hub
5. Test pulling and running the published image

## Priority
**CRITICAL** - Blocks Docker image publishing for v1.3.1 and all future releases

## Related Tickets
- BINPKG-1903: Enabled vendored OpenSSL for cross-compilation (root cause)
