# Analysis: Ollama Parallel Embedding Optimization

## Problem Definition

The current Ollama embedding implementation achieves a fraction of the potential throughput on Apple Silicon hardware. Users with M2 Max machines (and similar) are experiencing embedding generation speeds far below what the hardware can support.

### Current State

**OllamaProvider** (`crates/maproom/src/embedding/ollama.rs`):
- Hardcoded `MAX_CONCURRENT_REQUESTS = 10`
- Sends **one text per HTTP request** (ignoring Ollama's batch API)
- Does not use the configurable `ParallelConfig` from `config.rs`

**OpenAIClient** (`crates/maproom/src/embedding/client.rs`):
- Has `embed_batch_parallel()` with configurable concurrency
- Uses sub-batching (default 25 texts per request, 4 concurrent)
- **But this is not wired into the actual pipeline**

**EmbeddingService/Pipeline**:
- Calls `provider.embed_batch()` which uses the non-parallel path
- Does not leverage the parallel capabilities that exist

### Performance Gap

| Configuration | Estimated Throughput | Notes |
|---------------|---------------------|-------|
| Current (10 concurrent, 1 text/req) | ~50-100 texts/sec | Massive overhead per request |
| Batch API (100 texts/req, 1 concurrent) | ~300-500 texts/sec | Reduced HTTP overhead |
| Optimal (batch + parallel) | ~1,000-2,000 texts/sec | Full M2 Max utilization |

---

## Research Findings

### Ollama API Capabilities

1. **`/api/embed` endpoint supports batch input**
   - Accepts `"input": ["text1", "text2", ...]` as an array
   - Single request can embed multiple texts
   - Our implementation sends `"input": "single text"` instead

2. **`OLLAMA_NUM_PARALLEL` does NOT work for embeddings**
   - Per [GitHub Issue #8778](https://github.com/ollama/ollama/issues/8778): "ollama currently doesn't allow parallel completions for embedding models"
   - Only way to parallelize is multiple HTTP requests

3. **Ollama batches concurrent requests internally**
   - Per [Glukhov's analysis](https://www.glukhov.org/post/2025/05/how-ollama-handles-parallel-requests/): When multiple requests arrive for the same model, Ollama batches them together
   - This means parallel requests + batch input = maximum throughput

### M2 Max Performance Benchmarks

From [CollabnIX 2025 Guide](https://collabnix.com/ollama-embedded-models-the-complete-technical-guide-for-2025-enterprise-deployment/):

| Model | Optimal Batch Size | Throughput |
|-------|-------------------|------------|
| nomic-embed-text | 128 | 9,340 tokens/sec |
| mxbai-embed-large | 64 | 6,780 tokens/sec |

**Average code chunk**: ~500-1000 tokens
**Theoretical max**: ~9-18 chunks/second per request stream

### Hardware Considerations

- M2 Max: 38-core GPU, ~400 GB/s memory bandwidth
- nomic-embed-text: ~137M parameters, fits entirely in unified memory
- 75% of unified memory available as VRAM (72GB on 96GB model)
- GPU utilization reported at only 15% in current implementations

---

## Bottleneck Analysis

### Current Bottlenecks (Priority Order)

1. **Single-text requests** - Each HTTP request embeds only one text
   - HTTP overhead: ~5-10ms per request
   - With 10 concurrent: 100 texts = 10 requests = 50-100ms overhead
   - With batching: 100 texts = 1 request = 5-10ms overhead

2. **Hardcoded concurrency** - `MAX_CONCURRENT_REQUESTS = 10`
   - Does not adapt to hardware capabilities
   - M2 Max can likely handle 20-30+ concurrent requests

3. **ParallelConfig ignored** - Configuration exists but not used
   - `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY` has no effect on Ollama
   - `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE` has no effect on Ollama

4. **Pipeline not wired** - `embed_batch_parallel()` exists but unused
   - `EmbeddingService.embed_batch()` calls non-parallel path
   - `EmbeddingPipeline.process_batch()` uses sequential path

---

## Optimal Configuration Research

### Batch Size Tuning

Based on research and hardware specs:

| Batch Size | Pros | Cons |
|------------|------|------|
| 25 | Low memory, fast response | Higher HTTP overhead |
| 50 | Balanced | Moderate memory |
| 100 | Good throughput | Higher latency per batch |
| 128+ | Maximum throughput | Memory pressure, timeout risk |

**Recommendation**: Default 50, configurable up to 128

### Concurrency Tuning

| Concurrent Requests | Scenario | Risk |
|--------------------|----------|------|
| 4 | Conservative default | Safe for all systems |
| 8 | M1/M2 MacBooks | Good utilization |
| 16 | M2 Pro/Max | Near-optimal |
| 24 | M2 Max/Ultra | Maximum throughput |
| 32+ | Multi-GPU/Server | Diminishing returns |

**Recommendation**: Auto-detect based on available GPU cores or let user configure

### HTTP Client Tuning

- Connection pooling: Keep-alive for reduced TCP overhead
- HTTP/2: Multiplexing for concurrent requests on single connection
- Timeout: Increase from 30s to 60s for large batches

---

## Existing Infrastructure

### What We Have

1. **ParallelConfig** (config.rs:363-404)
   ```rust
   pub struct ParallelConfig {
       pub enabled: bool,
       pub sub_batch_size: usize,    // default: 25
       pub max_concurrency: usize,   // default: 4
   }
   ```

2. **Environment Variables**
   - `MAPROOM_EMBEDDING_PARALLEL_ENABLED`
   - `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE`
   - `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY`

3. **embed_batch_parallel()** (client.rs:409-482)
   - Semaphore-based concurrency control
   - Sub-batch splitting
   - Maintains result ordering

### What We Need

1. **OllamaProvider batch support** - Use array input in API calls
2. **OllamaProvider parallel support** - Use ParallelConfig
3. **Pipeline integration** - Wire parallel methods to actual usage
4. **Auto-tuning** - Detect optimal settings for hardware

---

## Success Metrics

| Metric | Current | Target | Stretch |
|--------|---------|--------|---------|
| Throughput (texts/sec) | ~50-100 | 500+ | 1,000+ |
| M2 Max GPU utilization | ~15% | 60%+ | 80%+ |
| 10K chunk embedding time | ~2-3 min | <30 sec | <15 sec |
| HTTP requests per 100 texts | 100 | 2-4 | 1-2 |

---

## References

- [Ollama GitHub Issue #8778 - Parallel embeddings](https://github.com/ollama/ollama/issues/8778)
- [How Ollama Handles Parallel Requests](https://www.glukhov.org/post/2025/05/how-ollama-handles-parallel-requests/)
- [CollabnIX Ollama Embedded Models Guide 2025](https://collabnix.com/ollama-embedded-models-the-complete-technical-guide-for-2025-enterprise-deployment/)
- [Apple Metal Performance Shaders Ollama Guide](https://markaicode.com/apple-metal-performance-shaders-m1-m2-ollama-optimization/)
