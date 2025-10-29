# Ticket: DKRHUB-1000: Create Combined Dockerfile

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create Dockerfile.combined that builds BOTH the Rust indexing binary (crewchief-maproom) AND the Node.js MCP server (index.ts) in a single multi-stage build. This is the critical prerequisite that fixes the broken Dockerfile architecture.

## Background
**Critical Blocker Discovered**: The MCP server architecture requires both components working together:
1. **Node.js MCP server** (`src/index.ts`) - Handles stdio JSON-RPC protocol communication with Claude/Cursor
2. **Rust indexing binary** (`crewchief-maproom`) - Performs semantic code scanning and search operations (spawned by Node.js)

**Current Problem**: The existing Dockerfiles are incomplete:
- `Dockerfile.maproom`: Only builds Rust binary (❌ missing Node.js runtime, TypeScript dist/, npm dependencies)
- `Dockerfile.mcp-server`: Only builds Node.js server (❌ missing Rust binary that index.ts spawns)

**Evidence**: The CLI (bin/cli.cjs:957) executes:
```bash
docker exec -i maproom-mcp node /app/dist/index.js
```

This command requires the container to have:
- ✅ Node.js runtime
- ✅ Compiled TypeScript in /app/dist/
- ✅ npm dependencies (pg, pino, zod, execa)
- ✅ Rust binary at /usr/local/bin/crewchief-maproom

**Impact**: This ticket BLOCKS all other DKRHUB tickets. Without a correct Dockerfile, the GitHub Actions workflow will build non-functional images.

Reference: DKRHUB_TICKETS_REVIEW_REPORT.md "Critical Issue #1"

## Acceptance Criteria
- [ ] File created: `packages/maproom-mcp/config/Dockerfile.combined`
- [ ] Stage 1: Rust Builder - Builds crewchief-maproom binary from workspace root
- [ ] Stage 2: Node.js Builder - Compiles TypeScript MCP server to dist/
- [ ] Stage 3: Runtime image contains BOTH components in single container
- [ ] Runtime base: node:20-alpine (for Node.js runtime + minimal size)
- [ ] Rust binary installed at: `/usr/local/bin/crewchief-maproom`
- [ ] Node.js app installed at: `/app/dist/index.js`
- [ ] Runtime dependencies installed: libgcc, libssl3 (for Rust), postgresql-client (for healthcheck)
- [ ] Image size < 400MB (target, compared to 300MB Rust-only)
- [ ] Healthcheck configured: `pg_isready -h maproom-postgres -U maproom`
- [ ] Entrypoint: `["node", "/app/dist/index.js"]`
- [ ] Non-root user: Uses node user (uid 1000)
- [ ] Security: No unnecessary build tools in runtime image

## Technical Requirements

**File**: `packages/maproom-mcp/config/Dockerfile.combined`

**Multi-Stage Build Structure**:

```dockerfile
# ========================================
# Stage 1: Build Rust Binary
# ========================================
FROM rust:1.82-slim AS rust-builder

# Install Rust build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy maproom crate (includes benches/ for manifest validation)
COPY crates/maproom/ ./crates/maproom/

# Fetch dependencies (cached layer if dependencies unchanged)
RUN cargo fetch --manifest-path crates/maproom/Cargo.toml --locked

# Build with aggressive optimizations (LTO, opt-level=z, strip, panic=abort)
RUN cargo build --release --bin crewchief-maproom --manifest-path crates/maproom/Cargo.toml

# Additional stripping for maximum size reduction
RUN strip --strip-all /build/target/release/crewchief-maproom 2>/dev/null || \
    strip /build/target/release/crewchief-maproom

# ========================================
# Stage 2: Build Node.js MCP Server
# ========================================
FROM node:20-alpine AS node-builder

# Install Node.js build dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++

WORKDIR /build

# Copy package files for dependency caching
COPY packages/maproom-mcp/package.json ./

# Install all dependencies (including devDependencies for TypeScript)
RUN npm install --production=false --no-audit --no-fund

# Copy TypeScript config and source code
COPY packages/maproom-mcp/tsconfig.json ./
COPY packages/maproom-mcp/src/ ./src/

# Compile TypeScript to JavaScript
RUN npx tsc

# ========================================
# Stage 3: Runtime Image (Combined)
# ========================================
FROM node:20-alpine

# Install runtime dependencies for both Rust and Node.js components
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libssl3 \
    postgresql-client

# Create necessary directories
RUN mkdir -p /app/dist /app/logs && \
    chown -R node:node /app

# Copy Rust binary from rust-builder stage
COPY --from=rust-builder --chown=node:node /build/target/release/crewchief-maproom /usr/local/bin/crewchief-maproom

# Set working directory for Node.js app
WORKDIR /app

# Copy package.json for production dependency installation
COPY --chown=node:node packages/maproom-mcp/package.json ./

# Install production dependencies only (no devDependencies)
RUN npm install --production --no-audit --no-fund --no-optional

# Copy compiled JavaScript from node-builder stage
COPY --from=node-builder --chown=node:node /build/dist ./dist

# Copy tools directory (if needed by index.js)
COPY --from=node-builder --chown=node:node /build/src/tools ./src/tools

# Switch to non-root user for security
USER node

# Health check: Verify database connectivity
# (MCP server is stdio-based, so we check database instead)
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD pg_isready -h maproom-postgres -U maproom || exit 1

# Set the entrypoint to Node.js running the MCP server
ENTRYPOINT ["node", "/app/dist/index.js"]

# No CMD needed - MCP server runs in stdio mode by default
```

