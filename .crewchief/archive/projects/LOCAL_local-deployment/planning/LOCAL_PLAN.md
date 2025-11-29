# LOCAL: Local LLM Embedding Service - Implementation Plan

**Project Slug**: LOCAL
**Created**: 2025-10-26
**Status**: Ready for Implementation

## Executive Summary

This plan outlines a phased approach to implementing a fully containerized Maproom MCP service with local LLM embeddings (Ollama + nomic-embed-text), bundled PostgreSQL, and zero-configuration deployment.

**Timeline**: 3-4 weeks
**Team Size**: 3-4 specialized agents
**Estimated Effort**: 137-177 agent hours (increased from 120-160 due to npm package wrapper)

## Phase 1: Core Infrastructure (Week 1)

### Deliverables

1. **Docker Images**
   - Maproom MCP Dockerfile (multi-stage build)
   - PostgreSQL initialization scripts
   - Ollama model provisioning scripts

2. **Docker Compose Configuration**
   - Service definitions (postgres, ollama, maproom)
   - Volume management
   - Network configuration
   - Health checks

3. **Database Schema**
   - Complete PostgreSQL schema with pgvector
   - Index optimization for hybrid search
   - Migration scripts

4. **npm Package Wrapper**
   - @crewchief/maproom-mcp npm package structure
   - CLI wrapper with docker-compose orchestration
   - Embedded configuration files

### Tasks

| Task ID | Description | Agent | Effort | Dependencies |
|---------|-------------|-------|--------|--------------|
| LOCAL-1001 | Create Maproom Dockerfile with multi-stage build | docker-engineer | 8h | None |
| LOCAL-1002 | Write PostgreSQL init.sql schema | database-engineer | 6h | None |
| LOCAL-1003 | Create docker-compose.yml with all services | docker-engineer | 8h | LOCAL-1001 |
| LOCAL-1004 | Implement Ollama model provisioning script | docker-engineer | 4h | LOCAL-1003 |
| LOCAL-1005 | Configure health checks for all services | monitoring-observability-engineer | 4h | LOCAL-1003 |
| LOCAL-1006 | Create volume persistence strategy | docker-engineer | 3h | LOCAL-1003 |
| LOCAL-1007 | Create npm package structure for @crewchief/maproom-mcp | rust-indexer-engineer | 6h | LOCAL-1003 |
| LOCAL-1008 | Implement CLI wrapper with docker-compose orchestration | rust-indexer-engineer | 8h | LOCAL-1007 |

**Phase Milestone**: Docker Compose stack runs successfully with all services healthy AND npm package launches stack via npx

**Agent Assignments**:
- **docker-engineer**: Containerization and orchestration
- **database-engineer**: PostgreSQL schema and optimization
- **monitoring-observability-engineer**: Health checks and monitoring

## Phase 2: Ollama Integration (Week 2)

### Deliverables

1. **Ollama Provider Support**
   - Provider enum extension (Ollama variant)
   - API client modifications for Ollama compatibility
   - Configuration validation updates

2. **Embedding Service Updates**
   - Ollama-compatible API requests
   - Response parsing for Ollama format
   - Batch processing optimization

3. **Integration Tests**
   - Ollama connectivity tests
   - Embedding generation tests
   - Batch processing tests

### Tasks

| Task ID | Description | Agent | Effort | Dependencies |
|---------|-------------|-------|--------|--------------|
| LOCAL-2001 | Add Ollama variant to Provider enum | rust-indexer-engineer | 2h | LOCAL-1003 |
| LOCAL-2002 | Modify OpenAIClient for Ollama support | embeddings-engineer | 8h | LOCAL-2001 |
| LOCAL-2003 | Update EmbeddingConfig validation for Ollama | rust-indexer-engineer | 4h | LOCAL-2001 |
| LOCAL-2004 | Implement Ollama-specific request formatting | embeddings-engineer | 6h | LOCAL-2002 |
| LOCAL-2005 | Add integration tests for Ollama provider | integration-tester | 8h | LOCAL-2004 |
| LOCAL-2006 | Test batch embedding with nomic-embed-text | integration-tester | 6h | LOCAL-2004 |

