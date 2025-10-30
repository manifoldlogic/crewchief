# LOCAL: Containerized Maproom with Local LLM Embeddings

**Project Slug**: LOCAL
**Status**: Ready for Implementation
**Created**: 2025-10-26

## Overview

This project implements a fully containerized version of Maproom MCP with a local LLM (Ollama + nomic-embed-text) for embedding generation, distributed as an npm package for zero-configuration deployment. No API keys, no manual setup, no repository clone required.

## Project Goals

1. **Zero Configuration**: Single npx command in .mcp.json
2. **Offline Operation**: Works completely offline after initial setup
3. **No API Keys**: Local LLM eliminates OpenAI dependency
4. **Bundled Database**: PostgreSQL with pgvector included
5. **npm Distribution**: No repository clone required
6. **Portable**: Run anywhere Docker + npm runs

## Documents

### 1. [LOCAL_ANALYSIS.md](./LOCAL_ANALYSIS.md)
**Purpose**: Problem space analysis and solution evaluation

**Contents**:
- Current state vs desired state
- Local LLM options comparison (Ollama, Sentence Transformers, EmbeddingGemma)
- Recommended solution: Ollama + nomic-embed-text
- Performance metrics and resource requirements
- Risk analysis and success criteria

**Key Finding**: Ollama with nomic-embed-text provides optimal balance of performance, ease of deployment, and resource efficiency.

### 2. [LOCAL_ARCHITECTURE.md](./LOCAL_ARCHITECTURE.md)
**Purpose**: Technical design and implementation details

**Contents**:
- System architecture diagram
- Component specifications (Maproom MCP, Ollama, PostgreSQL)
- Docker Compose configuration
- Code changes required in Rust embedding client
- Database schema with pgvector
- Testing strategy
- Security considerations

