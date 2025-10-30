# DKRHUB: Docker Hub Publishing - Technical Architecture

**Project Slug**: DKRHUB
**Created**: 2025-10-29
**Status**: Architecture Design

## Architecture Overview

This document defines the technical architecture for publishing Maproom MCP Docker images to Docker Hub via automated CI/CD, enabling seamless deployment of the npm package.

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          GitHub Repository                              │
│                                                                         │
│  packages/maproom-mcp/                                                  │
│  ├── src/                 (TypeScript source)                           │
│  ├── config/                                                            │
│  │   ├── docker-compose.yml           (UPDATED - uses image:)          │
│  │   ├── docker-compose.override.yml  (NEW - for local builds)         │
│  │   └── Dockerfile.mcp-server        (existing build instructions)    │
│  └── package.json                                                       │
│                                                                         │
│  .github/workflows/                                                     │
│  └── publish-maproom-mcp-image.yml  (NEW - GitHub Actions workflow)    │
│                                                                         │
│         │                                                               │
│         │ Push tag: v1.1.10                                             │
│         ▼                                                               │
│  ┌────────────────────────────────────────────────────────────┐        │
│  │            GitHub Actions Workflow                         │        │
│  │                                                             │        │
│  │  1. Checkout code                                          │        │
│  │  2. Set up Docker Buildx (multi-platform support)          │        │
│  │  3. Login to Docker Hub (DOCKERHUB_TOKEN)                  │        │
│  │  4. Extract version from tag (1.1.10)                      │        │
│  │  5. Build for linux/amd64 + linux/arm64                    │        │
│  │  6. Tag: 1.1.10, 1.1, 1, latest                            │        │
│  │  7. Push to Docker Hub                                     │        │
│  │  8. Run security scan (Trivy)                              │        │
│  └───────────────────────┬────────────────────────────────────┘        │
│                          │                                             │
└──────────────────────────┼─────────────────────────────────────────────┘
                           │
                           │ Push multi-arch image
                           ▼
                  ┌────────────────────┐
                  │    Docker Hub      │
                  │                    │
                  │  crewchief/        │
                  │  maproom-mcp:      │
                  │   - 1.1.10         │
                  │   - 1.1            │
                  │   - 1              │
                  │   - latest         │
                  │                    │
                  │  (AMD64 + ARM64)   │
                  └──────┬─────────────┘
                         │
                         │ Pull image
                         ▼
           ┌─────────────────────────────────┐
           │      User's System              │
           │                                 │
           │  $ npm install -g               │
           │    @crewchief/maproom-mcp       │
           │                                 │
           │  $ npx @crewchief/maproom-mcp   │
           │    start                        │
           │         │                       │
           │         ▼                       │
           │  docker-compose up -d           │
           │         │                       │
           │         ├─► Pull: crewchief/   │
           │         │   maproom-mcp:1.1.10 │
           │         │   (200MB, multi-arch) │
           │         │                       │
           │         ├─► Pull: pgvector/    │
           │         │   pgvector:pg16      │
           │         │                       │
           │         └─► Pull: ollama/      │
           │             ollama:latest      │
           │                                 │
           │  Services running in ~30s      │
           └─────────────────────────────────┘
```

## Component Specifications

### 1. GitHub Actions Workflow

**File**: `.github/workflows/publish-maproom-mcp-image.yml`

#### Workflow Configuration

```yaml
name: Publish Maproom MCP Docker Image

# Trigger only on version tags (v1.2.3, v2.0.0, etc.)
on:
  push:
    tags:
      - 'v*.*.*'
  # Allow manual triggering for testing
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish (e.g., 1.1.10)'
        required: true
      push_to_registry:
        description: 'Push to Docker Hub (true/false)'
        required: true
        default: 'false'

# Permissions for GitHub token
permissions:
  contents: read
  packages: write  # For potential ghcr.io support later

env:
  DOCKER_HUB_REPO: crewchief/maproom-mcp
  DOCKERFILE_PATH: packages/maproom-mcp/config/Dockerfile.mcp-server
  BUILD_CONTEXT: packages/maproom-mcp

