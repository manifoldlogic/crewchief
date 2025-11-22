# Architecture: CI Workflow Fixes

## Solution Overview

Two independent, minimal-change fixes to restore CI functionality:

1. **Test Workflow**: Remove explicit pnpm version from GitHub Actions workflow
2. **Docker Build**: Add pnpm to Docker builder and use workspace-aware dependency installation

Both solutions follow **principle of least change** - touch only what's broken, preserve what works.

## Solution 1: Test Workflow Fix

### Current Architecture

```yaml
# .github/workflows/test.yml
steps:
  - uses: pnpm/action-setup@v4
    with:
      version: 10  # ← PROBLEM: Conflicts with package.json
```

```json
// package.json (root)
{
  "packageManager": "pnpm@10.12.1+sha512..."  // ← Source of truth
}
```

**Conflict**: Two sources of version specification

### Proposed Architecture

```yaml
# .github/workflows/test.yml
steps:
  - uses: pnpm/action-setup@v4
    # No version field - auto-detects from packageManager
```

```json
// package.json (root)
{
  "packageManager": "pnpm@10.12.1+sha512..."  // ← Single source of truth
}
```

**Change**: Remove `with:` block entirely

### How It Works

1. **Action startup**: pnpm/action-setup@v4 runs
2. **Version detection**: Action reads `package.json` from repo root
3. **Parse packageManager**: Extracts `pnpm@10.12.1`
4. **Install pnpm**: Downloads and caches specified version
5. **Verification**: `pnpm --version` returns `10.12.1`
6. **Continue**: Workflow proceeds with `pnpm install`

### Rationale

**Why remove explicit version?**
- packageManager field is npm standard (RFC, adopted 2022)
- pnpm/action-setup@v4 designed for auto-detection
- Eliminates manual sync burden
- Future pnpm updates: change one file, not two

**Why not update explicit version to match?**
- Still requires manual sync on every pnpm upgrade
- Defeats purpose of packageManager field
- Official docs recommend omitting version with packageManager

**Alternatives considered**:
- ❌ Remove packageManager field: Breaks local dev (no automatic pnpm version)
- ❌ Keep both, bump to v5: Still requires manual sync
- ✅ Remove explicit version: Standard practice, zero maintenance

## Solution 2: Docker Build Fix

### Current Architecture

```dockerfile
# Stage 2: Node.js builder (BROKEN)
FROM node:20-alpine

WORKDIR /build
COPY packages/maproom-mcp/package.json ./
RUN npm install  # ← FAILS: doesn't understand workspace:

# Copy and build
COPY packages/maproom-mcp/src ./src
RUN npx tsc
```

**Problem**: Single-package context, workspace dependency unresolved

### Proposed Architecture

```dockerfile
# Stage 2: Node.js builder (FIXED)
FROM node:20-alpine

# Install pnpm matching packageManager version
RUN npm install -g pnpm@10.12.1

WORKDIR /build

# Copy workspace configuration
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy package manifests (for layer caching)
COPY packages/maproom-mcp/package.json ./packages/maproom-mcp/
COPY packages/daemon-client/package.json ./packages/daemon-client/

# Copy daemon-client build artifacts (zero runtime deps)
COPY packages/daemon-client/dist ./packages/daemon-client/dist/
COPY packages/daemon-client/tsconfig.json ./packages/daemon-client/

# Install dependencies with workspace resolution
RUN pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...

# Copy source and build
COPY packages/maproom-mcp/tsconfig.json ./packages/maproom-mcp/
COPY packages/maproom-mcp/src ./packages/maproom-mcp/src/
WORKDIR /build/packages/maproom-mcp
RUN pnpm build
```

---

### 📋 Precise Dockerfile Implementation

**File**: `packages/maproom-mcp/config/Dockerfile.combined`
**Stage**: 2 (Node.js builder)
**Lines to Replace**: 38-59

**Current Implementation (BROKEN)**:
```dockerfile
38: FROM node:20-alpine AS node-builder
39:
40: # Install Node.js build dependencies
41: RUN apk add --no-cache \
42:     python3 \
43:     make \
44:     g++
45:
46: WORKDIR /build
47:
48: # Copy package files for dependency caching
49: COPY packages/maproom-mcp/package.json ./
50:
51: # Install all dependencies (including devDependencies for TypeScript)
52: RUN npm install --production=false --no-audit --no-fund
53:
54: # Copy TypeScript config and source code
55: COPY packages/maproom-mcp/tsconfig.json ./
56: COPY packages/maproom-mcp/src/ ./src/
57:
58: # Compile TypeScript to JavaScript
59: RUN npx tsc
```