**Key Decisions**:
- Multi-service Docker Compose (not monolithic)
- Multi-stage Dockerfile for minimal image size
- Init container pattern for automatic model provisioning
- 768-dimension embeddings (vs OpenAI's 1536)

### 3. [LOCAL_PLAN.md](./LOCAL_PLAN.md)
**Purpose**: Implementation roadmap with phases and agent assignments

**Contents**:
- 4-week phased implementation plan
- 24 discrete tasks with effort estimates
- Agent assignments and responsibilities
- Risk mitigation strategies
- Success criteria and deliverables checklist

**Phases**:
1. **Week 1**: Core Infrastructure (Docker, PostgreSQL, Compose)
2. **Week 2**: Ollama Integration (Provider support, API client)
3. **Week 3**: Configuration & UX (Zero-config setup, docs)
4. **Week 4**: Testing & Optimization (Performance, quality, multi-platform)

## Quick Reference

### Solution Architecture

```
User's .mcp.json:
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
         │
         ▼
    npm Package
 (@crewchief/maproom-mcp)
    CLI Wrapper
         │
         ▼
┌─────────────────────────────────────────┐
│      Docker Compose Stack               │
│                                         │
│  Maproom MCP ──► Ollama ──► PostgreSQL │
│                   │                     │
│              nomic-embed-text           │
│                (768 dim)                │
└─────────────────────────────────────────┘
```

### Resource Requirements

- **CPU**: 4 cores (2 minimum)
- **RAM**: 8GB (4GB minimum)
- **Disk**: 5GB (2GB images + 3GB data)
- **OS**: Linux, macOS, Windows (WSL2)

### Performance Targets

- Embedding generation: 500-1000 chunks/min (CPU)
- Search latency: <100ms (p95)
- Index throughput: 100+ files/min

### User Experience Goal

```json
// This should be the ONLY configuration users need:
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

**First Run**: ~2-3 minutes (downloads npm package, Docker images, model)
**Subsequent Runs**: ~10-20 seconds (everything cached)

## Agent Requirements

### Existing Agents Used
- rust-indexer-engineer
- embeddings-engineer
- database-engineer
- integration-tester
- performance-engineer
- search-quality-engineer
- monitoring-observability-engineer
- technical-researcher

### New Agent Created
- **docker-engineer** (definition at `.agents/specialized-agents/docker-engineer.md`)
  - Specializes in Docker, Docker Compose, containerization
  - Responsible for Dockerfiles, orchestration, deployment scripts
  - Assigned 6 tasks in LOCAL project

## Key Technical Decisions

1. **Ollama over Sentence Transformers**
   - Better developer experience
   - Automatic model management
   - OpenAI-compatible API
   - Active community support

2. **nomic-embed-text over larger models**
   - 768 dimensions (50% smaller than OpenAI)
   - Fast CPU inference
   - Excellent code/text retrieval performance
   - Small download size (~200MB)

3. **npm Wrapper + Docker Compose**
   - Simple UX (npx in .mcp.json)
   - No repository clone required
   - Modular, maintainable containers
   - Follows Docker best practices
   - Clear separation of concerns

4. **Multi-stage builds**
   - Minimal final image size
   - Faster deployments
   - Security (minimal attack surface)

## Success Metrics

### Must Have (P0)
- ✅ Single command startup
- ✅ Zero API key configuration
- ✅ Offline operation after model download
- ✅ Bundled PostgreSQL + pgvector
- ✅ Automated model provisioning
- ✅ Health checks for all services

### Should Have (P1)
- ✅ Performance within 2x of OpenAI
- ✅ Search quality within 10% of OpenAI
- ✅ Resource usage <6GB RAM
- ✅ Docker image <2GB
- ✅ Multi-platform support (AMD64, ARM64)

## Next Steps

1. **Review Documents**
   - Read LOCAL_ANALYSIS for problem understanding
   - Review LOCAL_ARCHITECTURE for technical details
   - Study LOCAL_PLAN for implementation roadmap

2. **Set Up Environment**
   - Ensure Docker and Docker Compose installed
   - Verify system meets minimum requirements
   - Create project tracking board

3. **Begin Phase 1**
   - Assign docker-engineer agent
   - Start with LOCAL-1001 (Maproom Dockerfile)
   - Coordinate with database-engineer for schema

4. **Track Progress**
   - Use task IDs from LOCAL_PLAN
   - Update milestone checklist
   - Report weekly progress

## Legacy Package Migration

The existing `maproom-mcp` npm package (manual PostgreSQL + API keys setup) will be deprecated:

- **Timeline**: 6-month migration period
- **Strategy**: Legacy package forwards to `@crewchief/maproom-mcp` with deprecation notice
- **Impact**: Existing users continue working, see upgrade prompt
- **New Package**: `@crewchief/maproom-mcp` (containerized, zero-config)

**Migration Message**:
```
⚠️  DEPRECATION: maproom-mcp → @crewchief/maproom-mcp

New version includes:
  • 🐳 Fully containerized
  • 🚀 Local LLM (no API keys)
  • 📦 Bundled PostgreSQL
  • 🔌 Zero-config setup

Update .mcp.json to:
  "command": "npx",
  "args": ["-y", "@crewchief/maproom-mcp"]
```

## Project Timeline

- **Week 1**: Infrastructure ready (Docker, DB, Compose, npm wrapper)
- **Week 2**: Ollama integration working
- **Week 3**: Zero-config UX complete, npm published
- **Week 4**: Production-ready with benchmarks

**Total**: 3-4 weeks, 137-177 agent hours (updated from 120-160 to include npm wrapper)

## Files Created

1. `crewchief_context/maproom/LOCAL/LOCAL_ANALYSIS.md` - Problem analysis
2. `crewchief_context/maproom/LOCAL/LOCAL_ARCHITECTURE.md` - Technical design
3. `crewchief_context/maproom/LOCAL/LOCAL_PLAN.md` - Implementation roadmap
4. `.agents/specialized-agents/docker-engineer.md` - New agent definition
5. `crewchief_context/maproom/LOCAL/README.md` - This file

## Questions or Issues?

Refer to:
- **LOCAL_ANALYSIS.md** for "Why this approach?"
- **LOCAL_ARCHITECTURE.md** for "How does it work?"
- **LOCAL_PLAN.md** for "What needs to be done?"

---

**Ready for Review**: All planning documents are complete and ready for stakeholder review before implementation begins.
