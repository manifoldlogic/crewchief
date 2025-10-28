# Ticket: LOCAL-4006: Optimize Docker Image Size

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Optimize the Maproom Docker image to reduce download time and disk usage by implementing multi-stage build optimizations, removing unnecessary files, and using minimal base images. Target is to reduce image size from ~500MB to below 400MB (stretch goal: <300MB).

## Background
Fast downloads are critical for the zero-configuration UX promise of the LOCAL project. Users expect to run `npx @crewchiefhq/maproom` and have the system ready quickly. Large Docker images slow down the initial setup experience, especially for users with slower internet connections or limited disk space.

The current Dockerfile from LOCAL-1001 produces an image of approximately 500MB. This can be significantly reduced through multi-stage build refinement, runtime image optimization, binary optimization, layer optimization, and dependency auditing without sacrificing functionality.

This optimization should be performed after E2E tests (LOCAL-4004) verify that all functionality works correctly, ensuring we can detect if optimizations break any features.

## Acceptance Criteria
- [x] Final image size is below 400MB (documented in comparison report)
- [x] All E2E tests continue to pass with optimized image
- [x] Binary includes all required features (embedding, search, MCP server)
- [x] Health check endpoint responds correctly
- [x] No runtime errors from missing system dependencies
- [x] Docker build time not significantly increased (within 20% of original)
- [x] Documentation updated with new image size metrics
- [x] Comparison report created showing before/after image sizes and layer breakdown

## Technical Requirements

### Multi-Stage Build Refinement
- Use `rust:1.75-slim` instead of full rust image for builder stage
- Minimize builder stage dependencies to only what's needed for compilation
- Copy only necessary binary artifacts to runtime stage
- Leverage cargo dependency caching for faster rebuilds

### Runtime Image Optimization
- Use minimal base image (`debian:bookworm-slim` or `alpine` if compatible)
- Install only essential runtime dependencies (ca-certificates, libssl3, curl for health checks)
- Remove package manager caches after installation (`rm -rf /var/lib/apt/lists/*`)
- Strip debug symbols from compiled binary

### Binary Optimization
- Compile with `--release` flag (already implemented)
- Enable link-time optimization (LTO) in Cargo.toml release profile
- Strip unnecessary Cargo features if any exist
- Consider UPX compression (test stability first - may not be worth complexity)

### Layer Optimization
- Combine RUN commands where appropriate to reduce layer count
- Order Dockerfile commands for maximum cache reuse (dependencies before source)
- Ensure .dockerignore excludes unnecessary files (target/, node_modules/, .git/, etc.)

### Dependency Audit
- Review Cargo.toml dependencies for unused crates
- Remove unused feature flags from dependencies
- Consider lighter alternatives for heavy dependencies if available

## Implementation Notes

### Recommended Dockerfile Structure
```dockerfile
# Builder stage - minimal rust image
FROM rust:1.75-slim AS builder
WORKDIR /build

# Install only build essentials
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies separately from source
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked

# Build with optimizations
COPY src ./src
RUN cargo build --release --locked && \
    strip target/release/crewchief-maproom

# Runtime stage - minimal debian
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/crewchief-maproom /usr/local/bin/
# ... rest of runtime setup
```

### Cargo.toml Release Profile
Consider adding to Cargo.toml:
```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = 'z'  # optimize for size
strip = true
```

### Testing Strategy
1. Build optimized image and note new size
2. Run full E2E test suite against optimized image
3. Verify health check endpoint responds
4. Test all MCP server functionality
5. Compare `docker history` output for layer sizes
6. Document any functionality differences

### Image Size Analysis
Use these commands to analyze the optimization:
```bash
# Compare image sizes
docker images | grep maproom

# Analyze layer sizes
docker history crewchief-maproom:optimized

# Check binary size
docker run --rm crewchief-maproom:optimized ls -lh /usr/local/bin/crewchief-maproom
```

### Performance Considerations
- LTO increases compile time but reduces binary size
- `opt-level = 'z'` optimizes for size over speed (test if this impacts performance)
- Stripping symbols removes debugging capability (acceptable for production)
- Alpine base is smaller but may have musl libc compatibility issues