**New Implementation (FIXED)**:
```dockerfile
38: FROM node:20-alpine AS node-builder
39:
40: # Install pnpm matching packageManager version
41: RUN npm install -g pnpm@10.12.1
42:
43: # Install Node.js build dependencies
44: RUN apk add --no-cache \
45:     python3 \
46:     make \
47:     g++
48:
49: WORKDIR /build
50:
51: # Copy workspace configuration
52: COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
53:
54: # Copy package manifests (for dependency caching)
55: COPY packages/maproom-mcp/package.json ./packages/maproom-mcp/
56: COPY packages/daemon-client/package.json ./packages/daemon-client/
57:
58: # Copy daemon-client build artifacts (pre-built via pnpm build)
59: COPY packages/daemon-client/dist ./packages/daemon-client/dist/
60: COPY packages/daemon-client/tsconfig.json ./packages/daemon-client/
61:
62: # Install dependencies with workspace resolution
63: RUN pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...
64:
65: # Copy TypeScript config and source code
66: COPY packages/maproom-mcp/tsconfig.json ./packages/maproom-mcp/
67: COPY packages/maproom-mcp/src/ ./packages/maproom-mcp/src/
68:
69: # Change to package directory and build
70: WORKDIR /build/packages/maproom-mcp
71: RUN pnpm build
```

**Key Changes**:
1. **Lines 40-41**: Add pnpm installation before apk (Alpine packages need npm from base image)
2. **Lines 51-52**: Copy workspace root configs (package.json, pnpm-lock.yaml, pnpm-workspace.yaml)
3. **Lines 55-60**: Copy both package manifests plus daemon-client dist/
4. **Line 63**: Replace `npm install` with `pnpm install --frozen-lockfile --filter`
5. **Lines 65-67**: Copy source code (unchanged)
6. **Lines 70-71**: Change WORKDIR to packages/maproom-mcp/ and run `pnpm build` (not `npx tsc`)

**Validation After Edit**:
```bash
# Verify Stage 2 starts at correct line
sed -n '38p' packages/maproom-mcp/config/Dockerfile.combined
# Should show: FROM node:20-alpine AS node-builder

# Count lines in Stage 2 (should be ~33 lines now, was ~22)
sed -n '38,71p' packages/maproom-mcp/config/Dockerfile.combined | wc -l

# Verify pnpm install line exists
grep -n "pnpm install --frozen-lockfile" packages/maproom-mcp/config/Dockerfile.combined
```

---

### How It Works

1. **pnpm installation**: Global install in Alpine (~20MB, removed in final stage)
2. **Workspace setup**: Copy root configs (package.json, pnpm-lock.yaml, pnpm-workspace.yaml)
3. **Package manifests**: Copy both maproom-mcp and daemon-client package.json files
4. **Dependency resolution**: `pnpm install --filter @crewchief/maproom-mcp...`
   - Reads workspace config
   - Resolves `workspace:*` to daemon-client
   - Installs maproom-mcp + its workspace deps
   - Uses lockfile for reproducibility
5. **Build**: TypeScript compilation with workspace types available

### Key Design Decisions

#### Why pnpm over npm?

**Consistency**:
- Local dev: pnpm
- CI tests: pnpm
- Docker build: pnpm (new)
- Result: Single package manager everywhere

**Correctness**:
- npm doesn't support `workspace:` protocol
- pnpm invented it, handles it natively
- No package.json rewriting needed

**Future-proofing**:
- Works for any number of workspace dependencies
- No special handling per dependency
- Automatic resolution via `--filter` flag

#### Why --filter instead of full install?

**Efficiency**:
```bash
# Without --filter: Installs ALL workspace packages
pnpm install --frozen-lockfile
# Result: cli, vscode-maproom, maproom-mcp, daemon-client (unnecessary bloat)

# With --filter: Installs only maproom-mcp + its deps
pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...
# Result: maproom-mcp, daemon-client (precise, minimal)
```

**Cache optimization**:
- Only maproom-mcp and daemon-client layers
- Changes to cli or vscode-maproom don't invalidate cache
- Faster builds, smaller context

#### Why copy daemon-client dist/?

