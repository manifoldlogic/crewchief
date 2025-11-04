# LOCAL: Local LLM Embedding Service - Problem Space Analysis

**Project Slug**: LOCAL
**Created**: 2025-10-26
**Status**: Analysis Complete

## Executive Summary

This analysis evaluates the problem space for creating a fully containerized, zero-configuration version of Maproom MCP that includes a local LLM for embeddings, eliminating dependencies on external APIs and internet connectivity.

**Key Finding**: Using Ollama with nomic-embed-text provides the optimal balance of performance, ease of deployment, and resource efficiency for a plug-and-play containerized solution.

## Problem Statement

### Current State
- Maproom requires OpenAI API key ($0.02 per 1M tokens)
- Internet connectivity required for embedding generation
- External PostgreSQL database setup needed
- Multi-step configuration process for users
- Dependency on third-party API availability and rate limits

### Desired State
- Single command to run complete Maproom stack via npx
- Zero external dependencies (no API keys, no internet after initial setup)
- Bundled PostgreSQL database
- Local LLM for embedding generation
- Portable across different machines and architectures
- Minimal resource footprint for local development
- npm package distribution (no repository clone required)

### User Experience Goal
```json
// This should be the ONLY configuration needed in .mcp.json:
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

**First Run**: npx downloads package (~500KB) → docker pulls images (~2GB) → model download (~200MB) → ready in 2-3 minutes
**Subsequent Runs**: npx cache hit → docker starts containers → ready in 10-20 seconds

## Local LLM Embedding Options Analysis

### 1. Ollama (RECOMMENDED)

**Description**: Server framework for running LLMs locally with built-in model management.

**Pros**:
- Official Docker image with excellent container support
- Automatic model downloading and management
- Simple REST API compatible with OpenAI format
- Multiple embedding models available (nomic-embed-text, mxbai-embed-large)
- Active development and community support
- GPU support with NVIDIA Container Toolkit
- Low memory overhead when not processing

**Cons**:
- Slightly larger initial download (model + runtime)
- Requires pulling models on first run (can be automated)

**Performance Metrics**:
- nomic-embed-text: 768 dimensions, ~200MB model size
- Throughput: ~1000-2000 embeddings/sec on CPU
- Memory: ~2-4GB RAM (model loaded)
- GPU acceleration available (optional)

**Docker Integration**:
```yaml
ollama:
  image: ollama/ollama:latest
  volumes:
    - ollama-models:/root/.ollama
  ports:
    - "11434:11434"
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
```

### 2. Sentence Transformers (Alternative)

**Description**: Python library for state-of-the-art sentence and text embeddings.

**Pros**:
- Excellent model variety (all-MiniLM-L6-v2, all-mpnet-base-v2)
- Well-documented and mature
- Hugging Face integration
- Lower memory usage for smaller models

**Cons**:
- Requires custom API server implementation
- Manual model management in container
- No built-in server framework
- More complex container setup

**Performance Metrics**:
- all-MiniLM-L6-v2: 384 dimensions, ~80MB
- Throughput: ~5000-14000 sentences/sec on CPU
- Memory: ~1-2GB RAM

### 3. EmbeddingGemma (Emerging)

**Description**: Google's on-device embedding model (2025).

**Pros**:
- Extremely lightweight (<200MB RAM with quantization)
- Customizable dimensions (768 to 128)
- Multilingual support (100+ languages)
- State-of-the-art MTEB benchmark performance

**Cons**:
- Very new (limited production usage)
- Less Docker/container documentation
- Requires custom server implementation
- Smaller community compared to Ollama

**Performance Metrics**:
- Dimensions: 768 (default), can compress to 128
- Memory: <200MB with quantization
- 2K token context window

### 4. nomic-embed-text (Ollama Model - RECOMMENDED)

**Description**: State-of-the-art embedding model specifically designed for retrieval.

**Why This Model**:
- Optimized for code and text embeddings
- 768 dimensions (vs OpenAI's 1536) - smaller but effective
- Long context support (8192 tokens)
- Excellent performance on MTEB benchmarks
- Small download size (~200MB)
- Fast inference on CPU

**Performance Comparison to OpenAI**:
- Dimension reduction: 768 vs 1536 (50% smaller vectors)
- Cost: $0 vs $0.02 per 1M tokens
- Latency: Similar for local deployment
- Quality: Competitive on retrieval tasks

### 5. mxbai-embed-large (Ollama Model - Alternative)

**Description**: Large embedding model for maximum quality.

**Characteristics**:
- 1024 dimensions
- Larger model size (~500MB)
- Higher accuracy on complex tasks
- Slower inference than nomic-embed-text

**Trade-off**: Better quality vs slower speed and larger footprint.

## Recommended Solution: Ollama + nomic-embed-text

### Rationale

1. **Ease of Deployment**: Official Docker image, automatic model management
2. **Performance**: Competitive with OpenAI for code/text retrieval tasks
3. **Resource Efficiency**: ~2-4GB RAM, 200MB model download
4. **API Compatibility**: OpenAI-compatible API (minimal code changes)
5. **Maintenance**: Active development, large community
6. **User Experience**: Zero configuration model downloading

### Implementation Strategy

**Distribution via npm**:
- Package name: `@crewchief/maproom-mcp`
- Entry point: npx (no global install required)
- CLI wrapper orchestrates docker-compose
- Embedded configuration files (docker-compose.yml, init.sql)
- Proxies stdio to MCP container for IDE integration

**Container Architecture**:
- Multi-service Docker Compose setup
- Ollama service (embedding generation)
- PostgreSQL service (vector storage with pgvector)
- Maproom MCP service (API + indexing)

**Automatic Model Provisioning**:
- Init container pulls nomic-embed-text on first startup
- Health checks ensure model ready before MCP starts
- Volume persistence for model cache

**Configuration Abstraction**:
- Zero configuration required by default
- Environment variable overrides available
- Default endpoint: `http://ollama:11434`
- Volumes managed in `~/.maproom-mcp/`

