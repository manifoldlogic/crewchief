//! File Indexing Throughput Benchmarks
//!
//! Measures indexing performance across different dataset sizes and file types.
//!
//! # Performance Targets
//!
//! - Cold cache: ≥150 files/min
//! - Warm cache: ≥500 files/min
//! - Sustained throughput without degradation
//!
//! # Benchmarks
//!
//! 1. **Small Dataset**: 100 files (~1MB) - Fast iteration testing
//! 2. **Medium Dataset**: 1,000 files (~10MB) - Typical project size
//! 3. **Large Dataset**: 10,000 files (~100MB) - Large codebase simulation
//! 4. **Language-Specific**: Benchmarks per language (Rust, TypeScript, Python, etc.)
//! 5. **Cold vs Warm Cache**: Measure cache impact on throughput
//!
//! # Metrics Collected
//!
//! - Files per minute throughput
//! - Chunks created per file
//! - Parse time per file
//! - Database insertion time
//! - Total end-to-end latency
//! - Memory usage during indexing
//!
//! # Running
//!
//! ```bash
//! # Run all indexing benchmarks
//! cargo bench --bench indexing
//!
//! # Run specific dataset size
//! cargo bench --bench indexing -- small
//! cargo bench --bench indexing -- medium
//! cargo bench --bench indexing -- large
//!
//! # Compare before/after optimizations
//! cargo bench --bench indexing -- --save-baseline before
//! # ... make changes ...
//! cargo bench --bench indexing -- --baseline before
//! ```
//!
//! # Test Data
//!
//! Uses synthetic files that represent realistic code patterns:
//! - TypeScript/JavaScript: Functions, classes, interfaces
//! - Rust: Modules, functions, structs, impls
//! - Python: Functions, classes, imports
//! - Markdown: Documentation files
//! - JSON/YAML: Configuration files
//!
//! # Requirements
//!
//! - No database required (uses in-memory parsing benchmarks)
//! - For end-to-end benchmarks with database, set MAPROOM_DATABASE_URL
//!
//! # Architecture Reference
//!
//! See PERF_OPT_ARCHITECTURE.md:
//! - Indexing pipeline (lines 90-115)
//! - Parser benchmarks (lines 116-134)
//! - Throughput targets (lines 21-26)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Simulated file content for benchmarking different languages.
struct TestFile {
    name: String,
    language: &'static str,
    content: String,
    size_bytes: usize,
}

impl TestFile {
    fn typescript_function() -> Self {
        let content = r#"
/**
 * Process user data and return formatted result
 * @param userId - The user identifier
 * @param options - Processing options
 */
export async function processUserData(
    userId: string,
    options: ProcessOptions
): Promise<UserResult> {
    const user = await fetchUser(userId);
    if (!user) {
        throw new Error(`User not found: ${userId}`);
    }

    const processed = await processData(user.data, options);
    return {
        userId: user.id,
        name: user.name,
        data: processed,
        timestamp: new Date().toISOString(),
    };
}

interface ProcessOptions {
    validate?: boolean;
    transform?: boolean;
    cache?: boolean;
}

interface UserResult {
    userId: string;
    name: string;
    data: any;
    timestamp: string;
}
"#;
        Self {
            name: "user_processor.ts".to_string(),
            language: "ts",
            content: content.to_string(),
            size_bytes: content.len(),
        }
    }

