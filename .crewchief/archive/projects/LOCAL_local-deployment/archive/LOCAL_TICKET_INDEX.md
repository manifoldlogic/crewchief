# LOCAL Project - Work Ticket Index

**Project**: LOCAL - Containerized Maproom with Local LLM Embeddings
**Status**: Ready for Implementation
**Created**: 2025-10-26
**Total Tickets**: 30

## Project Overview

The LOCAL project implements a fully containerized version of Maproom MCP with local LLM embeddings (Ollama + nomic-embed-text), bundled PostgreSQL, and zero-configuration deployment via npm. Users can run the entire stack with a single npx command.

### Key Goals
- ✅ Zero Configuration (single npx command in .mcp.json)
- ✅ Offline Operation (works completely offline after initial setup)
- ✅ No API Keys (local LLM eliminates OpenAI dependency)
- ✅ Bundled Database (PostgreSQL with pgvector included)
- ✅ npm Distribution (no repository clone required)
- ✅ Portable (run anywhere Docker + npm runs)

### Planning Documents
- [LOCAL/README.md](../../crewchief_context/maproom/LOCAL/README.md) - Project overview
- [LOCAL/LOCAL_PLAN.md](../../crewchief_context/maproom/LOCAL/LOCAL_PLAN.md) - Implementation roadmap
- [LOCAL/LOCAL_ARCHITECTURE.md](../../crewchief_context/maproom/LOCAL/LOCAL_ARCHITECTURE.md) - Technical design
- [LOCAL/LOCAL_ANALYSIS.md](../../crewchief_context/maproom/LOCAL/LOCAL_ANALYSIS.md) - Problem analysis

---

## Phase 1: Core Infrastructure (Week 1)

**Goal**: Build Docker infrastructure with PostgreSQL, Ollama, and Maproom services orchestrated via Docker Compose, distributed as an npm package.

**Milestone**: Docker Compose stack runs successfully with all services healthy AND npm package launches stack via npx.

### Tickets (8)

| Ticket | Title | Agent | Effort | Dependencies |
|--------|-------|-------|--------|--------------|
| [LOCAL-1001](LOCAL-1001_maproom-dockerfile-multistage.md) | Create Maproom Dockerfile with multi-stage build | docker-engineer | 8h | None |
| [LOCAL-1002](LOCAL-1002_postgresql-init-schema.md) | Write PostgreSQL init.sql schema | database-engineer | 6h | None |
| [LOCAL-1003](LOCAL-1003_docker-compose-orchestration.md) | Create docker-compose.yml with all services | docker-engineer | 8h | LOCAL-1001 |
| [LOCAL-1004](LOCAL-1004_ollama-model-provisioning.md) | Implement Ollama model provisioning script | docker-engineer | 4h | LOCAL-1003 |
| [LOCAL-1005](LOCAL-1005_configure-health-checks.md) | Configure health checks for all services | monitoring-observability-engineer | 4h | LOCAL-1003 |
| [LOCAL-1006](LOCAL-1006_volume-persistence-strategy.md) | Create volume persistence strategy | docker-engineer | 3h | LOCAL-1003 |
| [LOCAL-1007](LOCAL-1007_npm-package-structure.md) | Create npm package structure for @crewchief/maproom-mcp | rust-indexer-engineer | 6h | LOCAL-1003 |
| [LOCAL-1008](LOCAL-1008_cli-wrapper-docker-compose.md) | Implement CLI wrapper with docker-compose orchestration | rust-indexer-engineer | 8h | LOCAL-1007 |

**Total Effort**: 47 hours

---

## Phase 2: Ollama Integration (Week 2)

**Goal**: Integrate Ollama provider support into the Rust codebase with API client modifications, configuration validation, and comprehensive integration testing.

**Milestone**: Maproom successfully generates embeddings using Ollama with nomic-embed-text model.

### Tickets (6)

| Ticket | Title | Agent | Effort | Dependencies |
|--------|-------|-------|--------|--------------|
| [LOCAL-2001](LOCAL-2001_add-ollama-provider-enum.md) | Add Ollama variant to Provider enum | rust-indexer-engineer | 2h | LOCAL-1003 |
| [LOCAL-2002](LOCAL-2002_ollama-client-support.md) | Modify OpenAIClient for Ollama support | embeddings-engineer | 8h | LOCAL-2001 |
| [LOCAL-2003](LOCAL-2003_update-embedding-config-validation-ollama.md) | Update EmbeddingConfig validation for Ollama | rust-indexer-engineer | 4h | LOCAL-2001 |
| [LOCAL-2004](LOCAL-2004_ollama-request-formatting.md) | Implement Ollama-specific request formatting | embeddings-engineer | 6h | LOCAL-2002 |
| [LOCAL-2005](LOCAL-2005_ollama-integration-tests.md) | Add integration tests for Ollama provider | integration-tester | 8h | LOCAL-2004 |
| [LOCAL-2006](LOCAL-2006_test-batch-embedding-nomic.md) | Test batch embedding with nomic-embed-text | integration-tester | 6h | LOCAL-2004 |

