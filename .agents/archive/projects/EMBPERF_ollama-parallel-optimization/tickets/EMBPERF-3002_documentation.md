# Ticket: EMBPERF-3002: Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - Documentation-only ticket.

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary
Document the optimized Ollama embedding configuration with performance results, recommended settings by hardware, and tuning guidance.

## Background
With implementation and benchmarking complete (EMBPERF-0001 through EMBPERF-3001), we need user-facing documentation:
1. What the optimization does
2. How to configure it
3. Recommended settings by hardware
4. Performance expectations

This implements Phase 3 (documentation portion) from `plan.md`.

## Acceptance Criteria
- [x] Configuration documentation added to project docs
- [x] Environment variables documented with defaults
- [x] Hardware-specific recommendations provided
- [x] Performance expectations documented (with caveats)
- [x] Troubleshooting section included
- [x] README.md updated with quick reference

## Technical Requirements

### Documentation Location
Add to: `docs/configuration/embedding-optimization.md` (or similar appropriate location)

### Required Sections

1. **Overview** - What the optimization does, expected improvements
2. **Configuration** - All environment variables with defaults
3. **Hardware Recommendations** - Settings by hardware tier
4. **Performance Results** - Actual benchmark data from EMBPERF-3001
5. **Troubleshooting** - Common issues and solutions

## Implementation Notes

### Document Structure

```markdown
# Ollama Embedding Optimization

## Overview
CrewChief Maproom supports optimized parallel embedding generation with Ollama,
achieving 10-20x throughput improvement on Apple Silicon hardware.

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MAPROOM_EMBEDDING_PARALLEL_ENABLED` | `true` | Enable parallel batch processing |
| `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE` | `50` | Number of texts per HTTP request |
| `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY` | `8` | Maximum concurrent requests |

### Quick Start

```bash
# Default settings work well for most hardware
# For M2 Max or better, increase concurrency:
export MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16
export MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=100
```

## Hardware Recommendations

| Hardware | Sub-Batch Size | Concurrency | Expected Throughput |
|----------|----------------|-------------|---------------------|
| M1/M2 (base) | 50 (default) | 4-8 | ~300-400 texts/sec |
| M2 Pro | 50 (default) | 8 (default) | ~500-700 texts/sec |
| M2 Max | 100 | 16 | ~800-1200 texts/sec |
| M2 Ultra | 100 | 24 | ~1000-1500 texts/sec |
| NVIDIA GPU | 100 | 16 | Varies by model |

## Performance Expectations

**Baseline**: ~50-100 texts/sec (sequential single-text requests)
**Optimized**: ~500-1500 texts/sec (depending on hardware)

### Improvement Factors

1. **Batch API** (~5-10x): Multiple texts per HTTP request reduces overhead
2. **Parallelism** (~2-4x additional): Concurrent requests saturate GPU

### Caveats

- Results vary by hardware, Ollama version, and model
- First-run may be slower due to model loading
- Very long texts may reduce throughput

## Troubleshooting

### Embeddings are slow

1. Check Ollama is running: `ollama list`
2. Verify model is loaded: `ollama run nomic-embed-text`
3. Try increasing concurrency: `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16`

### Out of memory

1. Reduce batch size: `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=25`
2. Reduce concurrency: `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=4`

### Timeouts

1. Check Ollama is responsive: `curl http://localhost:11434/api/embed -d '{"model":"nomic-embed-text","input":["test"]}'`
2. Increase timeout if needed (contact maintainers)

### Disabling Optimization

If issues persist, disable parallel processing:
```bash
export MAPROOM_EMBEDDING_PARALLEL_ENABLED=false
```
```

## Dependencies
- EMBPERF-3001 (benchmarks must be complete for actual performance data)
- Access to benchmark results

## Risk Assessment
- **Risk**: Performance claims don't match user experience
  - **Mitigation**: Include caveats, document hardware specs for benchmarks
- **Risk**: Documentation becomes outdated
  - **Mitigation**: Keep configuration reference close to code, automate if possible

## Files/Packages Affected
- New: `docs/configuration/embedding-optimization.md`
- Modified: `README.md` (add reference to optimization docs)
- Modified: `.agents/projects/EMBPERF_ollama-parallel-optimization/README.md` (update status)

## Deliverables
1. Main documentation file with all sections
2. README.md update with quick reference
3. Project README.md marked as complete