**Phase Milestone**: Maproom successfully generates embeddings using Ollama

**Agent Assignments**:
- **rust-indexer-engineer**: Core Rust modifications
- **embeddings-engineer**: Embedding service implementation
- **integration-tester**: E2E and integration tests

## Phase 3: Configuration & User Experience (Week 3)

### Deliverables

1. **Zero-Config Setup**
   - Default environment variables
   - Automatic service discovery
   - Simplified user scripts

2. **Documentation**
   - README with quick start
   - Troubleshooting guide
   - Configuration reference

3. **npm Package Distribution**
   - @crewchief/maproom-mcp published to npm
   - npx-based startup (no install required)
   - Legacy package migration

### Tasks

| Task ID | Description | Agent | Effort | Dependencies |
|---------|-------------|-------|--------|--------------|
| LOCAL-3001 | Test npx @crewchief/maproom-mcp startup flow | integration-tester | 4h | LOCAL-1008 |
| LOCAL-3002 | Write README with npx installation instructions | technical-researcher | 6h | LOCAL-1008 |
| LOCAL-3003 | Implement default environment variable handling | rust-indexer-engineer | 4h | LOCAL-2002 |
| LOCAL-3004 | Create health-check.sh script | monitoring-observability-engineer | 3h | LOCAL-1005 |
| LOCAL-3005 | Write troubleshooting guide | technical-researcher | 5h | LOCAL-2006 |
| LOCAL-3006 | Add configuration reference documentation | technical-researcher | 4h | LOCAL-2003 |
| LOCAL-3007 | Update legacy maproom-mcp with deprecation wrapper | rust-indexer-engineer | 4h | LOCAL-1008 |
| LOCAL-3008 | Publish @crewchief/maproom-mcp to npm (test release) | rust-indexer-engineer | 2h | LOCAL-3007 |

**Phase Milestone**: Users can start Maproom with `npx @crewchief/maproom-mcp` from .mcp.json, no configuration required

**Agent Assignments**:
- **docker-engineer**: Deployment scripts
- **technical-researcher**: Documentation
- **monitoring-observability-engineer**: Health monitoring
- **rust-indexer-engineer**: Configuration defaults

## Phase 4: Testing & Optimization (Week 4)

### Deliverables

1. **Performance Testing**
   - Benchmark embedding generation
   - Compare Ollama vs OpenAI performance
   - Resource usage profiling

2. **Quality Assurance**
   - End-to-end workflow tests
   - Multi-platform testing (AMD64, ARM64)
   - Stress testing

3. **Optimization**
   - Reduce Docker image size
   - Optimize batch processing
   - Tune resource limits

### Tasks

| Task ID | Description | Agent | Effort | Dependencies |
|---------|-------------|-------|--------|--------------|
| LOCAL-4001 | Benchmark embedding generation performance | performance-engineer | 8h | LOCAL-3001 |
| LOCAL-4002 | Compare Ollama vs OpenAI quality metrics | search-quality-engineer | 10h | LOCAL-4001 |
| LOCAL-4003 | Profile resource usage (CPU, RAM, disk) | performance-engineer | 6h | LOCAL-4001 |
| LOCAL-4004 | Run E2E tests for full indexing workflow | integration-tester | 8h | LOCAL-3001 |
| LOCAL-4005 | Test on ARM64 architecture (Apple Silicon) | integration-tester | 6h | LOCAL-4004 |
| LOCAL-4006 | Optimize Docker image size | docker-engineer | 6h | LOCAL-4004 |
| LOCAL-4007 | Stress test with large codebase (100k chunks) | performance-engineer | 8h | LOCAL-4004 |
| LOCAL-4008 | Tune PostgreSQL configuration | database-engineer | 5h | LOCAL-4007 |

**Phase Milestone**: Production-ready containerized Maproom with performance benchmarks