**Build artifacts vs source**:
```dockerfile
# Option A: Copy daemon-client source, build in Docker
COPY packages/daemon-client/src ./packages/daemon-client/src
RUN pnpm --filter @maproom/daemon-client build

# Option B: Copy pre-built dist (CHOSEN)
COPY packages/daemon-client/dist ./packages/daemon-client/dist
# No build needed - already compiled locally
```

**Rationale for Option B**:
- daemon-client built during `pnpm build` at repo root
- Re-building in Docker is redundant
- Faster builds (skip TypeScript compilation)
- Smaller build context (dist is 124KB vs src + node_modules)

**Trade-off acknowledged**:
- **CRITICAL**: Requires `pnpm build` before `docker build`
- daemon-client dist/ must exist before Docker build starts
- This dependency must be maintained in ALL build contexts

---

## ⚠️ CRITICAL PREREQUISITE: Release Workflow Must Run pnpm build

**BLOCKER**: The release workflow (`.github/workflows/publish-maproom-mcp-image.yml`) MUST run `pnpm build` before Docker build.

### Current State (BROKEN)
```yaml
# publish-maproom-mcp-image.yml
- name: Checkout code
  uses: actions/checkout@v4

- name: Build and push Docker image  # ← FAILS: daemon-client/dist doesn't exist
  uses: docker/build-push-action@v5
```

### Required Fix
Add these steps BEFORE Docker build:

```yaml
# Setup Node.js
- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: '20'

# Setup pnpm (auto-detects from packageManager field)
- name: Setup pnpm
  uses: pnpm/action-setup@v4

# Install dependencies
- name: Install dependencies
  run: pnpm install --frozen-lockfile

# Build all workspace packages (includes daemon-client)
- name: Build packages
  run: pnpm build

# Now Docker build will succeed
- name: Build and push Docker image
  uses: docker/build-push-action@v5
```

**Why This is Critical**:
1. Dockerfile.combined copies `packages/daemon-client/dist/` (line 116 in proposed changes)
2. If dist/ doesn't exist, Docker build fails immediately: `COPY failed: file not found`
3. `pnpm build` creates daemon-client dist/ by running TypeScript compilation
4. This is a HARD DEPENDENCY that cannot be worked around

**Validation**:
```bash
# Verify daemon-client dist/ exists after pnpm build
pnpm build
ls -la packages/daemon-client/dist/
# Must show: index.js, index.d.ts, client.js, etc.

# Docker build will fail if dist/ missing
if [ ! -d "packages/daemon-client/dist" ]; then
  echo "ERROR: daemon-client dist/ not found. Run 'pnpm build' first."
  exit 1
fi
```

---

### Layer Caching Strategy

```dockerfile
# Layer 1: Base image (Alpine + Node.js) - rarely changes
FROM node:20-alpine

# Layer 2: pnpm installation - changes only with pnpm version
RUN npm install -g pnpm@10.12.1

# Layer 3: Workspace config - changes with package manager updates
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Layer 4: Package manifests - changes when dependencies update
COPY packages/maproom-mcp/package.json ./packages/maproom-mcp/
COPY packages/daemon-client/package.json ./packages/daemon-client/
COPY packages/daemon-client/dist ./packages/daemon-client/dist/

# Layer 5: Dependency installation - invalidated by Layer 4
RUN pnpm install --frozen-lockfile --filter @crewchief/maproom-mcp...

# Layer 6: Source code - changes frequently
COPY packages/maproom-mcp/src ./packages/maproom-mcp/src/

# Layer 7: TypeScript build - invalidated by Layer 6
RUN pnpm build
```

**Cache hit scenarios**:

| Change | Invalidates Layers | Rebuild Time |
|--------|-------------------|--------------|
| Source code only | 6-7 | ~30s |
| Add dependency | 4-7 | ~2min |
| Update pnpm | 2-7 | ~3min |
| Base image | 1-7 | ~5min |

**Optimization**: Most changes (source code) only rebuild final 2 layers

### Multi-Stage Build Preservation

```dockerfile
# Stage 1: Rust builder (UNCHANGED)
FROM rustlang/rust:nightly-bookworm-slim AS rust-builder
# ... Rust build steps ...

# Stage 2: Node.js builder (MODIFIED)
FROM node:20-alpine AS node-builder
# ... New pnpm-based build ...

# Stage 3: Runtime (UNCHANGED)
FROM node:20-slim
COPY --from=rust-builder /build/target/release/crewchief-maproom /usr/local/bin/
COPY --from=node-builder /build/packages/maproom-mcp/dist ./dist
# ... Runtime setup ...
```

