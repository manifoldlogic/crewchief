# EMBPERF: Ollama Parallel Embedding Optimization

## Project Summary

Optimize the Ollama embedding pipeline to achieve **10-20x throughput improvement** on Apple Silicon hardware (M2 Max and similar).

**Status**: Planning Complete
**Phase**: Ready for ticket creation

---

## Problem Statement

The current Ollama embedding implementation achieves only a fraction of potential throughput:

| Issue | Current State | Impact |
|-------|---------------|--------|
| Single-text requests | 1 text per HTTP request | Massive overhead |
| Hardcoded concurrency | MAX_CONCURRENT = 10 | Ignores hardware capabilities |
| Unused parallel code | `embed_batch_parallel()` exists but not wired | Wasted development |
| GPU underutilization | ~15% on M2 Max | Hardware bottleneck |

### Performance Gap

| Configuration | Throughput |
|---------------|------------|
| **Current** | ~50-100 texts/sec |
| **Target** | 500-1500 texts/sec |
| **Improvement** | 10-20x |

---

## Proposed Solution

### Approach

**Phase 0: Baseline & Validation**
- Establish current performance baseline
- Verify Ollama batch API works as expected
- Test optimal batch sizes and concurrency levels

**Phase 1-2: Implementation**
1. **Batch API Usage** - Use Ollama's `"input": ["text1", "text2", ...]` array format
2. **Parallel Processing** - Concurrent batched requests using tokio semaphore
3. **Configuration Integration** - Wire `MAPROOM_EMBEDDING_PARALLEL_*` env vars to OllamaProvider

**Phase 3: Validation**
- Benchmark improvements against baseline
- Document optimal configurations

### Expected Results

| Hardware | Expected Throughput | Improvement |
|----------|---------------------|-------------|
| M1/M2 base | ~300-400 texts/sec | 4-6x |
| M2 Pro | ~500-700 texts/sec | 6-10x |
| M2 Max | ~800-1200 texts/sec | 10-15x |
| M2 Ultra | ~1000-1500 texts/sec | 12-20x |

---

## Planning Documents

| Document | Purpose |
|----------|---------|
| [analysis.md](planning/analysis.md) | Research findings, bottleneck analysis |
| [architecture.md](planning/architecture.md) | Technical design, component changes |
| [quality-strategy.md](planning/quality-strategy.md) | Testing approach, acceptance criteria |
| [security-review.md](planning/security-review.md) | Security assessment (minimal impact) |
| [plan.md](planning/plan.md) | Implementation phases and tickets |

---

## Relevant Agents

| Agent | Role |
|-------|------|
| **rust-indexer-engineer** | Core implementation (OllamaProvider changes) |
| **integration-tester** | Benchmark suite, integration tests |
| **technical-researcher** | Performance documentation |

---

## Quick Reference

### Environment Variables

```bash
# Enable parallel processing (default: true)
export MAPROOM_EMBEDDING_PARALLEL_ENABLED=true

# Texts per HTTP request (default: 50)
export MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=100

# Concurrent requests (default: 8)
export MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16
```

### For M2 Max Users

```bash
# Recommended settings for M2 Max
export MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=100
export MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16
```

---

## Dependencies

### Project Dependencies

| Dependency | Type | Impact |
|------------|------|--------|
| **VECSTORE-1000** | Soft | SQLite backend requires 768-dim support |

**VECSTORE Relationship**: EMBPERF optimizes Ollama embedding throughput, producing 768-dimensional embeddings. The VECSTORE project (specifically ticket VECSTORE-1000) adds 768-dim support to SQLite. Without VECSTORE-1000:

- **PostgreSQL users**: EMBPERF works immediately (PostgreSQL supports 768-dim)
- **SQLite users**: Cannot benefit from EMBPERF until VECSTORE-1000 completes

**Recommendation**: VECSTORE-1000 should complete before EMBPERF for the zero-config experience (SQLite + Ollama) to work.

---

## Research Sources

- [Ollama GitHub Issue #8778](https://github.com/ollama/ollama/issues/8778) - Parallel embeddings limitation
- [How Ollama Handles Parallel Requests](https://www.glukhov.org/post/2025/05/how-ollama-handles-parallel-requests/) - Internal batching behavior
- [CollabnIX Ollama Guide 2025](https://collabnix.com/ollama-embedded-models-the-complete-technical-guide-for-2025-enterprise-deployment/) - M2 Max benchmarks
- [Apple MPS Ollama Optimization](https://markaicode.com/apple-metal-performance-shaders-m1-m2-ollama-optimization/) - Metal acceleration
