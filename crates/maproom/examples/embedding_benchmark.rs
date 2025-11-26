//! Simple embedding performance test
//!
//! This is a standalone example that measures actual Ollama embedding performance
//! without the complexity of Criterion benchmarks. It provides real-world timing
//! data for embedding generation.
//!
//! Run with: cargo run --release --example embedding_benchmark

use crewchief_maproom::embedding::cache::EmbeddingCache;
use crewchief_maproom::embedding::config::{CacheConfig, ParallelConfig};
use crewchief_maproom::embedding::ollama::OllamaProvider;
use crewchief_maproom::embedding::service::EmbeddingService;
use std::sync::Arc;
use std::time::Instant;

fn generate_unique_text(index: usize) -> String {
    format!(
        r#"
/**
 * Process user data for request {}
 * @param data - Input data containing user information
 * @returns Processed result with unique ID {}
 */
export async function processData{}(data: UserInput): Promise<ProcessedResult> {{
    const validated = await validateInput(data);
    if (!validated) {{
        throw new Error("Invalid input data for request {}");
    }}

    const processed = await transformData(validated, {{
        requestId: {},
        timestamp: Date.now(),
        version: "1.0.{}",
    }});

    return {{
        id: "result-{}",
        data: processed,
        metadata: {{
            processedAt: new Date().toISOString(),
            requestId: {},
        }},
    }};
}}
"#,
        index, index, index, index, index, index, index, index
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Ollama Embedding Performance Benchmark ===\n");

    // Configure Ollama with parallel processing enabled
    let parallel_config = ParallelConfig {
        enabled: true,      // Test with optimal sub-batch size
        sub_batch_size: 10, // Smaller batches for better parallelism
        max_concurrency: 6, // More concurrency with 12 threads
    };

    let cache_config = CacheConfig {
        max_entries: 1, // Minimal cache for accurate measurements
        ttl_seconds: 0, // Expire immediately
        enable_metrics: false,
    };

    let model = "nomic-embed-text".to_string();
    let endpoint = "http://ollama:11434/api/embed".to_string();

    println!("Configuration:");
    println!("  Provider: Ollama");
    println!("  Model: {}", model);
    println!("  Endpoint: http://ollama:11434");
    println!("  Cache: Disabled (for accurate measurements)");
    println!(
        "  Parallel: {} (sub_batch={}, concurrency={})\n",
        parallel_config.enabled, parallel_config.sub_batch_size, parallel_config.max_concurrency
    );

    // Create provider and cache separately, then compose into service
    let provider = OllamaProvider::new_with_config(endpoint, model, parallel_config)?;
    let cache = EmbeddingCache::new(cache_config)?;
    let service = EmbeddingService::new(Box::new(provider), Arc::new(cache));

    // Test 1: Single embedding (cold start)
    println!("Test 1: Single Embedding (Cold Start)");
    println!("{}", "-".repeat(50));
    let text = generate_unique_text(0);
    let start = Instant::now();
    match service.embed_text(&text).await {
        Ok(embedding) => {
            let elapsed = start.elapsed();
            println!("✓ Success");
            println!("  Latency: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
            println!("  Embedding dim: {}", embedding.len());
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 2: Single embedding (warm start)
    println!("Test 2: Single Embedding (Warm Start)");
    println!("{}", "-".repeat(50));
    let mut latencies = Vec::new();
    for i in 1..=10 {
        let text = generate_unique_text(i);
        let start = Instant::now();
        match service.embed_text(&text).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                latencies.push(elapsed.as_secs_f64() * 1000.0);
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
            Err(e) => {
                println!("\n✗ Failed on iteration {}: {}", i, e);
                return Err(e.into());
            }
        }
    }
    println!("\n✓ Completed 10 iterations");

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
    let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
    let mean = latencies.iter().sum::<f64>() / latencies.len() as f64;

    println!("  Mean: {:.2} ms", mean);
    println!("  p50:  {:.2} ms", p50);
    println!("  p95:  {:.2} ms", p95);
    println!("  p99:  {:.2} ms", p99);
    println!();

    // Test 3: Small batch (10 chunks)
    println!("Test 3: Small Batch (10 chunks)");
    println!("{}", "-".repeat(50));
    let texts: Vec<String> = (100..110).map(generate_unique_text).collect();
    let start = Instant::now();
    match service.embed_batch(texts.clone()).await {
        Ok(embeddings) => {
            let elapsed = start.elapsed();
            let chunks_per_sec = 10.0 / elapsed.as_secs_f64();
            let chunks_per_min = chunks_per_sec * 60.0;

            println!("✓ Success");
            println!("  Total time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
            println!(
                "  Per-chunk avg: {:.2} ms",
                elapsed.as_secs_f64() * 1000.0 / 10.0
            );
            println!("  Throughput: {:.1} chunks/sec", chunks_per_sec);
            println!("  Throughput: {:.1} chunks/min", chunks_per_min);
            println!("  Embeddings: {}", embeddings.len());
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 4: Medium batch (50 chunks)
    println!("Test 4: Medium Batch (50 chunks)");
    println!("{}", "-".repeat(50));
    let texts: Vec<String> = (200..250).map(generate_unique_text).collect();
    let start = Instant::now();
    match service.embed_batch(texts.clone()).await {
        Ok(embeddings) => {
            let elapsed = start.elapsed();
            let chunks_per_sec = 50.0 / elapsed.as_secs_f64();
            let chunks_per_min = chunks_per_sec * 60.0;

            println!("✓ Success");
            println!("  Total time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
            println!(
                "  Per-chunk avg: {:.2} ms",
                elapsed.as_secs_f64() * 1000.0 / 50.0
            );
            println!("  Throughput: {:.1} chunks/sec", chunks_per_sec);
            println!("  Throughput: {:.1} chunks/min", chunks_per_min);
            println!("  Embeddings: {}", embeddings.len());
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 5: Large Batch (100 chunks)
    println!("Test 5: Large Batch (100 chunks)");
    println!("{}", "-".repeat(50));
    let texts: Vec<String> = (300..400).map(generate_unique_text).collect();
    let start = Instant::now();
    match service.embed_batch(texts.clone()).await {
        Ok(embeddings) => {
            let elapsed = start.elapsed();
            let chunks_per_sec = 100.0 / elapsed.as_secs_f64();
            let chunks_per_min = chunks_per_sec * 60.0;

            println!("✓ Success");
            println!("  Total time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
            println!(
                "  Per-chunk avg: {:.2} ms",
                elapsed.as_secs_f64() * 1000.0 / 100.0
            );
            println!("  Throughput: {:.1} chunks/sec", chunks_per_sec);
            println!("  Throughput: {:.1} chunks/min", chunks_per_min);
            println!("  Embeddings: {}", embeddings.len());

            // Check target achievement
            println!();
            println!("Target Achievement:");
            if chunks_per_min >= 500.0 {
                println!(
                    "  ✓ PASS: Exceeds 500 chunks/min target ({:.1} chunks/min)",
                    chunks_per_min
                );
            } else {
                println!(
                    "  ✗ FAIL: Below 500 chunks/min target ({:.1} chunks/min)",
                    chunks_per_min
                );
            }
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 6: Sustained throughput (200 chunks to measure stability)
    println!("Test 6: Sustained Throughput (200 chunks)");
    println!("{}", "-".repeat(50));
    let texts: Vec<String> = (500..700).map(generate_unique_text).collect();
    let start = Instant::now();

    // Process in batches of 100 (Ollama's max batch size)
    let mut total_embeddings = 0;
    for (i, chunk) in texts.chunks(100).enumerate() {
        print!("  Batch {}/2...", i + 1);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match service.embed_batch(chunk.to_vec()).await {
            Ok(embeddings) => {
                total_embeddings += embeddings.len();
                println!(" ✓ ({} embeddings)", embeddings.len());
            }
            Err(e) => {
                println!("\n✗ Failed on batch {}: {}", i + 1, e);
                return Err(e.into());
            }
        }
    }

    let elapsed = start.elapsed();
    let chunks_per_sec = 200.0 / elapsed.as_secs_f64();
    let chunks_per_min = chunks_per_sec * 60.0;

    println!("✓ Completed 200 chunks");
    println!("  Total time: {:.2} s", elapsed.as_secs_f64());
    println!("  Throughput: {:.1} chunks/sec", chunks_per_sec);
    println!("  Throughput: {:.1} chunks/min", chunks_per_min);
    println!("  Total embeddings: {}", total_embeddings);
    println!();

    // Summary
    println!("=== Summary ===\n");
    println!("✓ All tests completed successfully");
    println!();
    println!("Key Metrics:");
    println!("  Single embedding (warm): {:.2} ms (p50)", p50);
    println!("  Recommended batch size: 50-100 chunks");
    println!("  Sustained throughput: {:.1} chunks/min", chunks_per_min);

    Ok(())
}