**Impact**: Only Stage 2 changes, Stages 1 and 3 untouched

### Image Size Analysis

```
Before:
- Stage 1 (Rust): ~1.2GB (build stage, discarded)
- Stage 2 (Node.js): ~450MB (build stage, discarded)
- Stage 3 (Runtime): ~220MB (final image)

After:
- Stage 1 (Rust): ~1.2GB (unchanged)
- Stage 2 (Node.js): ~500MB (+50MB pnpm, build stage, discarded)
- Stage 3 (Runtime): ~220MB (unchanged)
```

**Result**: Final image size unchanged (pnpm only in discarded builder stage)

## Technology Choices

### pnpm@10.12.1

**Why this version?**
- Matches local development (packageManager field)
- Stable release (not beta/RC)
- Known to work with current lockfile

**Why not pnpm@latest?**
- Risk of lockfile incompatibility
- Local dev would drift
- Unnecessary version skew

**Pinning strategy**: Mirror packageManager version exactly

### node:20-alpine (Base Image)

**Why Alpine?**
- Minimal size (~40MB vs ~200MB for Debian-slim)
- Standard for Node.js builders
- apk package manager for native deps (python3, make, g++)

**Why Node.js 20?**
- Matches project's engine requirement (`engines.node: ">=18.0.0"`)
- LTS until April 2026
- Native fetch, test runner, TypeScript support

**Why not upgrade to Node.js 22?**
- Not an LTS release yet (becomes LTS Oct 2024)
- Unnecessary risk for this fix
- Can upgrade separately

### --frozen-lockfile Flag

**Purpose**: Enforce lockfile fidelity

**Behavior**:
- Reads pnpm-lock.yaml
- Fails if package.json and lockfile diverge
- Prevents phantom dependency resolution
- Ensures reproducible builds

**Why not --prefer-frozen-lockfile?**
- Would auto-update lockfile in Docker build
- Builds would become non-reproducible
- Lockfile changes belong in source control, not CI

## Performance Considerations

### Build Time Impact

**Baseline** (current, broken build):
- ~3 minutes on GitHub Actions (amd64 + arm64)

**Projected** (with pnpm):
- First build: +30s (pnpm install overhead)
- Cached builds: +5s (pnpm binary download)
- Net impact: ~10% slower on average

**Mitigation**: Layer caching minimizes repeated work

### Network Efficiency

**pnpm's advantages**:
- Content-addressable storage (deduplicated downloads)
- Tarball caching in GitHub Actions
- `--frozen-lockfile` skips metadata fetches

**Comparison to npm**:
- pnpm: Downloads package once, links everywhere
- npm: May download same package multiple times
- Result: pnpm often faster despite extra tool

### Parallel Builds

**Multi-platform strategy** (unchanged):
```yaml
# GitHub Actions workflow
platforms: linux/amd64,linux/arm64
```

**Parallelization**:
- amd64 and arm64 build simultaneously
- Separate runners, no shared cache
- pnpm install runs twice (once per platform)

**Optimization opportunity** (future):
- Layer cache pushed to GitHub Container Registry
- Cross-platform cache sharing
- Not needed for MVP

## Security Considerations

### pnpm Installation Method

**Chosen approach**:
```dockerfile
RUN npm install -g pnpm@10.12.1
```

**Why npm install -g?**
- Official pnpm installation method
- Verifies package signature via npm registry
- Downloads from npmjs.com (trusted source)
- No curl | sh patterns (avoid arbitrary code execution)

**Alternatives considered**:
```dockerfile
# Option A: npm install (CHOSEN)
RUN npm install -g pnpm@10.12.1

# Option B: Official installer script
RUN wget -qO- https://get.pnpm.io/install.sh | sh
# ❌ Executes remote shell script, harder to audit

# Option C: Pre-compiled binary
RUN wget https://github.com/pnpm/pnpm/releases/download/v10.12.1/pnpm-linux-x64
# ❌ No signature verification, manual platform detection
```

**Security properties**:
- npm verifies package integrity (SHA-512)
- pnpm package signed by pnpm maintainers
- Consistent with local installation method
- Auditable via npm registry

### Dependency Pinning

**Lockfile immutability**:
```dockerfile
RUN pnpm install --frozen-lockfile
```

**Security guarantee**:
- Exact versions from lockfile
- No automatic upgrades during build
- Reproducible dependency tree
- Prevents supply chain attacks via version skew