**Build Context**: Must be repository root (`.`) to access both:
- Rust workspace: `Cargo.toml`, `crates/maproom/`
- Node.js package: `packages/maproom-mcp/`

**Validation Commands**:
```bash
# Build from repository root
cd /workspace
docker build -f packages/maproom-mcp/config/Dockerfile.combined -t maproom-test:local .

# Verify both components exist
docker run --rm maproom-test:local /bin/sh -c "which node && which crewchief-maproom"

# Check image size
docker images maproom-test:local

# Test MCP server starts (should wait for stdin)
timeout 5 docker run --rm -i maproom-test:local || echo "MCP server started successfully"
```

## Implementation Notes

**Why This Architecture**:
1. **Rust Builder Stage**: Compiles the indexing binary with full optimization (same as Dockerfile.maproom)
2. **Node.js Builder Stage**: Compiles TypeScript MCP server (same as Dockerfile.mcp-server)
3. **Combined Runtime**: Single image with both components, Node.js base provides runtime for MCP server, Rust binary available for spawning

**Size Optimization**:
- Multi-stage build discards build tools (~1GB of compiler toolchains)
- Alpine base keeps runtime small
- Only production npm dependencies installed
- Rust binary is stripped
- Target: ~350-400MB (vs 300MB Rust-only, acceptable tradeoff)

**Security Best Practices**:
- Non-root user (node, uid 1000)
- No build tools in runtime image
- Minimal attack surface with alpine base
- Only necessary runtime dependencies

**Copy Strategy**:
- `COPY --from=rust-builder`: Gets optimized Rust binary
- `COPY --from=node-builder`: Gets compiled JavaScript (not source TypeScript)
- Workspace structure not copied to runtime (only final artifacts)

## Dependencies
- **BLOCKS**: ALL other DKRHUB tickets (1001-4005)
- Repository workspace structure must remain consistent
- Cargo.toml, crates/maproom/ for Rust build
- packages/maproom-mcp/ for Node.js build

## Risk Assessment
- **Risk**: Build time increases with two language compilers
  - **Mitigation**: Multi-stage caching minimizes rebuilds, GitHub Actions cache (type=gha) reuses layers
- **Risk**: Image size larger than Rust-only version
  - **Mitigation**: Still under 400MB target, acceptable for functionality gained
- **Risk**: Maintaining two build stages
  - **Mitigation**: Each stage is straightforward, matches existing Dockerfiles

## Testing Requirements
Before marking complete, validate:
1. ✅ Docker build succeeds from workspace root
2. ✅ Both binaries exist in image (`node`, `crewchief-maproom`)
3. ✅ MCP server process starts (node /app/dist/index.js)
4. ✅ npm dependencies installed correctly (pg, pino, zod, execa)
5. ✅ Image size reasonable (< 400MB)
6. ✅ Healthcheck works (pg_isready command available)
7. ✅ Non-root user properly configured

## Files/Packages Affected
- NEW: `packages/maproom-mcp/config/Dockerfile.combined`

## Estimated Effort
4-6 hours (includes creation, testing, and validation)

## Related Issues
- Fixes: DKRHUB_TICKETS_REVIEW_REPORT.md "Critical Issue #1"
- Unblocks: DKRHUB-1001 through DKRHUB-4005 (all other tickets)
