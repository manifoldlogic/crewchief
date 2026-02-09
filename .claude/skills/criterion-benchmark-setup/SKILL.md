---
name: criterion-benchmark-setup
description: Set up Criterion.rs performance benchmarks with size scaling tests and throughput measurement for parser or other performance-critical code
origin: MLLANG-1005
created: 2026-02-08
tags: [benchmark, performance, criterion, testing]
---

# Criterion.rs Benchmark Setup

## Overview

This skill documents the standard pattern for setting up Criterion.rs performance benchmarks in the Maproom codebase. It covers benchmark file organization, size scaling tests, throughput measurement, and synthetic data generation. This pattern was established for the C parser benchmark and should be replicated for all parser benchmarks and other performance-critical code.

## When to Use

- When adding a new language parser that needs performance validation
- When implementing performance-critical algorithms that need baseline metrics
- When you need to measure throughput (bytes/sec, items/sec) for scaling analysis
- When you want to track performance regressions across code changes

## Pattern/Procedure

### Step 1: Create Benchmark File

Create a benchmark file in `crates/maproom/benches/` with descriptive name:

```bash
touch crates/maproom/benches/FEATURE_bench.rs
```

Example: `c_parser_bench.rs`, `embedding_batch_bench.rs`

### Step 2: Add Benchmark Metadata Comment

Start with a module-level doc comment describing the benchmark:

```rust
//! TICKET_ID: FEATURE Performance Benchmarks
//!
//! Criterion benchmarks for FEATURE performance testing across different DIMENSION.
//! These benchmarks provide baseline metrics for FEATURE scalability.
```

Example from C parser:
```rust
//! MLLANG-1005.3006: C Parser Performance Benchmarks
//!
//! Criterion benchmarks for C parser performance testing across different file sizes.
//! These benchmarks provide baseline metrics for parser scalability.
```

### Step 3: Import Dependencies

Import Criterion and the code under test:

```rust
use crewchief_maproom::MODULE::FUNCTION;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
```

Example:
```rust
use crewchief_maproom::indexer::parser;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
```

### Step 4: Create Data Generator

Implement a function to generate realistic test data of varying sizes:

```rust
/// Generate realistic FEATURE data of approximately the specified size
fn generate_test_data(size_units: usize) -> DataType {
    let target_bytes = size_units * 1024; // or other unit conversion
    let mut data = DataType::with_capacity(target_bytes);

    // Generate realistic data patterns
    // Include variety: different patterns, edge cases, common structures
    while data.len() < target_bytes {
        // Add realistic data elements
    }

    data
}
```

Example from C parser benchmark:
```rust
fn generate_c_source(size_kb: usize) -> String {
    let target_bytes = size_kb * 1024;
    let mut source = String::with_capacity(target_bytes);

    // Standard header includes
    source.push_str("#include <stdio.h>\n");
    source.push_str("#include <stdlib.h>\n");

    // Generate functions with doc comments
    let mut func_count = 0;
    while source.len() < target_bytes {
        func_count += 1;
        source.push_str("/**\n");
        source.push_str(&format!(" * Process data function number {}\n", func_count));
        source.push_str(" */\n");
        source.push_str(&format!("int process_data_{}(void* data, size_t size) {{\n", func_count));
        source.push_str("    return 0;\n");
        source.push_str("}\n\n");
    }

    source
}
```

### Step 5: Implement Size Scaling Benchmark

Create a benchmark group that tests performance across different input sizes:

```rust
/// Benchmark FEATURE with varying input sizes
fn bench_FEATURE_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("FEATURE_size_scaling");

    for size_param in [SMALL, MEDIUM, LARGE, XLARGE].iter() {
        let data = generate_test_data(*size_param);
        let actual_size = measure_size(&data);

        // Set throughput for rate calculation
        group.throughput(Throughput::Bytes(actual_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}UNIT", size_param)),
            &data,
            |b, d| {
                b.iter(|| FUNCTION_UNDER_TEST(black_box(d)));
            },
        );
    }

    group.finish();
}
```

Example from C parser:
```rust
fn bench_parse_c_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("c_parser_size_scaling");

    for size_kb in [1, 10, 100, 1000].iter() {
        let source = generate_c_source(*size_kb);
        let actual_size = source.len();

        group.throughput(Throughput::Bytes(actual_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &source,
            |b, s| {
                b.iter(|| parser::extract_chunks(black_box(s), "c"));
            },
        );
    }

    group.finish();
}
```

### Step 6: Register Benchmark

Register the benchmark with Criterion macros at the end of the file:

```rust
criterion_group!(benches, bench_FEATURE_size_scaling);
criterion_main!(benches);
```

For multiple benchmark functions:
```rust
criterion_group!(
    benches,
    bench_FEATURE_size_scaling,
    bench_FEATURE_other_aspect
);
criterion_main!(benches);
```