jobs:
  build-and-push:
    runs-on: ubuntu-latest

    steps:
      # 1. Checkout repository
      - name: Checkout code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for git metadata

      # 2. Set up QEMU for multi-platform emulation
      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3
        with:
          platforms: linux/amd64,linux/arm64

      # 3. Set up Docker Buildx (multi-platform builder)
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
        with:
          driver-opts: |
            image=moby/buildkit:latest
            network=host

      # 4. Extract version from git tag
      - name: Extract version
        id: version
        run: |
          if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
            VERSION="${{ github.event.inputs.version }}"
          else
            # Remove 'v' prefix from tag (v1.1.10 → 1.1.10)
            VERSION="${GITHUB_REF#refs/tags/v}"
          fi

          echo "full=$VERSION" >> $GITHUB_OUTPUT

          # Extract major.minor (1.1.10 → 1.1)
          MINOR_VERSION=$(echo $VERSION | cut -d. -f1-2)
          echo "minor=$MINOR_VERSION" >> $GITHUB_OUTPUT

          # Extract major (1.1.10 → 1)
          MAJOR_VERSION=$(echo $VERSION | cut -d. -f1)
          echo "major=$MAJOR_VERSION" >> $GITHUB_OUTPUT

          echo "Version: $VERSION (major: $MAJOR_VERSION, minor: $MINOR_VERSION)"

      # 5. Login to Docker Hub
      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKERHUB_USERNAME }}
          password: ${{ secrets.DOCKERHUB_TOKEN }}

      # 6. Generate image metadata (tags, labels)
      - name: Generate Docker metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.DOCKER_HUB_REPO }}
          tags: |
            # Full version (1.1.10)
            type=raw,value=${{ steps.version.outputs.full }}
            # Minor version (1.1)
            type=raw,value=${{ steps.version.outputs.minor }}
            # Major version (1)
            type=raw,value=${{ steps.version.outputs.major }}
            # Latest (for most recent release)
            type=raw,value=latest
          labels: |
            org.opencontainers.image.title=Maproom MCP Server
            org.opencontainers.image.description=Semantic code search MCP server with local LLM embeddings
            org.opencontainers.image.vendor=CrewChief
            org.opencontainers.image.version=${{ steps.version.outputs.full }}

      # 7. Build and push multi-platform image
      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: ${{ env.BUILD_CONTEXT }}
          file: ${{ env.DOCKERFILE_PATH }}
          platforms: linux/amd64,linux/arm64
          push: ${{ github.event_name != 'workflow_dispatch' || github.event.inputs.push_to_registry == 'true' }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
          build-args: |
            VERSION=${{ steps.version.outputs.full }}
            COMMIT_SHA=${{ github.sha }}
            BUILD_DATE=${{ github.event.head_commit.timestamp }}

      # 8. Run security scan with Trivy
      - name: Run Trivy security scan
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: ${{ env.DOCKER_HUB_REPO }}:${{ steps.version.outputs.full }}
          format: 'sarif'
          output: 'trivy-results.sarif'
          severity: 'CRITICAL,HIGH'

      # 9. Upload Trivy results to GitHub Security
      - name: Upload Trivy results to GitHub Security
        uses: github/codeql-action/upload-sarif@v2
        if: always()
        with:
          sarif_file: 'trivy-results.sarif'

      # 10. Generate build summary
      - name: Generate build summary
        if: always()
        run: |
          echo "## Docker Image Build Summary" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          echo "**Version**: ${{ steps.version.outputs.full }}" >> $GITHUB_STEP_SUMMARY
          echo "**Tags**: ${{ steps.version.outputs.full }}, ${{ steps.version.outputs.minor }}, ${{ steps.version.outputs.major }}, latest" >> $GITHUB_STEP_SUMMARY
          echo "**Platforms**: linux/amd64, linux/arm64" >> $GITHUB_STEP_SUMMARY
          echo "**Repository**: ${{ env.DOCKER_HUB_REPO }}" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          echo "**Pull command**:" >> $GITHUB_STEP_SUMMARY
          echo "\`\`\`bash" >> $GITHUB_STEP_SUMMARY
          echo "docker pull ${{ env.DOCKER_HUB_REPO }}:${{ steps.version.outputs.full }}" >> $GITHUB_STEP_SUMMARY
          echo "\`\`\`" >> $GITHUB_STEP_SUMMARY
```

#### Workflow Features

1. **Multi-Platform Builds**:
   - Uses Docker Buildx with QEMU for cross-compilation
   - Builds for `linux/amd64` (x86_64) and `linux/arm64` (ARM64/Apple Silicon)
   - Creates manifest list (Docker automatically selects platform)

2. **Intelligent Tagging**:
   - Full version: `1.1.10` (immutable, specific release)
   - Minor version: `1.1` (moves with patch releases)
   - Major version: `1` (moves with minor/patch releases)
   - Latest: `latest` (always points to newest release)

3. **Caching Strategy**:
   - GitHub Actions cache (`cache-from: type=gha`)
   - Reuses layers between builds
   - Significantly speeds up subsequent builds (2-3min vs 10min)

4. **Security Scanning**:
   - Trivy scans for vulnerabilities (CRITICAL, HIGH)
   - Results uploaded to GitHub Security tab
   - Fails build if critical vulnerabilities found (configurable)

5. **Build Metadata**:
   - OCI-compliant labels (title, description, version, vendor)
   - Git commit SHA embedded in image
   - Build timestamp for traceability

6. **Manual Testing Support**:
   - `workflow_dispatch` allows manual triggering
   - Can build without pushing (for testing)
   - Useful for pre-release validation

### 2. Updated docker-compose.yml

**File**: `packages/maproom-mcp/config/docker-compose.yml`

#### Production Configuration (Published to npm)

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg16
    container_name: maproom-postgres
    environment:
      POSTGRES_DB: maproom
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
      PGDATA: /var/lib/postgresql/data/pgdata
    volumes:
      - maproom-data:/var/lib/postgresql/data
    ports:
      - "127.0.0.1:15433:5432"
    command: >
      postgres
      -c max_connections=50
      -c shared_buffers=512MB
      -c effective_cache_size=3GB
      -c maintenance_work_mem=256MB
      -c checkpoint_completion_target=0.9
      -c wal_buffers=16MB
      -c default_statistics_target=100
      -c random_page_cost=1.1
      -c effective_io_concurrency=200
      -c maintenance_io_concurrency=100
      -c work_mem=16MB
      -c min_wal_size=512MB
      -c max_wal_size=2GB
      -c max_parallel_workers_per_gather=2
      -c max_parallel_workers=4
    networks:
      maproom-network:
        aliases:
          - maproom-postgres
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U maproom -d maproom"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    restart: unless-stopped

  ollama:
    image: ollama/ollama:latest
    container_name: maproom-ollama
    volumes:
      - ollama-models:/root/.ollama
    ports:
      - "127.0.0.1:${OLLAMA_PORT:-11434}:11434"
    networks:
      - maproom-network
    healthcheck:
      test: ["CMD", "ollama", "list"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 120s
    restart: unless-stopped
    entrypoint: ["/bin/sh", "-c"]
    command: >
      "ollama serve &
      OLLAMA_PID=$$!;
      echo 'Waiting for Ollama server...';
      sleep 5;
      echo 'Pulling nomic-embed-text model...';
      ollama pull nomic-embed-text;
      echo 'Model ready!';
      wait $$OLLAMA_PID"

  # MCP server service - UPDATED to use pre-built image
  maproom-mcp:
    # Pull pre-built image from Docker Hub (UPDATED)
    image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}

    # Build section REMOVED - no longer building from source
    # (Development builds use docker-compose.override.yml)

    container_name: maproom-mcp
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
      EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}
      EMBEDDING_MODEL: ${EMBEDDING_MODEL:-nomic-embed-text}
      EMBEDDING_DIMENSION: ${EMBEDDING_DIMENSION:-768}
      EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-http://ollama:11434}
      GOOGLE_PROJECT_ID: ${GOOGLE_PROJECT_ID:-}
      GOOGLE_APPLICATION_CREDENTIALS: ${GOOGLE_APPLICATION_CREDENTIALS:-}
      GOOGLE_VERTEX_REGION: ${GOOGLE_VERTEX_REGION:-us-west1}
      OPENAI_API_KEY: ${OPENAI_API_KEY:-}
      LOG_LEVEL: ${LOG_LEVEL:-info}
      MAPROOM_MCP_LOG_FILE: /app/logs/mcp.log
      NODE_ENV: production
    volumes:
      - maproom-logs:/app/logs
    networks:
      - maproom-network
    stdin_open: true
    tty: false
    healthcheck:
      test: ["CMD", "sh", "-c", "pg_isready -h maproom-postgres -U maproom || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 30s
    restart: unless-stopped

volumes:
  maproom-data:
    driver: local
  ollama-models:
    driver: local
  maproom-logs:
    driver: local

networks:
  maproom-network:
    driver: bridge
```

**Key Changes**:
1. **Line 87-90**: Changed from `build:` to `image: crewchief/maproom-mcp:${MAPROOM_VERSION:-latest}`
2. **Environment Variable**: `MAPROOM_VERSION` allows version pinning (defaults to `latest`)
3. **No Build Context**: Removed `context: ../../..` and `dockerfile:` directives

### 3. Development Override Configuration

**File**: `packages/maproom-mcp/config/docker-compose.override.yml`
(For local development only - NOT published to npm)

```yaml
# Development override - allows building from source
# Place this file in your development workspace root
# Docker Compose automatically merges this with docker-compose.yml

services:
  maproom-mcp:
    # Override image with build configuration for local development
    build:
      context: ../../..
      dockerfile: packages/maproom-mcp/config/Dockerfile.mcp-server
    # Image directive is overridden by build in override files
```

**Usage**:
```bash
# In development workspace:
cd packages/maproom-mcp/config/

# Build from source (uses override)
docker-compose build

# Run with local build
docker-compose up -d

# In production deployment (npm package):
# No override file exists, so uses image: from docker-compose.yml
npx @crewchief/maproom-mcp start
```

### 4. Dockerfile Enhancements

**File**: `packages/maproom-mcp/config/Dockerfile.mcp-server` (existing, minor updates)

```dockerfile
# ========================================
# Stage 1: Build Stage
# ========================================
FROM node:20-alpine AS builder

# Build arguments for metadata
ARG VERSION=unknown
ARG COMMIT_SHA=unknown
ARG BUILD_DATE=unknown

# Install build dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++

WORKDIR /build

# Copy package files first for better caching
COPY package.json ./

# Install dependencies
RUN echo '{"dependencies":{"pg":"^8.11.3","pino":"^8.16.2","zod":"^3.22.4","execa":"^8.0.1"},"devDependencies":{"typescript":"^5.3.3","@types/node":"^20.10.5","@types/pg":"^8.10.9"}}' > temp-package.json && \
    npm install --production=false --package-lock=false --no-audit --no-fund pg pino zod execa typescript @types/node @types/pg || \
    (cat temp-package.json >> package.json && npm install --production=false --package-lock=false --no-audit --no-fund)

# Copy source
COPY tsconfig.json ./
COPY src/ ./src/

# Compile TypeScript
RUN npx tsc

# ========================================
# Stage 2: Runtime Stage
# ========================================
FROM node:20-alpine

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    postgresql-client

# Create directories
RUN mkdir -p /app/dist /app/logs && \
    chown -R node:node /app

WORKDIR /app

# Copy package.json
COPY package.json ./

# Install production dependencies
RUN npm install --production --no-audit --no-fund --no-optional pg pino zod execa || \
    (echo '{"dependencies":{"pg":"^8.11.3","pino":"^8.16.2","zod":"^3.22.4","execa":"^8.0.1"}}' > package.json && \
     npm install --production --no-audit --no-fund --no-optional)

# Copy compiled code
COPY --from=builder /build/dist ./dist
COPY --from=builder /build/src/tools ./src/tools

# Add metadata labels (NEW)
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.revision="${COMMIT_SHA}"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.title="Maproom MCP Server"
LABEL org.opencontainers.image.description="Semantic code search MCP server"
LABEL org.opencontainers.image.vendor="CrewChief"

# Switch to non-root user
USER node

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD pg_isready -h postgres -U maproom || exit 1

# Entrypoint
ENTRYPOINT ["node", "/app/dist/index.js"]
```

**Enhancements**:
1. Build arguments for version tracking
2. OCI-compliant metadata labels
3. Multi-stage build already optimized

### 5. Version Management Strategy

#### Tag Naming Convention

**Semantic Versioning** (MAJOR.MINOR.PATCH):
- `1.1.10` - Patch release (bug fixes, no breaking changes)
- `1.2.0` - Minor release (new features, backward compatible)
- `2.0.0` - Major release (breaking changes)

#### Docker Image Tags

For release `v1.1.10`, create these tags:

1. **Full Version** (`1.1.10`):
   - Immutable, specific release
   - Use for production pinning
   - Never changes after creation
   - Example: `docker pull crewchief/maproom-mcp:1.1.10`

2. **Minor Version** (`1.1`):
   - Points to latest patch in 1.1.x series
   - Moves with patch releases (1.1.10 → 1.1.11)
   - Good for "stay updated within minor version"
   - Example: `docker pull crewchief/maproom-mcp:1.1`

3. **Major Version** (`1`):
   - Points to latest release in 1.x.x series
   - Moves with minor and patch releases
   - Good for "stay on major version, accept minor updates"
   - Example: `docker pull crewchief/maproom-mcp:1`

4. **Latest** (`latest`):
   - Points to most recent release (any version)
   - Moves with every release
   - Good for "always use newest"
   - Example: `docker pull crewchief/maproom-mcp:latest`

#### Recommended Usage

**For Users** (via npm package):
```bash
# Default: latest (set in docker-compose.yml)
npx @crewchief/maproom-mcp start

# Pin to specific version (set MAPROOM_VERSION env var)
MAPROOM_VERSION=1.1.10 npx @crewchief/maproom-mcp start

# Pin to minor version (get patches automatically)
MAPROOM_VERSION=1.1 npx @crewchief/maproom-mcp start
```

**For Developers** (testing):
```bash
# Test specific version
docker pull crewchief/maproom-mcp:1.1.10
docker run crewchief/maproom-mcp:1.1.10

# Test latest
docker pull crewchief/maproom-mcp:latest
```

#### Update Strategy

**When to update docker-compose.yml default**:
- Major releases: Update to new major version (`2` when releasing 2.0.0)
- Keep `latest` as default for most users
- Document version pinning for stability-critical deployments

### 6. npm Package Integration

#### package.json Updates

**File**: `packages/maproom-mcp/package.json`

```json
{
  "name": "@crewchief/maproom-mcp",
  "version": "1.1.10",
  "description": "Maproom MCP server with local LLM embeddings - zero configuration required",
  "bin": {
    "maproom-mcp": "./bin/cli.cjs"
  },
  "files": [
    "bin/cli.cjs",
    "config/docker-compose.yml",
    "config/init.sql",
    "dist/",
    "src/",
    "tsconfig.json",
    "README.md",
    "LICENSE"
  ],
  "scripts": {
    "build": "tsc",
    "test": "node bin/cli.cjs --test",
    "dev": "node bin/cli.cjs",
    "prepublishOnly": "tsc && pnpm audit --audit-level=high --prod"
  }
}
```

**Key Points**:
1. **No Dockerfile in files array**: Not needed - image comes from Docker Hub
2. **docker-compose.yml included**: Uses `image:` instead of `build:`
3. **Small package size**: ~50KB (no build context, no large files)

#### CLI Wrapper Behavior

The CLI wrapper (`bin/cli.cjs`) already handles docker-compose correctly:
```javascript
// Existing code (no changes needed)
const configPath = path.join(__dirname, '../config/docker-compose.yml');
execSync(`docker-compose -f ${configPath} up -d`, { stdio: 'inherit' });
```

Docker Compose will:
1. Read `docker-compose.yml`
2. See `image: crewchief/maproom-mcp:latest`
3. Pull from Docker Hub if not cached
4. Start container

### 7. Build Performance Optimization

#### Caching Strategy

**Layer Caching**:
```dockerfile
# Good: Dependencies cached separately from source
COPY package.json ./
RUN npm install

# Then copy source (changes frequently)
COPY src/ ./src/
```

**GitHub Actions Cache**:
```yaml
cache-from: type=gha
cache-to: type=gha,mode=max
```
- Reuses layers between builds
- Saves 5-8 minutes per build
- Free on GitHub Actions

#### Build Time Estimates

**First Build** (cold cache):
- AMD64: ~8-10 minutes
- ARM64: ~12-15 minutes (QEMU emulation)
- Total: ~15 minutes

**Subsequent Builds** (warm cache):
- AMD64: ~2-3 minutes
- ARM64: ~3-4 minutes
- Total: ~5 minutes

**Image Size**:
- Uncompressed: ~300MB
- Compressed (download): ~120MB
- node:20-alpine base: ~180MB
- Added layers: ~40MB

### 8. Rollback and Versioning

#### Rollback Strategy

**If v1.1.10 has issues**:
```bash
# Users can pin to previous version
MAPROOM_VERSION=1.1.9 npx @crewchief/maproom-mcp start

# Or pull specific older image
docker pull crewchief/maproom-mcp:1.1.9
```

**Maintaining Old Versions**:
- Images never deleted from Docker Hub
- All versions remain pullable indefinitely
- No automatic pruning

#### Version Compatibility Matrix

| npm Package Version | Docker Image Version | Compatible |
|---------------------|----------------------|------------|
| 1.1.10+             | 1.1.10+              | Yes        |
| 1.1.9               | 1.1.9 (broken)       | No         |
| 1.1.8               | N/A (pre-Docker Hub) | N/A        |

### 9. Monitoring and Observability

#### Build Monitoring

**GitHub Actions Dashboard**:
- Workflow status (success/failure)
- Build duration
- Cache hit rate
- Security scan results

**Docker Hub Insights**:
- Pull count per tag
- Geographic distribution
- Platform breakdown (AMD64 vs ARM64)

#### Runtime Monitoring

**Container Health**:
```yaml
healthcheck:
  test: ["CMD", "sh", "-c", "pg_isready -h maproom-postgres -U maproom || exit 1"]
  interval: 30s
  timeout: 5s
  retries: 3
  start_period: 30s
```

**Logs**:
```bash
# View container logs
docker logs maproom-mcp

# Follow logs
docker logs -f maproom-mcp

# View compose logs
docker-compose -f ~/.maproom-mcp/config/docker-compose.yml logs
```

### 10. Security Considerations

(See DKRHUB_SECURITY_REVIEW.md for detailed security analysis)

#### Key Security Measures

1. **Non-root User**: Container runs as `node` (uid 1000)
2. **Minimal Base**: node:20-alpine reduces attack surface
3. **No Secrets**: No credentials baked into image
4. **Vulnerability Scanning**: Trivy scans on every build
5. **Signed Images**: Plan to add Cosign signatures (future)

## Implementation Checklist

### Phase 1: GitHub Actions Workflow
- [ ] Create `.github/workflows/publish-maproom-mcp-image.yml`
- [ ] Configure workflow triggers (tags, manual)
- [ ] Set up Docker Buildx with multi-platform
- [ ] Configure Docker Hub login with secrets
- [ ] Implement version extraction logic
- [ ] Configure image metadata generation
- [ ] Set up build caching
- [ ] Add Trivy security scanning
- [ ] Add build summary generation

### Phase 2: Docker Compose Updates
- [ ] Update `docker-compose.yml` to use `image:` instead of `build:`
- [ ] Add `MAPROOM_VERSION` environment variable support
- [ ] Create `docker-compose.override.yml` for development
- [ ] Update documentation for override pattern
- [ ] Test production compose (pull from Docker Hub)
- [ ] Test development compose (build from source)

### Phase 3: Dockerfile Enhancements
- [ ] Add build arguments (VERSION, COMMIT_SHA, BUILD_DATE)
- [ ] Add OCI metadata labels
- [ ] Verify multi-stage build optimization
- [ ] Test image size (<500MB target)

### Phase 4: npm Package Updates
- [ ] Update package.json to v1.1.10
- [ ] Remove Dockerfile from files array (if present)
- [ ] Verify docker-compose.yml is included
- [ ] Update README with Docker Hub instructions
- [ ] Add version pinning documentation

### Phase 5: Testing
- [ ] Test workflow with test tag (v1.1.10-rc1)
- [ ] Verify multi-platform builds
- [ ] Test image pull on AMD64 Linux
- [ ] Test image pull on ARM64 macOS
- [ ] Integration test: clean install → start → verify
- [ ] Performance test: startup time < 30s

### Phase 6: Release
- [ ] Tag v1.1.10 in git
- [ ] Verify GitHub Actions workflow succeeds
- [ ] Verify images appear on Docker Hub
- [ ] Publish npm package v1.1.10
- [ ] Test end-to-end: npm install → npx start
- [ ] Update CHANGELOG.md

## Next Steps

1. **Review DKRHUB_SECURITY_REVIEW.md** for security considerations
2. **Review DKRHUB_QUALITY_STRATEGY.md** for testing approach
3. **Review DKRHUB_PLAN.md** for implementation roadmap
4. **Begin Phase 1 implementation** (GitHub Actions workflow)

---

**Status**: Architecture design complete, ready for implementation
