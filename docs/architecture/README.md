# Architecture Documentation

Technical documentation for Maproom's system design and implementation.

## Overview Documents

- **[Architecture Overview](overview.md)** - High-level system design with diagrams
  - Component relationships
  - Data flow pipelines
  - Technology stack

- **[Sequence Diagrams](sequences.md)** - Detailed request/response flows
  - Search request flow
  - Indexing pipeline
  - Daemon lifecycle
  - Provider auto-detection

- **[Daemon Architecture](daemon.md)** - JSON-RPC daemon design
  - Communication protocol
  - Connection pooling
  - Lifecycle management
  - Error handling

## Database & Search

- **[Database Architecture](DATABASE_ARCHITECTURE.md)** - SQLite and PostgreSQL backends
  - Schema design
  - Connection configuration
  - Migration management

- **[Maproom Architecture](MAPROOM_ARCHITECTURE.md)** - Semantic search system
  - Indexing pipeline
  - Search algorithms
  - Embedding generation

- **[Search Evaluation](SEARCH_EVALUATION.md)** - Search quality metrics

## Features

- **[Branch-Aware Indexing](branch-aware-indexing.md)** - Worktree-scoped search
- **[Optimization Tracking](optimization-tracking-system.md)** - Query optimization

## Quick Links

| Topic | Document |
|-------|----------|
| System overview | [overview.md](overview.md) |
| How search works | [sequences.md](sequences.md) |
| Daemon details | [daemon.md](daemon.md) |
| Database setup | [DATABASE_ARCHITECTURE.md](DATABASE_ARCHITECTURE.md) |

## Related Documentation

- **[API Reference](../api/README.md)** - MCP tools documentation
- **[Troubleshooting](../troubleshooting/README.md)** - Common errors and debugging
- **[Performance Tuning](../guides/performance-tuning.md)** - Optimization guide
