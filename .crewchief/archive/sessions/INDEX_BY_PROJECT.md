# Work Tickets Index by Project

Quick reference for navigating tickets by project.

**Last Updated**: 2025-10-24
**Total Tickets**: 91 (HYBRID_SEARCH: 22, MCP_CORE: 6, CONTEXT_ASM: 14, INC_INDEX: 8, LANG_PARSE: 20, PERF_OPT: 10, MD_ENHANCE: 8)
**Projects**: 7

---

## HYBRID_SEARCH (22 tickets)
Hybrid retrieval system combining FTS, vector similarity, and graph signals

### Phase 1: Embedding Infrastructure (Week 1) - 4 tickets
- [HYBRID_SEARCH-1001](HYBRID_SEARCH-1001_embedding-service-setup.md) - Embedding service setup
- [HYBRID_SEARCH-1002](HYBRID_SEARCH-1002_database-vector-preparation.md) - Database vector preparation
- [HYBRID_SEARCH-1003](HYBRID_SEARCH-1003_embedding-generation-pipeline.md) - Embedding generation pipeline
- [HYBRID_SEARCH-1901](HYBRID_SEARCH-1901_test-embedding-infrastructure.md) - Test embedding infrastructure

### Phase 2: Search Pipeline (Week 2) - 4 tickets
- [HYBRID_SEARCH-2001](HYBRID_SEARCH-2001_query-processing-pipeline.md) - Query processing pipeline
- [HYBRID_SEARCH-2002](HYBRID_SEARCH-2002_parallel-search-execution.md) - Parallel search execution
- [HYBRID_SEARCH-2003](HYBRID_SEARCH-2003_initial-search-integration.md) - Initial search integration
- [HYBRID_SEARCH-2901](HYBRID_SEARCH-2901_test-search-pipeline.md) - Test search pipeline

### Phase 3: Score Fusion (Week 3) - 4 tickets
- [HYBRID_SEARCH-3001](HYBRID_SEARCH-3001_reciprocal-rank-fusion.md) - Reciprocal rank fusion
- [HYBRID_SEARCH-3002](HYBRID_SEARCH-3002_weighted-score-combination.md) - Weighted score combination
- [HYBRID_SEARCH-3003](HYBRID_SEARCH-3003_signal-integration.md) - Signal integration
- [HYBRID_SEARCH-3901](HYBRID_SEARCH-3901_test-score-fusion.md) - Test score fusion

### Phase 4: Performance Optimization (Week 4) - 4 tickets
- [HYBRID_SEARCH-4001](HYBRID_SEARCH-4001_query-optimization.md) - Query optimization
- [HYBRID_SEARCH-4002](HYBRID_SEARCH-4002_index-tuning.md) - Index tuning
- [HYBRID_SEARCH-4003](HYBRID_SEARCH-4003_caching-strategy.md) - Caching strategy
- [HYBRID_SEARCH-4901](HYBRID_SEARCH-4901_test-performance-optimization.md) - Test performance optimization

### Phase 5: Quality Validation (Week 5) - 2 tickets
- [HYBRID_SEARCH-5001](HYBRID_SEARCH-5001_golden-test-set.md) - Golden test set
- [HYBRID_SEARCH-5002](HYBRID_SEARCH-5002_ab-testing-framework.md) - A/B testing framework

### Phase 6: Production Rollout (Week 6) - 4 tickets
- [HYBRID_SEARCH-6001](HYBRID_SEARCH-6001_mcp-integration-update.md) - MCP integration update
- [HYBRID_SEARCH-6002](HYBRID_SEARCH-6002_configuration-management.md) - Configuration management
- [HYBRID_SEARCH-6003](HYBRID_SEARCH-6003_monitoring-alerting.md) - Monitoring and alerting
- [HYBRID_SEARCH-6901](HYBRID_SEARCH-6901_test-production-readiness.md) - Test production readiness

---

## MCP_CORE (6 tickets)
MCP server implementation with 5 tools

### Phase 1: Core Tools (Week 1) - 4 tickets
- [MCP_CORE-1001](MCP_CORE-1001_context-tool-implementation.md) - Context tool implementation
- [MCP_CORE-1002](MCP_CORE-1002_open-tool-enhancement.md) - Open tool enhancement
- [MCP_CORE-1003](MCP_CORE-1003_upsert-tool-implementation.md) - Upsert tool implementation
- [MCP_CORE-1004](MCP_CORE-1004_explain-tool-implementation.md) - Explain tool implementation