**Legacy Package Migration**:
- Existing `maproom-mcp` users see deprecation notice
- Automatic forwarding to `@crewchief/maproom-mcp`
- 6-month migration timeline

## Alternative Architectures Considered

### Architecture A: All-in-One Container
**Description**: Single container with PostgreSQL, Ollama, and Maproom

**Pros**: Simplest Docker command (one docker run)
**Cons**: Violates single-responsibility, harder to scale, larger image, complex supervisord setup

**Verdict**: Rejected - too monolithic, poor maintainability

### Architecture B: npm Wrapper + Docker Compose (SELECTED)
**Description**: npm package wraps docker-compose orchestration, separate service containers

**Pros**:
- Simple user experience (npx command in .mcp.json)
- Modular containers (follows Docker best practices)
- Familiar distribution (npm ecosystem)
- No repository clone required
- Easier to maintain and debug

**Cons**:
- Requires both npm and Docker (with Compose v2 plugin) installed
- Small npx overhead (~1s cache check)

**Verdict**: Selected - best balance of UX and architecture

### Architecture C: Pure Docker Compose (No npm)
**Description**: Distribute docker-compose.yml for users to download manually

**Pros**: No npm dependency, pure Docker
**Cons**: Users must clone repo or download files manually, harder to version

**Verdict**: Rejected - worse user experience than npm wrapper

### Architecture D: Kubernetes Manifests
**Description**: Full K8s deployment with Helm charts

**Pros**: Production-ready, highly scalable
**Cons**: Overkill for local development, complex setup, requires K8s cluster

**Verdict**: Rejected - too complex for target use case (local development)

## Resource Requirements Analysis

### Minimum System Requirements
- CPU: 4 cores (2 cores minimum)
- RAM: 8GB (4GB minimum with swap)
- Disk: 5GB (2GB for images + 3GB for data)
- OS: Linux, macOS (Docker Desktop), Windows (WSL2)