**Audit trail**:
- Lockfile changes reviewed in PRs
- CI validates lockfile consistency
- Renovate bot manages updates (future)

### Multi-Stage Build Isolation

**Attack surface minimization**:
```dockerfile
# Builder stage: Has pnpm, build tools
FROM node:20-alpine AS node-builder
RUN npm install -g pnpm@10.12.1  # ← Build-time only

# Runtime stage: No pnpm, no build tools
FROM node:20-slim
# Only production artifacts copied, no builders
```

**Result**: pnpm never reaches production image

## Deployment Strategy

### Rollout Plan

**Phase 1: Test Workflow** (Zero Risk)
1. Merge PR with test.yml change
2. Observe CI on next push
3. Validate pnpm version detection
4. Confirm tests pass

**Phase 2: Docker Build** (Low Risk)
1. Test locally: `docker build -f packages/maproom-mcp/config/Dockerfile.combined .`
2. Merge PR with Dockerfile.combined changes
3. Create test tag: `v2.2.1-rc.1`
4. Trigger release workflow
5. Validate multi-platform build
6. Pull image and smoke test: `docker run maproom-mcp --help`
7. Promote to stable release

### Rollback Strategy

**Test workflow rollback**:
```bash
git revert <commit-sha>
git push
```
- Immediate revert if pnpm detection fails
- Fallback: manually specify version while debugging

**Docker build rollback**:
- Revert Dockerfile.combined to previous version
- Re-tag previous Docker image as latest
- Update GitHub release to point to rollback image

**Incident detection**:
- GitHub Actions failure notifications
- Docker Hub pull metrics
- User reports via GitHub Issues

### Verification Checklist

**Test workflow verification**:
- [ ] CI runs without pnpm version errors
- [ ] pnpm version matches packageManager field
- [ ] Tests execute and complete
- [ ] Workflow runtime within 10% of baseline

**Docker build verification**:
- [ ] Multi-platform build succeeds (amd64 + arm64)
- [ ] Image size within 5% of baseline (~220MB)
- [ ] Container starts without errors
- [ ] MCP server responds to stdio input
- [ ] Database connection works
- [ ] Rust binary invocation succeeds

## Maintainability

### Documentation Requirements

**Files to update**:
1. `.github/workflows/test.yml` - Comment explaining auto-detection
2. `packages/maproom-mcp/config/Dockerfile.combined` - Comment pnpm strategy
3. `.github/CLAUDE.md` - Update workflow debugging guidance
4. `packages/maproom-mcp/CLAUDE.md` - Document Docker build requirements

**Developer onboarding**:
- README: No changes needed (Docker build still `docker build`)
- Contributing guide: Note `pnpm build` prerequisite for Docker builds

### Future Extensibility

**Adding more workspace dependencies**:
```json
// packages/maproom-mcp/package.json
{
  "dependencies": {
    "@maproom/daemon-client": "workspace:*",
    "@maproom/common-types": "workspace:*"  // ← Just works
  }
}
```

**No Dockerfile changes needed** - `--filter` resolves transitively

**Upgrading pnpm**:
```json
// package.json (root)
{
  "packageManager": "pnpm@11.0.0+sha512..."
}
```

**Update locations**:
1. Root package.json (automatic via `pnpm self-update` or manual)
2. Dockerfile.combined (line 42: `RUN npm install -g pnpm@11.0.0`)

**Automation opportunity** (future): Renovate bot syncs both

### Monitoring and Alerting

**Metrics to track**:
- CI failure rate (should drop to zero for these specific errors)
- Docker build time (should stay within 10% of baseline)
- Docker image size (should remain stable)
- pnpm version drift (manual check: package.json vs Dockerfile)

**Alert conditions**:
- CI failure with "EUNSUPPORTEDPROTOCOL"
- Docker build fails with workspace resolution errors
- pnpm version mismatch detected

## Non-Goals

What this architecture explicitly does NOT address:

1. **Renovate automation** - Manual pnpm version sync acceptable for MVP
2. **Build time optimization** - 10% slower is acceptable trade-off for correctness
3. **Image size reduction** - Already minimal (~220MB), not a bottleneck
4. **Multi-registry support** - npmjs.com sufficient for current needs
5. **Offline builds** - Not required, network assumed available
6. **Cross-platform local testing** - Docker Buildx handles multi-arch in CI

These can be future improvements if needed.