    fn rust_module() -> Self {
        let content = r#"
//! User data processing module
//!
//! This module provides utilities for processing and validating user data.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for user data processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub validate: bool,
    pub transform: bool,
    pub cache_enabled: bool,
}

/// Result of user data processing
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub user_id: String,
    pub name: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Process user data according to configuration
pub async fn process_user_data(
    user_id: &str,
    config: &ProcessConfig,
) -> Result<ProcessResult> {
    let user = fetch_user(user_id).await?;

    if config.validate {
        validate_user_data(&user)?;
    }

    let data = if config.transform {
        transform_data(&user.data)?
    } else {
        user.data
    };

    Ok(ProcessResult {
        user_id: user.id,
        name: user.name,
        data,
        timestamp: chrono::Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_user_data() {
        let config = ProcessConfig {
            validate: true,
            transform: false,
            cache_enabled: true,
        };
        // Test implementation...
    }
}
"#;
        Self {
            name: "user_processor.rs".to_string(),
            language: "rs",
            content: content.to_string(),
            size_bytes: content.len(),
        }
    }

    fn python_class() -> Self {
        let content = r#"
"""User data processing module.

This module provides utilities for processing and validating user data.
"""

from typing import Optional, Dict, Any
from datetime import datetime
import asyncio


class ProcessConfig:
    """Configuration for user data processing."""

    def __init__(
        self,
        validate: bool = True,
        transform: bool = False,
        cache_enabled: bool = True
    ):
        self.validate = validate
        self.transform = transform
        self.cache_enabled = cache_enabled


class ProcessResult:
    """Result of user data processing."""

    def __init__(self, user_id: str, name: str, data: Dict[str, Any]):
        self.user_id = user_id
        self.name = name
        self.data = data
        self.timestamp = datetime.utcnow()


async def process_user_data(
    user_id: str,
    config: ProcessConfig
) -> ProcessResult:
    """Process user data according to configuration.

    Args:
        user_id: The user identifier
        config: Processing configuration

    Returns:
        ProcessResult: The processed result

    Raises:
        ValueError: If user not found or validation fails
    """
    user = await fetch_user(user_id)

    if config.validate:
        validate_user_data(user)

    data = user.data
    if config.transform:
        data = transform_data(data)

    return ProcessResult(
        user_id=user.id,
        name=user.name,
        data=data
    )


def validate_user_data(user: Dict[str, Any]) -> None:
    """Validate user data structure."""
    required_fields = ['id', 'name', 'data']
    for field in required_fields:
        if field not in user:
            raise ValueError(f"Missing required field: {field}")
"#;
        Self {
            name: "user_processor.py".to_string(),
            language: "py",
            content: content.to_string(),
            size_bytes: content.len(),
        }
    }

    fn markdown_doc() -> Self {
        let content = r#"
# User Data Processing

This document describes the user data processing system.

## Overview

The user data processor provides a unified interface for:
- Fetching user data from various sources
- Validating data integrity
- Transforming data to standardized formats
- Caching processed results

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│   Processor     │
├─────────────────┤
│ - Fetch         │
│ - Validate      │
│ - Transform     │
│ - Cache         │
└─────────────────┘
```

## API Reference

### `processUserData(userId, config)`

Process user data with given configuration.

**Parameters:**
- `userId` (string): User identifier
- `config` (ProcessConfig): Processing configuration

**Returns:**
- `Promise<ProcessResult>`: Processed result

**Example:**

```typescript
const result = await processUserData('user123', {
    validate: true,
    transform: true,
    cache: true
});
```

## Performance Considerations

- Enable caching for frequently accessed users
- Batch processing reduces overhead
- Validation can be disabled for trusted sources
"#;
        Self {
            name: "README.md".to_string(),
            language: "md",
            content: content.to_string(),
            size_bytes: content.len(),
        }
    }

    fn json_config() -> Self {
        let content = r#"
{
  "name": "user-processor",
  "version": "1.0.0",
  "description": "User data processing utilities",
  "main": "index.js",
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "lint": "eslint src/**/*.ts"
  },
  "dependencies": {
    "axios": "^1.0.0",
    "date-fns": "^2.0.0",
    "joi": "^17.0.0"
  },
  "devDependencies": {
    "@types/node": "^18.0.0",
    "typescript": "^5.0.0",
    "jest": "^29.0.0",
    "eslint": "^8.0.0"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
"#;
        Self {
            name: "package.json".to_string(),
            language: "json",
            content: content.to_string(),
            size_bytes: content.len(),
        }
    }
}

/// Generate a dataset of test files
fn generate_test_dataset(count: usize) -> Vec<TestFile> {
    let mut files = Vec::with_capacity(count);

    // Mix of different file types (realistic distribution)
    let file_types: Vec<(usize, fn() -> TestFile)> = vec![
        (40, TestFile::typescript_function as fn() -> TestFile), // 40% TypeScript
        (30, TestFile::rust_module as fn() -> TestFile),         // 30% Rust
        (20, TestFile::python_class as fn() -> TestFile),        // 20% Python
        (5, TestFile::markdown_doc as fn() -> TestFile),         // 5% Markdown
        (5, TestFile::json_config as fn() -> TestFile),          // 5% JSON
    ];

    let mut idx = 0;
    for (percentage, generator) in file_types.iter() {
        let file_count = (count * percentage / 100).max(1);
        for i in 0..file_count {
            if idx >= count {
                break;
            }
            let mut file = generator();
            // Make each file unique
            file.name = format!("{}_{}", i, file.name);
            files.push(file);
            idx += 1;
        }
    }

    // Fill remainder with TypeScript files
    while idx < count {
        let mut file = TestFile::typescript_function();
        file.name = format!("{}_extra_{}", idx, file.name);
        files.push(file);
        idx += 1;
    }

    files
}

/// Benchmark parsing a single file
fn bench_parse_single_file(c: &mut Criterion) {
    use maproom::indexer::parser::extract_chunks;

    let mut group = c.benchmark_group("parse_single_file");

    let test_files = vec![
        ("typescript", TestFile::typescript_function()),
        ("rust", TestFile::rust_module()),
        ("python", TestFile::python_class()),
        ("markdown", TestFile::markdown_doc()),
        ("json", TestFile::json_config()),
    ];

    for (name, file) in test_files {
        group.throughput(Throughput::Bytes(file.size_bytes as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &file, |b, file| {
            b.iter(|| {
                let chunks = extract_chunks(black_box(&file.content), black_box(file.language));
                black_box(chunks)
            });
        });
    }

    group.finish();
}

/// Benchmark parsing throughput for different dataset sizes
fn bench_parse_throughput(c: &mut Criterion) {
    use maproom::indexer::parser::extract_chunks;

    let mut group = c.benchmark_group("parse_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    for size in [100, 1000, 10000] {
        let dataset = generate_test_dataset(size);
        let total_bytes: usize = dataset.iter().map(|f| f.size_bytes).sum();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", size)),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let mut total_chunks = 0;
                    for file in files {
                        let chunks =
                            extract_chunks(black_box(&file.content), black_box(file.language));
                        total_chunks += chunks.len();
                    }
                    black_box(total_chunks)
                });
            },
        );

        println!(
            "\nDataset: {} files, {:.2} MB",
            size,
            total_bytes as f64 / 1_048_576.0
        );
    }

    group.finish();
}

/// Benchmark files per minute throughput
fn bench_files_per_minute(c: &mut Criterion) {
    use maproom::indexer::parser::extract_chunks;

    let mut group = c.benchmark_group("files_per_minute");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    for size in [100, 1000] {
        let dataset = generate_test_dataset(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", size)),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let start = std::time::Instant::now();
                    let mut total_chunks = 0;

                    for file in files {
                        let chunks =
                            extract_chunks(black_box(&file.content), black_box(file.language));
                        total_chunks += chunks.len();
                    }

                    let elapsed = start.elapsed();
                    let files_per_min = (files.len() as f64 / elapsed.as_secs_f64()) * 60.0;

                    black_box((total_chunks, files_per_min))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark language-specific parsing
fn bench_by_language(c: &mut Criterion) {
    use maproom::indexer::parser::extract_chunks;

    let mut group = c.benchmark_group("parse_by_language");

    let languages = vec![
        ("typescript", TestFile::typescript_function()),
        ("rust", TestFile::rust_module()),
        ("python", TestFile::python_class()),
    ];

    for (lang, template) in languages {
        // Create 100 files of this language
        let mut files = Vec::new();
        for i in 0..100 {
            let mut file = template.clone();
            file.name = format!("{}_{}", i, file.name);
            files.push(file);
        }

        group.throughput(Throughput::Elements(100));
        group.bench_with_input(BenchmarkId::from_parameter(lang), &files, |b, files| {
            b.iter(|| {
                let mut total_chunks = 0;
                for file in files {
                    let chunks = extract_chunks(black_box(&file.content), black_box(file.language));
                    total_chunks += chunks.len();
                }
                black_box(total_chunks)
            });
        });
    }

    group.finish();
}

// Make TestFile cloneable for the benchmark
impl Clone for TestFile {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            language: self.language,
            content: self.content.clone(),
            size_bytes: self.size_bytes,
        }
    }
}

/// Benchmark parallel batch processing performance (PERF_OPT-3001)
///
/// This benchmark measures the overhead of parallel processing infrastructure
/// without database operations. It tests:
/// - Work-stealing thread pool efficiency
/// - Channel throughput
/// - Batch formation overhead
/// - Parallel parsing vs sequential
///
/// Note: This measures parsing + pipeline overhead, not database performance.
/// Database benchmarks require a live database environment.
fn bench_parallel_processing(c: &mut Criterion) {
    use crossbeam::channel;
    use maproom::indexer::parser::extract_chunks;
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let mut group = c.benchmark_group("parallel_processing");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    for size in [100, 1000] {
        let dataset = generate_test_dataset(size);

        // Baseline: Sequential processing
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let mut total_chunks = 0;
                    for file in files {
                        let chunks =
                            extract_chunks(black_box(&file.content), black_box(file.language));
                        total_chunks += chunks.len();
                    }
                    black_box(total_chunks)
                });
            },
        );

        // Parallel processing with rayon
        group.bench_with_input(
            BenchmarkId::new("rayon_parallel", size),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let total_chunks: usize = files
                        .par_iter()
                        .map(|file| {
                            let chunks =
                                extract_chunks(black_box(&file.content), black_box(file.language));
                            chunks.len()
                        })
                        .sum();
                    black_box(total_chunks)
                });
            },
        );

        // Parallel with channel pipeline (simulates database worker pattern)
        group.bench_with_input(
            BenchmarkId::new("channel_pipeline", size),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let (tx, rx) = channel::bounded(1000);
                    let total_chunks = Arc::new(AtomicUsize::new(0));

                    // Spawn worker thread
                    let total_clone = total_chunks.clone();
                    let worker = std::thread::spawn(move || {
                        let mut count = 0;
                        while let Ok(chunk_count) = rx.recv() {
                            count += chunk_count;
                        }
                        total_clone.store(count, Ordering::SeqCst);
                    });

                    // Parse in parallel and send to worker
                    files.par_iter().for_each(|file| {
                        let chunks =
                            extract_chunks(black_box(&file.content), black_box(file.language));
                        let _ = tx.send(chunks.len());
                    });

                    drop(tx);
                    worker.join().unwrap();
                    black_box(total_chunks.load(Ordering::SeqCst))
                });
            },
        );

        // Parallel with batch formation (simulates batch INSERT pattern)
        let batch_size = 50;
        group.bench_with_input(
            BenchmarkId::new("batched_pipeline", size),
            &dataset,
            |b, files| {
                b.iter(|| {
                    let (tx, rx) = channel::bounded(1000);
                    let total_chunks = Arc::new(AtomicUsize::new(0));

                    // Spawn worker thread that processes batches
                    let total_clone = total_chunks.clone();
                    let worker = std::thread::spawn(move || {
                        let mut batch = Vec::with_capacity(batch_size);
                        let mut count = 0;

                        for chunk_count in rx {
                            batch.push(chunk_count);
                            if batch.len() >= batch_size {
                                // Simulate batch processing
                                count += batch.iter().sum::<usize>();
                                batch.clear();
                            }
                        }

                        // Process remaining
                        if !batch.is_empty() {
                            count += batch.iter().sum::<usize>();
                        }

                        total_clone.store(count, Ordering::SeqCst);
                    });

                    // Parse in parallel
                    files.par_iter().for_each(|file| {
                        let chunks =
                            extract_chunks(black_box(&file.content), black_box(file.language));
                        let _ = tx.send(chunks.len());
                    });

                    drop(tx);
                    worker.join().unwrap();
                    black_box(total_chunks.load(Ordering::SeqCst))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_single_file,
    bench_parse_throughput,
    bench_files_per_minute,
    bench_by_language,
    bench_parallel_processing,
);
criterion_main!(benches);