**Total Effort**: 34 hours

---

## Phase 3: Configuration & User Experience (Week 3)

**Goal**: Deliver zero-configuration UX with comprehensive documentation, troubleshooting guides, and npm package publication.

**Milestone**: Users can start Maproom with `npx @crewchief/maproom-mcp` from .mcp.json, no configuration required.

### Tickets (8)

| Ticket | Title | Agent | Effort | Dependencies |
|--------|-------|-------|--------|--------------|
| [LOCAL-3001](LOCAL-3001_test-npx-startup-flow.md) | Test npx @crewchief/maproom-mcp startup flow | integration-tester | 4h | LOCAL-1008 |
| [LOCAL-3002](LOCAL-3002_readme-npx-installation.md) | Write README with npx installation instructions | technical-researcher | 6h | LOCAL-1008 |
| [LOCAL-3003](LOCAL-3003_default-environment-variable-handling.md) | Implement default environment variable handling | rust-indexer-engineer | 4h | LOCAL-2002 |
| [LOCAL-3004](LOCAL-3004_health-check-script.md) | Create health-check.sh script | monitoring-observability-engineer | 3h | LOCAL-1005 |
| [LOCAL-3005](LOCAL-3005_write-troubleshooting-guide.md) | Write troubleshooting guide | technical-researcher | 5h | LOCAL-2006 |
| [LOCAL-3006](LOCAL-3006_configuration-reference-documentation.md) | Add configuration reference documentation | technical-researcher | 4h | LOCAL-2003 |
| [LOCAL-3007](LOCAL-3007_update-legacy-maproom-mcp-deprecation-wrapper.md) | Update legacy maproom-mcp with deprecation wrapper | rust-indexer-engineer | 4h | LOCAL-1008 |
| [LOCAL-3008](LOCAL-3008_npm-publish-test-release.md) | Publish @crewchief/maproom-mcp to npm (test release) | rust-indexer-engineer | 2h | LOCAL-3007 |

**Total Effort**: 32 hours

---

## Phase 4: Testing & Optimization (Week 4)

**Goal**: Validate production readiness through comprehensive testing, performance benchmarking, quality comparison, and optimization.

**Milestone**: Production-ready containerized Maproom with performance benchmarks and quality validation.

### Tickets (8)

| Ticket | Title | Agent | Effort | Dependencies |
|--------|-------|-------|--------|--------------|
| [LOCAL-4001](LOCAL-4001_benchmark-embedding-performance.md) | Benchmark embedding generation performance | performance-engineer | 8h | LOCAL-3001 |
| [LOCAL-4002](LOCAL-4002_compare-ollama-openai-quality.md) | Compare Ollama vs OpenAI quality metrics | search-quality-engineer | 10h | LOCAL-4001 |
| [LOCAL-4003](LOCAL-4003_profile-resource-usage.md) | Profile resource usage (CPU, RAM, disk) | performance-engineer | 6h | LOCAL-4001 |
| [LOCAL-4004](LOCAL-4004_e2e-indexing-workflow-tests.md) | Run E2E tests for full indexing workflow | integration-tester | 8h | LOCAL-3001 |
| [LOCAL-4005](LOCAL-4005_arm64-apple-silicon-testing.md) | Test on ARM64 architecture (Apple Silicon) | integration-tester | 6h | LOCAL-4004 |
| [LOCAL-4006](LOCAL-4006_optimize-docker-image-size.md) | Optimize Docker image size | docker-engineer | 6h | LOCAL-4004 |
| [LOCAL-4007](LOCAL-4007_stress-test-large-codebase.md) | Stress test with large codebase (100k chunks) | performance-engineer | 8h | LOCAL-4004 |
| [LOCAL-4008](LOCAL-4008_tune-postgresql-configuration.md) | Tune PostgreSQL configuration | database-engineer | 5h | LOCAL-4007 |

**Total Effort**: 57 hours

---

## Summary by Agent

### Agent Workload Distribution

| Agent | Ticket Count | Total Effort |
|-------|--------------|--------------|
| docker-engineer | 6 | 35h |
| rust-indexer-engineer | 5 | 26h |
| embeddings-engineer | 2 | 14h |
| integration-tester | 5 | 32h |
| performance-engineer | 3 | 22h |
| database-engineer | 2 | 11h |
| technical-researcher | 3 | 15h |
| monitoring-observability-engineer | 2 | 7h |
| search-quality-engineer | 1 | 10h |

**Total**: 30 tickets, 172 hours

### Critical Path

The following tickets are on the critical path and block multiple downstream tickets:

