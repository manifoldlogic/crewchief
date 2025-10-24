# Ticket: PERF_OPT-5001: Memory Optimization

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement memory optimizations including string interning, vector quantization, buffer pooling, and allocation reduction to achieve <500MB memory usage target for 100k chunks.

## Background
PERF_OPT_PLAN.md (lines 103-107) identifies critical memory optimizations: string interning, vector quantization, buffer pooling, and allocation reduction. The target is <500MB memory usage (PERF_OPT_PLAN.md line 124).

PERF_OPT_ANALYSIS.md (line 17) notes that quantization can provide 8x memory reduction. The architecture document (PERF_OPT_ARCHITECTURE.md lines 109-139) provides detailed implementations for string interning and vector quantization.

## Acceptance Criteria
- [ ] String interning implemented for repeated strings (paths, symbols)
- [ ] Vector quantization implemented for embeddings (f32 → i8)
- [ ] Buffer pooling implemented for file reading and parsing
- [ ] Allocation reduction achieved through optimizations
- [ ] Memory usage <500MB for 100k chunks verified
- [ ] Memory profiling shows reduced allocation count
- [ ] No memory leaks detected in long-running tests
- [ ] Performance benchmarks show no significant slowdown

## Technical Requirements

### String Interning
Implement string interning (PERF_OPT_ARCHITECTURE.md lines 111-123):

```rust
pub struct StringInterner {
    strings: HashMap<String, Arc<str>>,
}

impl StringInterner {
    pub fn intern(&mut self, s: String) -> Arc<str> {
        self.strings.entry(s.clone())
            .or_insert_with(|| Arc::from(s.as_str()))
            .clone()
    }
}
```

Intern repeated strings:
- File paths (highly repeated)
- Symbol names (types, functions, classes)
- Language identifiers
- Repository identifiers
- Error messages

Benefits:
- Reduced memory for repeated strings
- Faster comparisons (pointer equality)
- Better cache locality

### Vector Quantization
Implement embedding quantization (PERF_OPT_ARCHITECTURE.md lines 126-139):

```rust
pub fn quantize_embedding(embedding: &[f32]) -> Vec<i8> {
    embedding.iter()
        .map(|&v| (v * 127.0).round() as i8)
        .collect()
}

pub fn dequantize_embedding(quantized: &[i8]) -> Vec<f32> {
    quantized.iter()
        .map(|&v| v as f32 / 127.0)
        .collect()
}
```

Benefits:
- 4x memory reduction (f32 → i8)
- Faster vector operations
- More embeddings in cache

Trade-off:
- Slight accuracy loss (acceptable for search)
- Need to benchmark impact on search quality

### Buffer Pooling
Implement buffer pool for reusable allocations:

```rust
pub struct BufferPool {
    buffers: Arc<Mutex<Vec<Vec<u8>>>>,
    buffer_size: usize,
    max_pool_size: usize,
}

impl BufferPool {
    pub fn acquire(&self) -> Vec<u8> {
        let mut pool = self.buffers.lock().unwrap();
        pool.pop().unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }

    pub fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        let mut pool = self.buffers.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push(buffer);
        }
    }
}
```

Pool buffers for:
- File reading (reuse read buffers)
- Parsing (reuse AST buffers)
- String building (reuse StringBuilder)
- Temporary allocations in hot loops

### Allocation Reduction
Reduce allocations through:

1. **Pre-allocation**: Size vectors correctly from the start
```rust
let mut results = Vec::with_capacity(expected_size);
```

2. **Reuse**: Reuse vectors with `.clear()` instead of allocating new
```rust
buffer.clear();  // Keeps capacity
// vs
buffer = Vec::new();  // Allocates new
```

3. **Avoid clones**: Use references and borrowing
```rust
// Avoid
let copy = chunk.clone();
process(copy);

// Prefer
process(&chunk);
```

4. **Cow (Clone-on-Write)**: For mostly-read data
```rust
use std::borrow::Cow;
fn process(data: Cow<str>) { /* ... */ }
```

5. **Box large data**: Keep stack frames small
```rust
struct Large { data: Box<[u8; 1_000_000]> }
```