**Agent Assignments**:
- **performance-engineer**: Performance testing and optimization
- **search-quality-engineer**: Quality benchmarking
- **integration-tester**: E2E and platform testing
- **docker-engineer**: Container optimization
- **database-engineer**: Database tuning

## Agent Requirements Analysis

### Existing Agents (Available)

1. **rust-indexer-engineer** ✅
   - Skills: Rust, tree-sitter, indexing pipeline
   - Tasks: Provider enum, configuration updates

2. **embeddings-engineer** ✅
   - Skills: Embedding APIs, caching, batch processing
   - Tasks: Ollama client integration

3. **database-engineer** ✅
   - Skills: PostgreSQL, pgvector, schema design
   - Tasks: Schema creation, index optimization

4. **integration-tester** ✅
   - Skills: E2E testing, workflow validation
   - Tasks: Integration tests, platform testing

5. **performance-engineer** ✅
   - Skills: Benchmarking, profiling, optimization
   - Tasks: Performance testing, stress testing

6. **search-quality-engineer** ✅
   - Skills: Search metrics, quality benchmarking
   - Tasks: Quality comparison, validation

7. **monitoring-observability-engineer** ✅
   - Skills: Metrics, logging, health checks
   - Tasks: Health monitoring, observability

8. **technical-researcher** ✅
   - Skills: Documentation, research, analysis
   - Tasks: README, guides, reference docs

### New Agents Required

1. **docker-engineer** ❌ (NEW)
   - **Reason**: No existing agent specializes in Docker/containerization
   - **Responsibilities**: Dockerfile creation, docker-compose orchestration, container optimization
   - **Skills**: Docker, Docker Compose, multi-stage builds, volume management, networking
   - **Tasks**: LOCAL-1001, LOCAL-1003, LOCAL-1004, LOCAL-1006, LOCAL-3001, LOCAL-4006

## Risk Mitigation

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Ollama API incompatibility | Medium | High | Early integration testing, fallback to OpenAI |
| Performance degradation | Medium | Medium | Benchmark early, optimize batch size, GPU support |
| Docker image too large | Low | Low | Multi-stage builds, alpine base images |
| ARM64 compatibility issues | Medium | Medium | Test on Apple Silicon early |
| Model download failures | Low | High | Retry logic, health checks, pre-baked image option |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Phase 2 delays (Ollama integration) | Medium | Medium | Allocate buffer time, parallel testing |
| Testing bottleneck in Phase 4 | Low | Medium | Start testing in Phase 3, automate tests |
| Documentation delays | Low | Low | Template docs early, incremental updates |

## Success Criteria

### Must Have (P0)

- ✅ Single command startup: `docker compose up -d` (via npx wrapper)
- ✅ Zero API key configuration required
- ✅ Works completely offline after model download
- ✅ Bundled PostgreSQL with pgvector
- ✅ Automated Ollama model provisioning
- ✅ Health checks for all services
- ✅ Data persistence with volumes

### Should Have (P1)

- ✅ Performance within 2x of OpenAI (throughput)
- ✅ Search quality within 10% of OpenAI (NDCG)
- ✅ Resource usage <6GB RAM
- ✅ Docker image <2GB
- ✅ Comprehensive documentation
- ✅ Multi-platform support (AMD64, ARM64)

### Nice to Have (P2)

- 🔄 GPU acceleration support
- 🔄 Pre-baked image with model included
- 🔄 Grafana dashboard for monitoring
- 🔄 Kubernetes manifests
- 🔄 Automatic model updates

## Deliverables Checklist

### Week 1: Infrastructure
- [ ] Maproom Dockerfile
- [ ] PostgreSQL init.sql
- [ ] docker-compose.yml
- [ ] Ollama provisioning script
- [ ] Health check configuration
- [ ] Volume strategy

### Week 2: Integration
- [ ] Provider enum with Ollama
- [ ] Ollama-compatible API client
- [ ] Configuration validation
- [ ] Request/response formatting
- [ ] Integration tests
- [ ] Batch processing tests