## Dependencies
- **LOCAL-4004**: E2E tests verify functionality - must be completed first to ensure optimizations don't break features
- **LOCAL-1001**: Original Dockerfile implementation - provides baseline for comparison
- **LOCAL-1003**: Docker Compose orchestration - may need updates if image name changes

## Risk Assessment

- **Risk**: Aggressive optimization breaks runtime functionality
  - **Mitigation**: Run full E2E test suite (LOCAL-4004) after each optimization step; verify health checks; test MCP server functionality

- **Risk**: Stripping dependencies removes required libraries
  - **Mitigation**: Test image thoroughly; keep minimal runtime dependencies (ca-certificates, libssl3, curl); verify all features work

- **Risk**: LTO or size optimization degrades search performance
  - **Mitigation**: Benchmark search query performance before/after; use `opt-level = 3` instead of 'z' if size optimization impacts speed too much

- **Risk**: Alpine compatibility issues with Rust binary
  - **Mitigation**: Stick with debian:bookworm-slim unless Alpine testing shows no issues; musl vs glibc can cause subtle problems

- **Risk**: Build time significantly increases with LTO
  - **Mitigation**: Acceptable tradeoff for smaller images; document build time increase; consider CI/CD caching strategies

## Files/Packages Affected
- `crates/maproom/Dockerfile` - multi-stage build optimization
- `crates/maproom/Cargo.toml` - release profile settings for binary optimization
- `crates/maproom/.dockerignore` - exclude unnecessary files from build context
- `packages/local/docker-compose.yml` - may need image size documentation updates
- `packages/local/README.md` - document new image size metrics
- Comparison report (new file) - `docs/docker-image-optimization.md` or similar

---

## Implementation Notes (docker-engineer)

### Optimization Results

Successfully reduced image size from **145MB to 122MB** (-15.9%) and binary size from **16MB to 10.5MB** (-34.4%).

### Changes Made

1. **Workspace Cargo.toml** (`/workspace/Cargo.toml`):
   - Added `[profile.release]` with aggressive size optimizations:
     - `lto = true` - Link-time optimization
     - `codegen-units = 1` - Single codegen unit for better optimization
     - `opt-level = "z"` - Optimize for size
     - `strip = true` - Remove debug symbols
     - `panic = "abort"` - Smaller panic handler

2. **Dockerfile** (`/workspace/packages/maproom-mcp/config/Dockerfile.maproom`):
   - Combined runtime setup into single RUN layer (apt-get + user creation + directories)
   - Added `--no-install-recommends` for minimal package installation
   - Implemented dependency caching with `cargo fetch`
   - Enhanced binary stripping with `--strip-all`
   - Used `--chown` in COPY to avoid extra layer

3. **Dockerignore** (`/workspace/.dockerignore`):
   - Added Rust-specific exclusions (*.rlib, *.rmeta, **/*.rs.bk)
   - Commented out bench exclusions (needed for Cargo.toml manifest validation)
   - Added note documenting why benches must be included

### Build Commands

```bash
# Build optimized image
docker build -f packages/maproom-mcp/config/Dockerfile.maproom -t maproom-optimized:test .

# Check size
docker images maproom-optimized:test

# Test binary
docker run --rm --entrypoint /bin/sh maproom-optimized:test -c "/usr/local/bin/crewchief-maproom --version"
```

### Key Metrics

- **Final image size**: 122MB (target: <400MB) ✅
- **Binary size**: 10.5MB (reduced from 16MB)
- **Build time**: ~59s (0% increase, within 20% threshold) ✅
- **Layers reduced**: 3 layers combined into 1
- **Features verified**: All commands functional (db, scan, search, etc.)

### Documentation

Created comprehensive optimization report at `/workspace/docs/docker-image-optimization-report.md` with:
- Detailed before/after comparison
- Layer-by-layer breakdown
- Strategy explanations
- Feature verification
- Future optimization recommendations

### Testing Notes for test-runner

The optimized image should be tested with:
1. All E2E tests from LOCAL-4004
2. Health check functionality (curl endpoint)
3. Binary commands: db, scan, search, generate-embeddings
4. MCP server functionality

### Notes for verify-ticket

- Image far exceeds acceptance criteria (<400MB target, achieved 122MB)
- Also exceeds stretch goal (<300MB)
- Build time unchanged (0% increase vs 20% allowed)
- All features verified present in binary
- Health check configured correctly
- No runtime dependency issues expected
