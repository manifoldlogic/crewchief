# Ticket: LOCAL-4001: Benchmark embedding generation performance

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Establish performance baselines for embedding generation with Ollama's nomic-embed-text model and compare against OpenAI embeddings to validate that the local solution meets the performance targets defined in the LOCAL project plan.

## Background
As part of Phase 4 (Testing & Optimization) of the LOCAL project, we need to validate that the Ollama-based embedding generation meets the required performance targets before considering the solution production-ready. This benchmark suite will:

1. Establish baseline performance metrics for the local embedding stack
2. Identify bottlenecks in CPU vs GPU performance
3. Compare local performance against OpenAI to validate the "2x slower is acceptable" threshold
4. Provide repeatable benchmarks for future optimization work
5. Guide decisions about hardware requirements for production deployments

The performance targets from LOCAL_ANALYSIS.md are:
- **Throughput**: 500-1000 chunks/minute on CPU, 2000-5000 chunks/minute on GPU (optional)
- **Latency**: <100ms per chunk for small batches
- **Search latency**: <100ms (p95) for hybrid search

## Acceptance Criteria
- [ ] Benchmark suite created using Criterion.rs or similar framework
- [ ] Benchmark suite is repeatable and can be executed via `cargo bench`
- [ ] Performance metrics collected for all batch size scenarios (1, 10, 50, 100+ chunks)
- [ ] CPU-only performance achieves 500+ chunks/min throughput
- [ ] Latency p95 < 200ms for batch processing
- [ ] Memory usage stays < 4GB during batching operations
- [ ] Results documented in `/docs/performance/LOCAL-4001-embedding-benchmarks.md` report
- [ ] Comparison to OpenAI performed (if API key available in environment)
- [ ] GPU benchmarks included (if hardware available, otherwise document N/A)

## Technical Requirements

### Benchmark Scenarios to Implement

1. **Single Embedding Generation**
   - Measure time for 1 chunk embedding
   - Test cold start vs warm start (with pre-warmed Ollama model)
   - Use typical code snippet size (~200 tokens)
   - Record baseline latency

2. **Small Batch (10 chunks)**
   - Measure throughput (chunks/sec)
   - Measure latency distribution: p50, p95, p99
   - Track memory usage during processing
   - Simulate small file indexing

3. **Medium Batch (50 chunks)**
   - Realistic file indexing scenario
   - Sustained throughput measurement over time
   - CPU utilization tracking
   - Memory stability check

4. **Large Batch (100+ chunks)**
   - Stress test for repository indexing
   - Memory stability over extended operations
   - Throughput degradation analysis
   - Identify concurrency bottlenecks

5. **Comparison Tests (Optional)**
   - Same workload with OpenAI embeddings (if `OPENAI_API_KEY` available)
   - Document performance delta
   - Validate within acceptable range (2x slower is acceptable)

### Metrics to Collect

For each benchmark scenario, collect:
- **Throughput**: Requests per second, chunks per minute
- **Latency**: p50, p95, p99 percentiles
- **Resource Usage**:
  - CPU utilization (%)
  - Memory usage (MB/GB)
  - GPU utilization (% if available)
  - Network I/O (should be minimal - local only)
- **Stability**: Memory leaks, degradation over time

### Technology Stack
- **Framework**: Criterion.rs for Rust benchmarking
- **Monitoring**: System resource tracking during benchmarks
- **Output**: JSON + markdown reports for analysis

## Implementation Notes

### Approach

1. **Setup Criterion.rs benchmarks** in `crates/maproom/benches/embedding_performance.rs`
   - Create benchmark groups for each scenario
   - Use realistic code chunk samples from the test fixtures
   - Parameterize batch sizes

2. **Mock vs Live Testing**
   - For repeatable benchmarks: mock Ollama responses with recorded latencies
   - For real-world testing: connect to live Ollama service
   - Document which mode is used for each benchmark

3. **Resource Monitoring**
   - Use `sysinfo` crate or similar for CPU/memory tracking
   - Integrate monitoring into benchmark harness
   - Output metrics alongside Criterion results

4. **Comparison Methodology**
   - If OpenAI API key available, run identical workloads
   - Use same batch sizes and chunking strategy
   - Fair comparison: exclude network latency for OpenAI (document separately)

5. **Output Format**
   - Criterion generates HTML reports automatically
   - Create markdown summary in `/docs/performance/`
   - Include charts/graphs for visual analysis

### Considerations

- **Ollama Warmup**: First request may be slower due to model loading - document cold vs warm start
- **Concurrency**: Test different concurrency levels (sequential, parallel batching)
- **Hardware Variance**: Document the test hardware specs (CPU model, RAM, GPU if present)
- **Reproducibility**: Pin Ollama model version, document system state
- **CI Integration**: Consider lightweight smoke tests for CI, full benchmarks manual/scheduled

### Edge Cases

- What if Ollama service is unavailable? (Graceful failure with clear message)
- What if GPU is not available? (Fall back to CPU benchmarks only)
- What if OpenAI key is not available? (Skip comparison, document as N/A)

## Dependencies

### Prerequisite Tickets
- **LOCAL-3001**: Test npx startup flow (stack running end-to-end) - MUST be complete
- All Phase 2 tickets (Ollama integration) must be complete
- All Phase 3 tickets (packaging and testing) must be complete

### External Dependencies
- Ollama service running and accessible
- `nomic-embed-text` model pulled and available
- Rust `criterion` crate added to dev-dependencies
- Optional: OpenAI API key for comparison benchmarks
- Optional: GPU-enabled hardware for GPU benchmarks

## Risk Assessment

- **Risk**: Benchmarks may be flaky due to system load variance
  - **Mitigation**: Run multiple iterations, report confidence intervals, document test conditions

- **Risk**: Performance may not meet targets on CPU-only systems
  - **Mitigation**: Document actual performance, identify optimization opportunities, consider GPU recommendation

- **Risk**: Ollama model loading time may skew results
  - **Mitigation**: Separate cold-start vs warm-start benchmarks, document model preloading strategy

- **Risk**: Comparison to OpenAI may be unfair due to network latency
  - **Mitigation**: Measure and exclude network latency separately, focus on processing time

- **Risk**: Hardware variance makes benchmarks non-reproducible
  - **Mitigation**: Document test hardware specifications, use relative metrics (speedup ratios), establish baseline on reference hardware

## Files/Packages Affected

### New Files
- `crates/maproom/benches/embedding_performance.rs` - Criterion benchmark suite
- `docs/performance/LOCAL-4001-embedding-benchmarks.md` - Results report
- `docs/performance/hardware-specs.md` - Test hardware specifications

### Modified Files
- `crates/maproom/Cargo.toml` - Add `criterion` to dev-dependencies
- `.gitignore` - Exclude Criterion output directories if needed

### Test Fixtures
- May need to create or reuse code chunk samples in `crates/maproom/tests/fixtures/`