### Phase 2: Integration & Testing (Weeks 2-3) - 2 tickets
- [MCP_CORE-2001](MCP_CORE-2001_end-to-end-testing.md) - End-to-end testing
- [MCP_CORE-2002](MCP_CORE-2002_client-integration.md) - Client integration

---

## CONTEXT_ASM (14 tickets)
Budget-aware context assembly engine

### Phase 1: Core Assembly (Weeks 1-2) - 4 tickets
- [CONTEXT_ASM-1001](CONTEXT_ASM-1001_basic-assembly-pipeline.md) - Basic assembly pipeline
- [CONTEXT_ASM-1002](CONTEXT_ASM-1002_relationship-queries.md) - Relationship queries
- [CONTEXT_ASM-1003](CONTEXT_ASM-1003_budget-management.md) - Budget management
- [CONTEXT_ASM-1004](CONTEXT_ASM-1004_content-formatting.md) - Content formatting

### Phase 2: Intelligence Layer (Weeks 3-4) - 4 tickets
- [CONTEXT_ASM-2001](CONTEXT_ASM-2001_importance-scoring.md) - Importance scoring
- [CONTEXT_ASM-2002](CONTEXT_ASM-2002_heuristics-implementation.md) - Heuristics implementation
- [CONTEXT_ASM-2003](CONTEXT_ASM-2003_react-specific-logic.md) - React-specific logic
- [CONTEXT_ASM-2004](CONTEXT_ASM-2004_strategy-framework.md) - Strategy framework

### Phase 3: Performance (Week 5) - 3 tickets
- [CONTEXT_ASM-3001](CONTEXT_ASM-3001_query-optimization.md) - Query optimization
- [CONTEXT_ASM-3002](CONTEXT_ASM-3002_caching-system.md) - Caching system
- [CONTEXT_ASM-3003](CONTEXT_ASM-3003_parallel-processing.md) - Parallel processing

### Phase 4: Integration (Week 6) - 3 tickets
- [CONTEXT_ASM-4001](CONTEXT_ASM-4001_mcp-tool-implementation.md) - MCP tool implementation
- [CONTEXT_ASM-4002](CONTEXT_ASM-4002_testing-suite.md) - Testing suite
- [CONTEXT_ASM-4003](CONTEXT_ASM-4003_documentation.md) - Documentation

---

## INC_INDEX (8 tickets)
Incremental indexing with file watching

### Phase 1: Change Detection (Week 1) - 2 tickets
- [INC_INDEX-1001](INC_INDEX-1001_file-hashing-system.md) - File hashing system
- [INC_INDEX-1002](INC_INDEX-1002_change-detection-api.md) - Change detection API

### Phase 2: File Watching (Week 2) - 2 tickets
- [INC_INDEX-2001](INC_INDEX-2001_watcher-implementation.md) - File watcher implementation
- [INC_INDEX-2002](INC_INDEX-2002_multi-worktree-support.md) - Multi-worktree support

### Phase 3: Update Processing (Week 3) - 2 tickets
- [INC_INDEX-3001](INC_INDEX-3001_update-queue.md) - Update processing queue
- [INC_INDEX-3002](INC_INDEX-3002_incremental-processing.md) - Incremental processing logic

### Phase 4: Integration (Week 4) - 2 tickets
- [INC_INDEX-4001](INC_INDEX-4001_watch-command.md) - Watch command implementation
- [INC_INDEX-4002](INC_INDEX-4002_testing-validation.md) - Testing and validation

---

## LANG_PARSE (20 tickets)
Multi-language parser support (Python, Rust, Go)

### Phase 1: Python Support (Weeks 1-2) - 8 tickets
- [LANG_PARSE-1001](LANG_PARSE-1001_python-grammar-setup.md) - Python grammar setup
- [LANG_PARSE-1002](LANG_PARSE-1002_python-symbol-extraction.md) - Python symbol extraction
- [LANG_PARSE-1003](LANG_PARSE-1003_python-import-extraction.md) - Python import extraction
- [LANG_PARSE-1004](LANG_PARSE-1004_python-docstring-parsing.md) - Python docstring parsing
- [LANG_PARSE-1005](LANG_PARSE-1005_python-integration.md) - Python integration
- [LANG_PARSE-1006](LANG_PARSE-1006_python-testing-suite.md) - Python testing suite
- [LANG_PARSE-1007](LANG_PARSE-1007_python-database-integration.md) - Python database integration
- [LANG_PARSE-1008](LANG_PARSE-1008_python-production-validation.md) - Python production validation

