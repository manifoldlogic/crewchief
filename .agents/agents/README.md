# Agent Definitions

This directory contains definitions for specialized AI agents that work on different aspects of the CrewChief project. Each agent has specific capabilities, tools, and expertise areas.

## Agent Categories

### 🗄️ Data & Storage (5 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [database-engineer](./database-engineer.md) | PostgreSQL, pgvector, hybrid search | SQL optimization, indexing, migrations |
| [vector-database-engineer](./vector-database-engineer.md) | Vector search, pgvector tuning | ANN algorithms, HNSW/IVFFlat, recall optimization |
| [embeddings-engineer](./embeddings-engineer.md) | Embedding generation, providers | OpenAI, Ollama, batch processing, caching |
| [google-cloud-integration-engineer](./google-cloud-integration-engineer.md) | Google Vertex AI, GCP IAM | Service accounts, API integration |
| [caching-engineer](./caching-engineer.md) | Multi-layer caches, invalidation | Redis, LRU, distributed caching |

### 🔍 Code Analysis & Parsing (3 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [parser-engineer](./parser-engineer.md) | Language support, tree-sitter | Grammar integration, symbol extraction |
| [graph-analysis-engineer](./graph-analysis-engineer.md) | Code relationships, AST parsing | Import/export extraction, call graphs |
| [graph-algorithms-engineer](./graph-algorithms-engineer.md) | Graph traversal, algorithms | BFS/DFS, PageRank, recursive queries |

### 🧪 Testing & Quality (6 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [contract-test-engineer](./contract-test-engineer.md) | API contracts, schema validation | Request/response testing |
| [concurrency-test-engineer](./concurrency-test-engineer.md) | Race conditions, thread safety | Stress testing, lock verification |
| [integration-tester](./integration-tester.md) | End-to-end workflows | Multi-component testing |
| [property-test-engineer](./property-test-engineer.md) | Invariant verification | QuickCheck, generators |
| [snapshot-test-engineer](./snapshot-test-engineer.md) | Parser outputs, golden files | Regression prevention |
| [search-quality-engineer](./search-quality-engineer.md) | Search relevance, benchmarks | Precision, recall, NDCG, MRR |

### ⚡ Performance (2 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [performance-engineer](./performance-engineer.md) | Profiling, optimization | Latency, throughput, benchmarking |
| [performance-regression-test-engineer](./performance-regression-test-engineer.md) | Performance baselines | Regression detection, automated benchmarks |

### 🛠️ Infrastructure & Deployment (3 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [docker-engineer](./docker-engineer.md) | Containerization, multi-stage builds | Docker Compose, health checks |
| [migration-safety-specialist](./migration-safety-specialist.md) | Database migrations, rollbacks | Zero-downtime, production safety |
| [monitoring-observability-engineer](./monitoring-observability-engineer.md) | Metrics, logging, tracing | Prometheus, Grafana, OpenTelemetry |

### 🏗️ Architecture & Design (2 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [provider-abstraction-architect](./provider-abstraction-architect.md) | Trait design, plugin systems | Factory patterns, object safety |
| [mcp-context-engineer](./mcp-context-engineer.md) | Context assembly, token budgets | Code bundling, relationship traversal |

### 🔧 Implementation Specialists (5 agents)
| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [rust-indexer-engineer](./rust-indexer-engineer.md) | Maproom indexer, tree-sitter | Parallel processing, CLI commands |
| [mcp-tools-engineer](./mcp-tools-engineer.md) | MCP server, tool handlers | Zod schemas, stdio/SSE transport |
| [documentation-engineer](./documentation-engineer.md) | Technical documentation | API docs, guides, tutorials |
| [migration-engineer](./migration-engineer.md) | Data migrations, transformations | ETL, validation, rollback |

### 🔬 Research & Exploration (1 agent)

| Agent | Primary Focus | Key Skills |
|-------|--------------|------------|
| [technical-researcher](./technical-researcher.md) | Technology evaluation, feasibility | Multi-source investigation, synthesis |

## Agent Recommendations

- [AGENT_RECOMMENDATIONS.md](./AGENT_RECOMMENDATIONS.md) - Recommendations for missing/needed agents
- [TESTING_AGENT_RECOMMENDATIONS.md](./TESTING_AGENT_RECOMMENDATIONS.md) - Testing-focused agent suggestions
- [_KEEP_THESE.md](./_KEEP_THESE.md) - Notes on agent organization

## Quick Reference

### Finding the Right Agent

**For database work:**
- Schema changes, migrations → `migration-safety-specialist`
- Query optimization → `database-engineer`
- Vector search tuning → `vector-database-engineer`
- Embedding generation → `embeddings-engineer`

**For testing:**
- API contracts → `contract-test-engineer`
- Concurrency issues → `concurrency-test-engineer`
- End-to-end flows → `integration-tester`
- Search quality → `search-quality-engineer`
- Parser outputs → `snapshot-test-engineer`

**For implementation:**
- MCP tools → `mcp-tools-engineer`
- Rust indexer → `rust-indexer-engineer`
- New language support → `parser-engineer`
- Code relationships → `graph-analysis-engineer`

**For infrastructure:**
- Docker setup → `docker-engineer`
- Monitoring → `monitoring-observability-engineer`
- Caching → `caching-engineer`

## Agent Capabilities Matrix

| Capability | Agents |
|------------|--------|
| PostgreSQL | database-engineer, migration-safety-specialist |
| pgvector | database-engineer, vector-database-engineer |
| Embeddings | embeddings-engineer, google-cloud-integration-engineer |
| Tree-sitter | parser-engineer, rust-indexer-engineer |
| Testing | contract-test-engineer, concurrency-test-engineer, integration-tester, property-test-engineer, snapshot-test-engineer |
| Performance | performance-engineer, performance-regression-test-engineer |
| MCP | mcp-tools-engineer, mcp-context-engineer |
| Rust | rust-indexer-engineer, provider-abstraction-architect |
| Docker | docker-engineer |
| Monitoring | monitoring-observability-engineer |
| Architecture | provider-abstraction-architect, mcp-context-engineer |

## Using Agent Definitions

### For AI Agents

When working on a ticket, find relevant agents by:

```bash
# Search for agents by keyword
grep -l "database" .agents/agents/*.md
grep -l "testing" .agents/agents/*.md

# Read agent definition
cat .agents/agents/database-engineer.md
```

### For Ticket Assignment

Tickets should specify which agents are suitable:

```markdown
## Agents
- database-engineer
- migration-safety-specialist
```

### Agent Workflow

Typical agent workflow for a ticket:

1. **Implementation Agent** completes the work
2. **Test Runner** executes tests
3. **Verify Agent** checks acceptance criteria
4. **Commit Agent** creates commit

## Contributing New Agent Definitions

When creating a new agent definition:

1. Use the template structure from existing agents
2. Clearly define:
   - Primary responsibilities
   - Tools and technologies
   - When to use this agent (with examples)
   - When NOT to use this agent
3. Add entry to this README in appropriate category
4. Update capability matrix

---

Total Agents: 28