### Step 7: Run and Validate

Run the benchmark to verify it works:

```bash
cargo bench --bench FEATURE_bench
```

Expected output:
```
FEATURE_size_scaling/SMALL  time: [XXX us YYY us ZZZ us]
                            thrpt: [A.A MiB/s B.B MiB/s C.C MiB/s]
FEATURE_size_scaling/MEDIUM time: [XXX ms YYY ms ZZZ ms]
                            thrpt: [A.A MiB/s B.B MiB/s C.C MiB/s]
```

## Examples

### C Parser Benchmark (Complete File)

From `crates/maproom/benches/c_parser_bench.rs`:

```rust
//! MLLANG-1005.3006: C Parser Performance Benchmarks
//!
//! Criterion benchmarks for C parser performance testing across different file sizes.
//! These benchmarks provide baseline metrics for parser scalability.

use crewchief_maproom::indexer::parser;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Generate realistic C source code of approximately the specified size
fn generate_c_source(size_kb: usize) -> String {
    let target_bytes = size_kb * 1024;
    let mut source = String::with_capacity(target_bytes);

    // Standard header includes
    source.push_str("#include <stdio.h>\n");
    source.push_str("#include <stdlib.h>\n");
    source.push_str("#include <string.h>\n");
    source.push_str("#include <stdint.h>\n\n");

    // Generate functions with variety
    let mut func_count = 0;
    while source.len() < target_bytes {
        func_count += 1;

        // Function with doc comment
        source.push_str("/**\n");
        source.push_str(&format!(" * Process data function number {}\n", func_count));
        source.push_str(" * @param data Pointer to data\n");
        source.push_str(" * @return Status code\n");
        source.push_str(" */\n");
        source.push_str(&format!("int process_data_{}(void* data, size_t size) {{\n", func_count));
        source.push_str("    if (data == NULL) return -1;\n");
        source.push_str("    return 0;\n");
        source.push_str("}\n\n");

        // Add static functions for variety
        if func_count % 3 == 0 {
            source.push_str(&format!("static void helper_{}(int param) {{\n", func_count));
            source.push_str("    printf(\"%d\\n\", param);\n");
            source.push_str("}\n\n");
        }
    }

    source
}

/// Benchmark parsing C files of varying sizes (1KB, 10KB, 100KB, 1MB)
fn bench_parse_c_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("c_parser_size_scaling");

    for size_kb in [1, 10, 100, 1000].iter() {
        let source = generate_c_source(*size_kb);
        let actual_size = source.len();

        group.throughput(Throughput::Bytes(actual_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &source,
            |b, s| {
                b.iter(|| parser::extract_chunks(black_box(s), "c"));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse_c_by_size);
criterion_main!(benches);
```

### Size Selection Guidelines

Choose test sizes that cover the expected usage range:

**Small (1-10 units):** Minimal overhead measurement, fast iteration
**Medium (10-100 units):** Typical use case size
**Large (100-1000 units):** Stress test, detect scaling issues
**Extra Large (1000+ units):** Edge case, validate linear scaling

For parsers:
- 1KB: Single function/class file
- 10KB: Typical module
- 100KB: Large source file
- 1MB: Concatenated multi-file scenario

For batch operations:
- 10 items: Minimal batch
- 100 items: Typical batch
- 1000 items: Large batch
- 10000 items: Stress test

## Best Practices

### Data Generation

1. Generate realistic data that exercises real code paths
2. Include variety (different patterns, edge cases)
3. Avoid unrealistic uniformity that might hit cache too favorably
4. Pre-allocate buffers with capacity hints to avoid measurement noise

### Throughput Measurement

1. Always set `throughput` for rate-based analysis
2. Use `Throughput::Bytes` for data processing
3. Use `Throughput::Elements` for item-based operations
4. Measure actual size, not requested size (generation may overshoot)

### Black Box Usage

1. Wrap inputs with `black_box()` to prevent compiler optimization
2. Don't black_box the generator, only the generated data
3. Black box prevents the compiler from optimizing away the work

### Benchmark Organization

1. One benchmark file per feature or module
2. Group related benchmarks with `benchmark_group`
3. Use descriptive parameter names in `BenchmarkId`
4. Include units in parameter labels (KB, MB, items, etc.)

## References

- Ticket: MLLANG-1005
- Related files:
  - `crates/maproom/benches/c_parser_bench.rs` (reference implementation)
  - `crates/maproom/benches/parser_bench.rs` (multi-language parser benchmarks)
  - `crates/maproom/benches/embedding_performance.rs` (batch operation benchmarks)
- Criterion.rs documentation: https://bheisler.github.io/criterion.rs/book/
- Commit: 384fd7e6 (MLLANG-1005.3006 add C parser performance benchmarks)