### Phase 2: Rust Support (Weeks 3-4) - 4 tickets
- [LANG_PARSE-2001](LANG_PARSE-2001_rust-grammar-setup.md) - Rust grammar setup
- [LANG_PARSE-2002](LANG_PARSE-2002_rust-symbol-extraction.md) - Rust symbol extraction
- [LANG_PARSE-2003](LANG_PARSE-2003_rust-documentation-extraction.md) - Rust documentation extraction
- [LANG_PARSE-2004](LANG_PARSE-2004_rust-integration.md) - Rust integration

### Phase 3: Go Support (Weeks 5-6) - 4 tickets
- [LANG_PARSE-3001](LANG_PARSE-3001_go-grammar-setup.md) - Go grammar setup
- [LANG_PARSE-3002](LANG_PARSE-3002_go-symbol-extraction.md) - Go symbol extraction
- [LANG_PARSE-3003](LANG_PARSE-3003_go-conventions.md) - Go conventions
- [LANG_PARSE-3004](LANG_PARSE-3004_go-integration-optimization.md) - Go integration and optimization

### Phase 4: Production Rollout (Week 7) - 4 tickets
- [LANG_PARSE-4001](LANG_PARSE-4001_large-scale-testing.md) - Large-scale testing
- [LANG_PARSE-4002](LANG_PARSE-4002_search-quality-validation.md) - Search quality validation
- [LANG_PARSE-4003](LANG_PARSE-4003_production-migration.md) - Production migration
- [LANG_PARSE-4004](LANG_PARSE-4004_production-rollout.md) - Production rollout and monitoring

---

## PERF_OPT (10 tickets)
Performance optimization across all components

### Phase 1: Benchmarking (Week 1) - 2 tickets
- [PERF_OPT-1001](PERF_OPT-1001_benchmark-suite.md) - Create benchmark suite
- [PERF_OPT-1002](PERF_OPT-1002_identify-bottlenecks.md) - Identify bottlenecks

### Phase 2: Database Optimization (Week 2) - 2 tickets
- [PERF_OPT-2001](PERF_OPT-2001_index-optimization.md) - Index optimization
- [PERF_OPT-2002](PERF_OPT-2002_query-tuning.md) - Query tuning

### Phase 3: Parallelization (Week 3) - 2 tickets
- [PERF_OPT-3001](PERF_OPT-3001_parallel-indexing.md) - Parallel indexing
- [PERF_OPT-3002](PERF_OPT-3002_concurrent-operations.md) - Concurrent operations

### Phase 4: Caching Implementation (Week 4) - 2 tickets
- [PERF_OPT-4001](PERF_OPT-4001_cache-systems.md) - Cache systems
- [PERF_OPT-4002](PERF_OPT-4002_cache-management.md) - Cache management

### Phase 5: Final Optimization (Week 5) - 2 tickets
- [PERF_OPT-5001](PERF_OPT-5001_memory-optimization.md) - Memory optimization
- [PERF_OPT-5002](PERF_OPT-5002_fine-tuning.md) - Fine tuning

---

## MD_ENHANCE (8 tickets)
Enhanced markdown parsing with tree-sitter

### Phase 1: Tree-Sitter Integration (Week 1) - 2 tickets
- [MD_ENHANCE-1001](MD_ENHANCE-1001_parser-setup.md) - Parser setup
- [MD_ENHANCE-1002](MD_ENHANCE-1002_ast-walking.md) - AST walking

### Phase 2: Hierarchy Tracking (Week 2) - 2 tickets
- [MD_ENHANCE-2001](MD_ENHANCE-2001_parent-tracking.md) - Parent tracking
- [MD_ENHANCE-2002](MD_ENHANCE-2002_section-boundaries.md) - Section boundaries

### Phase 3: Enhanced Extraction (Week 3) - 2 tickets
- [MD_ENHANCE-3001](MD_ENHANCE-3001_code-block-processing.md) - Code block processing
- [MD_ENHANCE-3002](MD_ENHANCE-3002_link-resolution.md) - Link resolution

### Phase 4: Migration & Testing (Week 4) - 2 tickets
- [MD_ENHANCE-4001](MD_ENHANCE-4001_migration-script.md) - Migration script
- [MD_ENHANCE-4002](MD_ENHANCE-4002_quality-testing.md) - Quality testing

---

**Total**: 91 tickets across 7 projects