### Resource Breakdown
| Component | RAM | Disk | CPU |
|-----------|-----|------|-----|
| PostgreSQL | 512MB | 500MB | 0.5 cores |
| Ollama (idle) | 1GB | 200MB | 0.1 cores |
| Ollama (active) | 2-3GB | 200MB | 1-2 cores |
| Maproom MCP | 512MB | 100MB | 0.5 cores |
| **Total** | **4-5GB** | **800MB** | **2-3 cores** |

### Performance Targets
- Embedding generation: 500-1000 chunks/minute (CPU)
- Embedding generation: 2000-5000 chunks/minute (GPU)
- Search latency: <100ms (p95) for hybrid search
- Index throughput: 100+ files/minute

## Risk Analysis

### Technical Risks
1. **Model Quality**: Local models may have lower accuracy than OpenAI
   - *Mitigation*: Benchmarking on code retrieval tasks, provide OpenAI fallback option

2. **Resource Constraints**: Users with limited RAM may struggle
   - *Mitigation*: Document minimum requirements, provide lightweight mode

3. **GPU Support**: NVIDIA GPU passthrough in Docker can be complex
   - *Mitigation*: Default to CPU mode, GPU as optional enhancement

4. **Model Download Time**: First startup may be slow (200MB download)
   - *Mitigation*: Progress indicators, optional pre-baked image with model included

### User Experience Risks
1. **Docker Compose v2 Requirement**: Users may have old docker-compose binary instead of plugin
   - *Mitigation*: CLI wrapper checks for `docker compose version` and provides clear error message if missing

2. **Port Conflicts**: Default ports may be in use
   - *Mitigation*: Configurable ports via environment variables

3. **Data Persistence**: Users may lose data if volumes not configured
   - *Mitigation*: Clear documentation, automatic volume creation

## Success Criteria

### Must Have
- ✅ Zero API key configuration required
- ✅ Works completely offline after initial model download
- ✅ Single command to start entire stack
- ✅ Bundled PostgreSQL with pgvector
- ✅ Automated model downloading on first run

### Should Have
- ✅ GPU acceleration support (optional)
- ✅ Hybrid mode (local + OpenAI fallback)
- ✅ Health checks and automatic recovery
- ✅ Volume persistence for data and models
- ✅ Resource usage monitoring

### Nice to Have
- 🔄 Pre-baked image with model included (optional download)
- 🔄 Multi-architecture support (AMD64, ARM64)
- 🔄 Kubernetes manifests for production deployment
- 🔄 Observability dashboard (Grafana)

## Cost-Benefit Analysis

### Benefits
- **Zero Ongoing Costs**: No API fees
- **Privacy**: All data stays local
- **Reliability**: No external API dependencies
- **Speed**: No network latency for embeddings
- **Portability**: Run anywhere Docker runs

### Costs
- **Initial Setup**: Model download time (~1-2 minutes)
- **Resource Usage**: 4-5GB RAM vs ~512MB for API-only
- **Maintenance**: Need to update models manually
- **Quality Trade-off**: Potentially lower accuracy vs GPT-based embeddings

### Break-Even Point
For embedding generation:
- OpenAI cost: $0.02 per 1M tokens
- Average code chunk: ~200 tokens
- 5,000 chunks = 1M tokens = $0.02
- **Break-even**: After indexing ~250,000 chunks (50 medium projects)

For most users, the local model is cost-effective immediately due to zero API costs.

## Conclusion

**Recommendation**: Implement Docker Compose solution with Ollama + nomic-embed-text

**Key Decision Factors**:
1. Ollama provides best developer experience with minimal configuration
2. nomic-embed-text offers optimal performance/size/quality trade-off
3. Docker Compose enables modular, maintainable architecture
4. Resource requirements are reasonable for modern development machines
5. Zero-cost and offline capability align with project goals

**Next Steps**: Proceed to LOCAL_ARCHITECTURE for technical design details.
