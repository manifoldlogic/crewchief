# Ticket: DOCKER-1001: Add Perl and Make to Dockerfile for vendored OpenSSL build

## Status
- [x] **Task completed** - added perl and make to rust-builder stage dependencies
- [x] **Tests pass** - Dockerfile change only, no code tests
- [x] **Verified** - all acceptance criteria met, workflow run 19056137907 succeeded
- [x] **Committed** - commit 8090d39 (perl), 7184cce (make)
- [x] **Pushed to GitHub** - commits on origin/main, workflow validated successfully

## Agents
- docker-engineer
- github-actions-engineer

## Summary
Add Perl and Make to the Dockerfile rust-builder stage to support vendored OpenSSL builds. The vendored OpenSSL feature (added in BINPKG-1903) builds OpenSSL from source, which requires:
1. Perl with core modules to run the Configure script
2. Make to compile the OpenSSL source code

## Background
Discovered in two sequential Docker image build workflow runs when attempting to publish v1.3.1.

**First Error** (workflow run #19055680179):
```
Can't locate FindBin.pm in @INC (you may need to install the FindBin module)
BEGIN failed--compilation aborted at ./Configure line 15.
Error configuring OpenSSL build: 'perl' reported failure with exit status: 2
```

**Second Error** (workflow run #19055937062, after adding perl):
```
building OpenSSL dependencies: Command 'make' not found. Is make installed?
Error building OpenSSL dependencies:
    Command 'make' not found. Is make installed?
```

**Root Cause**:
- BINPKG-1903 added `openssl = { version = "0.10", features = ["vendored"] }` to support cross-compilation
- Vendored feature builds OpenSSL from source instead of linking to system OpenSSL
- OpenSSL build process requires:
  1. Perl with core modules (FindBin.pm) to run Configure script
  2. Make to compile the source code
- The rustlang/rust:nightly-bookworm-slim base image lacks both complete Perl and make

**Why Vendored Feature Was Added**:
BINPKG-1903 added the vendored feature specifically for GitHub Actions cross-compilation where the `cross` Docker images don't include libssl-dev packages. The feature statically links OpenSSL, making binaries self-contained.

**Why It Breaks Docker Builds**:
The Dockerfile.combined already installs libssl-dev (system OpenSSL), but the vendored feature in Cargo.toml forces building from source, which requires Perl dependencies not present in the slim image.

## Acceptance Criteria
- [x] Perl added to rust-builder stage apt-get install command
- [x] Make added to rust-builder stage apt-get install command
- [x] Comment updated explaining why Perl and Make are needed
- [x] Docker image builds successfully
- [x] Image published to Docker Hub

## Technical Requirements

### Current Problematic Dockerfile (original lines 7-11)
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
# perl and make are required for vendored OpenSSL build (BINPKG-1903 cross-compilation support)
# The vendored feature compiles OpenSSL from source, requiring full build toolchain
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*
```

## Implementation Notes

### Why Install Perl and Make Instead of Removing Vendored Feature

**Option 1: Remove vendored feature from Cargo.toml**
- Pro: Faster Docker builds (use system OpenSSL)
- Con: Breaks npm package build workflow (BINPKG-1903)
- Con: Would need conditional features or separate builds

**Option 2: Add Perl and Make to Dockerfile (selected)**
- Pro: Works with both workflows (Docker + npm package)
- Pro: Simple fix (two additional packages)
- Pro: Maintains consistency across builds
- Con: Slightly larger build context (~6-7MB for perl + make packages)
- Con: Slightly longer build time (~3-5 seconds additional)

The vendored feature is beneficial for both workflows:
- **npm package build**: Required for cross-compilation (no system OpenSSL in cross images)
- **Docker build**: Creates self-contained image (no runtime OpenSSL dependency, though we install libssl3 anyway)

### Package Size Impact
Adding `perl` and `make` packages to rust-builder stage:
- Combined package size: ~6-7MB (perl ~5MB, make ~1-2MB)
- Build time impact: ~3-5 seconds during apt-get install
- Runtime impact: None (build tools only in build stage, not in final image)

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
- `packages/maproom-mcp/config/Dockerfile.combined` (added perl and make to lines 13-14)

## Verification
After implementing:
1. Trigger Docker image build workflow
2. Verify rust-builder stage completes successfully
3. Verify OpenSSL builds from source without Perl errors
4. Verify final image is published to Docker Hub
5. Test pulling and running the published image

### Verification Results (2025-11-04)
All acceptance criteria verified and met:

1. Perl added to rust-builder stage - VERIFIED
   - Evidence: Line 13 of Dockerfile.combined contains `perl \`
   - Commit: 8090d39

2. Make added to rust-builder stage - VERIFIED
   - Evidence: Line 14 of Dockerfile.combined contains `make \`
   - Commit: 7184cce

3. Comment updated - VERIFIED
   - Evidence: Lines 8-9 contain detailed explanation:
     "perl and make are required for vendored OpenSSL build (BINPKG-1903 cross-compilation support)"
     "The vendored feature compiles OpenSSL from source, requiring full build toolchain"

4. Docker image builds successfully - VERIFIED
   - Evidence: GitHub Actions workflow run 19056137907 completed successfully
   - Workflow conclusion: success
   - Head SHA: cf41c4d25571316ca7f86b8a33c6e643ddcd1aa2

5. Image published to Docker Hub - VERIFIED
   - Evidence: Workflow run 19056137907 completed all steps including publish
   - Docker image successfully pushed to registry

Code quality checks:
- No incomplete implementations
- No TODO comments in code
- Commits follow Conventional Commits format
- Both commits properly reference DOCKER-1001
- Documentation is thorough and accurate

Status: READY FOR COMMIT (already committed in 8090d39 and 7184cce)

## Priority
**CRITICAL** - Blocks Docker image publishing for v1.3.1 and all future releases

## Related Tickets
- BINPKG-1903: Enabled vendored OpenSSL for cross-compilation (root cause)