### Week 3: UX
- [ ] run-maproom-local.sh
- [ ] README with quick start
- [ ] Default environment variables
- [ ] Health check script
- [ ] Troubleshooting guide
- [ ] Configuration reference

### Week 4: QA
- [ ] Performance benchmarks
- [ ] Quality comparison report
- [ ] Resource usage profile
- [ ] E2E test suite
- [ ] Multi-platform testing
- [ ] Optimized Docker images
- [ ] Stress test results
- [ ] PostgreSQL tuning

## Post-Launch Roadmap

### v1.1 (Future Enhancements)
- Pre-baked Docker image with embedded model
- GPU acceleration guide
- Hybrid mode (local + OpenAI fallback)
- Custom model support (bring your own)

### v1.2 (Advanced Features)
- Kubernetes Helm chart
- Prometheus metrics exporter
- Grafana dashboard
- Multi-model support (switch between nomic/mxbai)

### v2.0 (Production Features)
- Horizontal scaling
- Model hot-swapping
- Zero-downtime updates
- Advanced observability

## Agent Onboarding

### For docker-engineer (New Agent)

**Prerequisites**:
- Docker and Docker Compose expertise
- Multi-stage build experience
- Container orchestration knowledge
- Linux systems administration

**First Tasks**:
1. Review LOCAL_ANALYSIS and LOCAL_ARCHITECTURE
2. Set up local development environment
3. Start with LOCAL-1001 (Maproom Dockerfile)
4. Coordinate with database-engineer for schema integration

**Resources**:
- Docker best practices: https://docs.docker.com/develop/dev-best-practices/
- Multi-stage builds: https://docs.docker.com/build/building/multi-stage/
- Ollama Docker: https://hub.docker.com/r/ollama/ollama

### For Existing Agents

**embeddings-engineer**:
- Review Ollama API documentation
- Understand nomic-embed-text model specifications
- Coordinate with rust-indexer-engineer for Provider changes

**database-engineer**:
- Review pgvector documentation for 768-dimension vectors
- Optimize indexes for new dimension size
- Coordinate with docker-engineer for init scripts

**integration-tester**:
- Set up Docker test environment
- Prepare test datasets for benchmarking
- Plan E2E test scenarios

## Communication Plan

### Daily Standups
- Review completed tasks
- Identify blockers
- Coordinate dependencies

### Phase Reviews
- End of each week: Phase completion review
- Milestone validation
- Go/no-go decision for next phase

### Stakeholder Updates
- Weekly progress reports
- Demo at end of Phase 2 (Ollama integration working)
- Final demo at end of Phase 4 (production ready)

## Next Steps

1. **Create docker-engineer agent definition** (see .crewchief/specialized-agents/)
2. **Review and approve this plan** with stakeholders
3. **Kick off Phase 1** with infrastructure setup
4. **Set up project tracking** (GitHub project board or similar)

## Appendix: Command Reference

### Development Commands

```bash
# Start the stack (Docker Compose v2)
docker compose up -d

# View logs
docker compose logs -f maproom
docker compose logs -f ollama

# Check service health
docker compose ps

# Execute commands in containers
docker compose exec maproom crewchief-maproom status
docker compose exec postgres psql -U maproom -d maproom

# Stop services
docker compose down

# Clean up everything (including volumes)
docker compose down -v

# Rebuild images
docker compose build --no-cache

# Pull latest images
docker compose pull
```

### Testing Commands

```bash
# Run integration tests
docker compose exec maproom cargo test --features integration

# Benchmark embedding generation
docker compose exec maproom cargo run --bin bench-embeddings

# Check Ollama model status
docker compose exec ollama ollama list

# Test embedding API directly
curl -X POST http://localhost:11434/api/embeddings \
  -d '{"model": "nomic-embed-text", "prompt": "test code snippet"}'
```

### Monitoring Commands

```bash
# Resource usage
docker stats

# Disk usage
docker system df

# Container inspection
docker inspect maproom-mcp
docker inspect maproom-ollama
docker inspect maproom-postgres
```