1. **LOCAL-1001** → LOCAL-1003 → LOCAL-1004, LOCAL-1005, LOCAL-1006, LOCAL-1007
2. **LOCAL-1003** → LOCAL-2001 → LOCAL-2002, LOCAL-2003 → LOCAL-2004 → LOCAL-2005, LOCAL-2006
3. **LOCAL-1007** → LOCAL-1008 → LOCAL-3001, LOCAL-3002, LOCAL-3007
4. **LOCAL-3001** → LOCAL-4001, LOCAL-4004 → LOCAL-4005, LOCAL-4006, LOCAL-4007

### Parallel Work Opportunities

These tickets can be executed in parallel:

**Week 1 Parallel**:
- LOCAL-1001 (Dockerfile) || LOCAL-1002 (PostgreSQL schema)

**Week 2 Parallel**:
- LOCAL-2001 (Provider enum) || LOCAL-2003 (Config validation) (both depend on same enum, but different files)

**Week 3 Parallel**:
- LOCAL-3002 (README) || LOCAL-3005 (Troubleshooting) || LOCAL-3006 (Config docs) (all documentation)

**Week 4 Parallel**:
- LOCAL-4001 (Benchmarks) || LOCAL-4003 (Resource profiling)
- LOCAL-4005 (ARM64) || LOCAL-4006 (Optimization) (after LOCAL-4004)

---

## Testing Strategy

### Test Tickets (9 of 30 tickets)

The project includes comprehensive testing aligned with MVP goals (confidence without full coverage):

1. **LOCAL-2005** - Integration tests for Ollama provider (validates core integration)
2. **LOCAL-2006** - Batch embedding tests (validates performance at scale)
3. **LOCAL-3001** - npx startup flow tests (validates user experience)
4. **LOCAL-4001** - Performance benchmarks (validates throughput targets)
5. **LOCAL-4002** - Quality comparison (validates search accuracy)
6. **LOCAL-4003** - Resource profiling (validates system requirements)
7. **LOCAL-4004** - E2E workflow tests (validates complete system)
8. **LOCAL-4005** - ARM64 testing (validates platform compatibility)
9. **LOCAL-4007** - Stress testing (validates scalability)

**Testing Philosophy**: Tests are embedded in implementation tickets where it makes sense. Separate test tickets exist only for critical validation that provides confidence for production deployment.

---

## Success Criteria

### Must Have (P0)
- ✅ Single command startup via npx
- ✅ Zero API key configuration
- ✅ Offline operation after model download
- ✅ Bundled PostgreSQL with pgvector
- ✅ Automated model provisioning
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
- 🔄 GPU acceleration support (documented, tested if hardware available)
- 🔄 Pre-baked image with model included (future optimization)
- 🔄 Grafana dashboard for monitoring (Phase 5)
- 🔄 Kubernetes manifests (Phase 5)

---

## Getting Started

### For Implementing Agents

1. **Review Planning Docs**: Read LOCAL_PLAN.md and LOCAL_ARCHITECTURE.md before starting
2. **Check Dependencies**: Ensure prerequisite tickets are completed
3. **Follow Ticket Workflow**: Implement → Test → Verify → Commit
4. **Update Ticket**: Mark checkboxes as you complete acceptance criteria
5. **Ask Questions**: If requirements are unclear, ask before implementing

### For Project Managers

1. **Track Progress**: Use ticket status to monitor phase completion
2. **Manage Dependencies**: Ensure critical path tickets are prioritized
3. **Coordinate Agents**: Maximize parallel work within phase constraints
4. **Review Milestones**: Validate phase milestones before proceeding to next phase
5. **Monitor Effort**: Track actual vs estimated effort for future planning

### Ticket Naming Convention

- **Format**: `LOCAL-XYYY_ticket-name.md`
- **X**: Phase number (1-4)
- **YYY**: Sequential number within phase (001-008)
- **Example**: LOCAL-1001 = Phase 1, first ticket

---

## Resources

### Documentation
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [Ollama Documentation](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [MCP Specification](https://modelcontextprotocol.io/)
- [npm Publishing Guide](https://docs.npmjs.com/creating-and-publishing-scoped-public-packages)

### Tools
- Docker Desktop (required)
- Rust 1.75+ (for Maproom binary)
- Node.js 18+ (for npm package)
- PostgreSQL 16 (via Docker)
- Ollama (via Docker)

### Community
- GitHub Issues: [Report bugs or request features]
- Documentation: [Contribute improvements]
- Pull Requests: [Submit enhancements]

---

## Timeline

- **Week 1**: Core Infrastructure (8 tickets, 47h)
- **Week 2**: Ollama Integration (6 tickets, 34h)
- **Week 3**: Configuration & UX (8 tickets, 32h)
- **Week 4**: Testing & Optimization (8 tickets, 57h)

**Total**: 4 weeks, 30 tickets, 172 hours (estimated 137-177h in planning docs)

---

**Last Updated**: 2025-10-26
**Status**: All tickets created, ready for implementation
**Next Step**: Begin Phase 1 with LOCAL-1001 and LOCAL-1002 in parallel