### Memory Profiling
Track memory metrics:
```rust
pub struct MemoryMetrics {
    total_allocated: AtomicUsize,
    current_usage: AtomicUsize,
    peak_usage: AtomicUsize,
    allocation_count: AtomicU64,
}
```

Use jemalloc for detailed profiling:
```toml
[dependencies]
jemallocator = "0.5"
```

```rust
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

### Small String Optimization
Use `SmallVec` for small collections:
```rust
use smallvec::SmallVec;

// Inline up to 16 bytes, avoiding heap allocation
type SmallString = SmallVec<[u8; 16]>;
```

### Compact Data Structures
Use compact representations:
```rust
// Instead of Vec<bool> (1 byte per bool)
use bit_vec::BitVec;  // 1 bit per bool

// Instead of HashMap (large overhead)
use linear_map::LinearMap;  // For small maps (<16 entries)
```

## Implementation Notes

### String Interning Strategy
Global interner for common strings:
```rust
lazy_static! {
    static ref STRING_INTERNER: Mutex<StringInterner> = Mutex::new(StringInterner::new());
}

pub fn intern(s: String) -> Arc<str> {
    STRING_INTERNER.lock().unwrap().intern(s)
}
```

Per-indexing-run interner for file-specific strings:
```rust
pub struct IndexingContext {
    interner: StringInterner,
    // ...
}
```

### Quantization Quality Testing
Verify quantization doesn't hurt search quality:
1. Benchmark search accuracy with quantized embeddings
2. Compare against unquantized baseline
3. Measure recall@k for various k
4. Acceptable threshold: >95% recall maintained

### Memory Budget Allocation
Allocate 500MB budget (PERF_OPT_PLAN.md line 124):
- String interning: ~50MB (paths, symbols)
- Embeddings (quantized): ~200MB (100k chunks × 2KB/chunk)
- Database connections: ~50MB
- Caches: ~150MB (from PERF_OPT-4001)
- Overhead: ~50MB (Rust runtime, etc.)

### Profiling Integration
Use valgrind/heaptrack for memory profiling:
```bash
# Heap profiling
heaptrack ./target/release/crewchief-maproom scan /path/to/repo

# Analyze results
heaptrack_gui heaptrack.crewchief-maproom.*.gz
```

### Testing Strategy
Test memory usage:
1. Small repo: 100 files, <10MB
2. Medium repo: 1,000 files, ~50MB
3. Large repo: 10,000 files, ~300MB
4. Huge repo: 100,000 files, should be <500MB

Test memory leaks:
- Long-running indexing (24h test)
- Repeated search queries (1M queries)
- Monitor with `valgrind --leak-check=full`

## Dependencies
- **PERF_OPT-1001** - Requires benchmark suite to measure memory usage
- **PERF_OPT-1002** - Requires memory profiling from bottleneck analysis
- **PERF_OPT-4001** - Cache memory usage must fit within budget
- jemalloc or mimalloc for better memory allocation
- smallvec, bit-vec for compact data structures

## Risk Assessment
- **Risk**: String interning may increase memory if strings aren't repeated
  - **Mitigation**: Profile to verify strings are actually repeated, measure overhead
- **Risk**: Quantization may hurt search quality
  - **Mitigation**: Benchmark search accuracy, make quantization optional if needed
- **Risk**: Buffer pooling may cause memory fragmentation
  - **Mitigation**: Limit pool size, use fixed-size buffers
- **Risk**: Over-optimization may hurt code readability
  - **Mitigation**: Document optimizations, use only where profiling shows benefit

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add smallvec, bit-vec, jemallocator dependencies
- `crates/maproom/src/memory/interner.rs` - New string interner module
- `crates/maproom/src/memory/quantization.rs` - New quantization module
- `crates/maproom/src/memory/pool.rs` - New buffer pool module
- `crates/maproom/src/memory/metrics.rs` - New memory metrics module
- `crates/maproom/src/embeddings/mod.rs` - Integrate quantization
- `crates/maproom/src/indexer/mod.rs` - Integrate string interning and buffer pooling
- `crates/maproom/src/main.rs` - Configure jemalloc allocator
- `crates/maproom/benches/memory.rs` - Update memory benchmarks
- `crates/maproom/tests/memory_leak.rs` - New memory leak test
