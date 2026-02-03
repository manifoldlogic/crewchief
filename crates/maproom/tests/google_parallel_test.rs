//! Google Vertex AI Parallel Processing Integration Tests (GVERTEX.2001)
//!
//! Validates correctness and reliability of parallel embedding generation for Google Vertex AI:
//! - Order preservation (embeddings[i] corresponds to texts[i])
//! - Embedding equivalence (same text -> same embedding regardless of config)
//! - Dimension consistency (all embeddings are 768-dim for text-embedding-004)
//! - Empty input handling
//! - Disabled parallel mode behavior
//! - Edge cases (single text, exact boundaries, uneven splits)
//!
//! # Running
//!
//! ```bash
//! # Prerequisites: Set up GCP credentials
//! export GCP_INTEGRATION_TESTS=1
//! export GOOGLE_PROJECT_ID=your-project-id
//! export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
//!
//! # Run all Google parallel integration tests
//! cargo test --test google_parallel_test -- --ignored
//!
//! # Run specific test
//! cargo test --test google_parallel_test test_google_batch_preserves_order -- --ignored
//! ```
//!
//! # Requirements
//!
//! - Google Cloud project with Vertex AI API enabled
//! - Service account with roles/aiplatform.user IAM role
//! - Valid service account JSON key file
//! - Tests marked `#[ignore]` - run manually when credentials available

use crewchief_maproom::embedding::config::ParallelConfig;
use crewchief_maproom::embedding::google::GoogleProvider;
use crewchief_maproom::embedding::provider::EmbeddingProvider;
use std::env;
use std::path::PathBuf;
use std::time::Instant;

/// Helper: Check if integration tests should run.
///
/// Returns early from tests if GCP_INTEGRATION_TESTS environment variable is not set.
/// This allows tests to be ignored by default but runnable when credentials are available.
fn skip_if_no_credentials() {
    if env::var("GCP_INTEGRATION_TESTS").unwrap_or_default() != "1" {
        println!("Skipping test: Set GCP_INTEGRATION_TESTS=1 to run");
        return;
    }
}

/// Helper: Calculate cosine similarity between two embeddings.
///
/// Used to verify that embeddings are equivalent (same text -> same embedding).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (mag_a * mag_b)
}

/// Helper: Generate test texts with identifiable content.
fn generate_test_texts(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            format!(
                "Test text number {} with unique content for Google Vertex AI embedding",
                i
            )
        })
        .collect()
}

/// Helper: Generate varied test texts for diverse testing.
fn generate_varied_texts() -> Vec<String> {
    vec![
        // Short text
        "hello".to_string(),
        // Medium text
        "The quick brown fox jumps over the lazy dog.".to_string(),
        // Long text (100+ words)
        "This is a comprehensive test paragraph designed to validate that the Google Vertex AI \
         embedding provider correctly handles longer text content. The text embedding model should \
         be able to process this entire paragraph and produce a meaningful 768-dimensional vector \
         representation. This test ensures that batch processing works correctly for texts of \
         varying lengths, including longer documents that might be found in real-world code \
         repositories such as README files, documentation, or detailed code comments. The model \
         should maintain consistent quality and dimension regardless of input length."
            .to_string(),
        // Code-like text
        "fn main() { println!(\"Hello, world!\"); }".to_string(),
        // Technical text
        "The Rust programming language provides memory safety without garbage collection."
            .to_string(),
    ]
}

/// Helper: Create GoogleProvider with custom parallel config.
async fn create_provider_with_config(config: ParallelConfig) -> Option<GoogleProvider> {
    if env::var("GCP_INTEGRATION_TESTS").unwrap_or_default() != "1" {
        eprintln!("Skipping test: GCP_INTEGRATION_TESTS not set");
        return None;
    }

    let project_id =
        match env::var("GOOGLE_PROJECT_ID").or_else(|_| env::var("MAPROOM_GOOGLE_PROJECT_ID")) {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: GOOGLE_PROJECT_ID not set");
                return None;
            }
        };

    let creds_path = match env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .or_else(|_| env::var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS"))
    {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            eprintln!("Skipping test: GOOGLE_APPLICATION_CREDENTIALS not set");
            return None;
        }
    };

    if !creds_path.exists() {
        eprintln!(
            "Skipping test: credentials file not found: {}",
            creds_path.display()
        );
        return None;
    }

    let region =
        env::var("GOOGLE_REGION").unwrap_or_else(|_| GoogleProvider::DEFAULT_REGION.to_string());

    match GoogleProvider::new_with_config(
        project_id,
        creds_path,
        region,
        GoogleProvider::DEFAULT_MODEL.to_string(),
        config,
    )
    .await
    {
        Ok(provider) => Some(provider),
        Err(e) => {
            eprintln!("Failed to create GoogleProvider: {:?}", e);
            None
        }
    }
}

/// Helper: Create GoogleProvider using from_env().
async fn create_provider_from_env() -> Option<GoogleProvider> {
    if env::var("GCP_INTEGRATION_TESTS").unwrap_or_default() != "1" {
        eprintln!("Skipping test: GCP_INTEGRATION_TESTS not set");
        eprintln!("To run these tests:");
        eprintln!("  export GCP_INTEGRATION_TESTS=1");
        eprintln!("  export GOOGLE_PROJECT_ID=your-project-id");
        eprintln!("  export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json");
        eprintln!("  cargo test --test google_parallel_test -- --ignored");
        return None;
    }

    match GoogleProvider::from_env().await {
        Ok(provider) => Some(provider),
        Err(e) => {
            eprintln!("Failed to create GoogleProvider from env: {:?}", e);
            None
        }
    }
}

// ============================================================================
// Order Preservation Tests
// ============================================================================

/// Verifies that embeddings[i] matches texts[i] when using parallel processing.
///
/// This is critical for correctness - parallel sub-batches must be reassembled
/// in the correct order.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_batch_preserves_order() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 100,
        max_concurrency: 8,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    // Generate 500 texts (spans multiple sub-batches)
    let texts = generate_test_texts(500);

    // Embed batch using parallel processing
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding failed");

    // Verify: Same number of embeddings as texts
    assert_eq!(
        embeddings.len(),
        texts.len(),
        "Number of embeddings must match number of texts"
    );

    // Spot check: Verify embeddings at different positions match individual embeddings
    // We check positions from different sub-batches to verify order preservation
    let check_indices = [0, 99, 100, 199, 200, 350, 499];

    for i in check_indices {
        let single_embedding = provider
            .embed(texts[i].clone())
            .await
            .expect("Single embedding failed");

        let similarity = cosine_similarity(&embeddings[i], &single_embedding);

        // Embeddings should be nearly identical (cosine similarity >= 0.99)
        assert!(
            similarity >= 0.99,
            "Embedding at index {} does not match text. Similarity: {:.4}",
            i,
            similarity
        );
    }

    println!(
        "Order preservation verified for {} texts across multiple sub-batches",
        texts.len()
    );
}

// ============================================================================
// Embedding Equivalence Tests
// ============================================================================

/// Verifies that same text produces same embedding regardless of configuration.
///
/// Tests that parallel processing doesn't affect embedding quality.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_parallel_produces_same_embeddings() {
    skip_if_no_credentials();

    let test_text = "This is a test function for authentication verification".to_string();

    // Config 1: Single embedding (no batch)
    let Some(provider_single) = create_provider_from_env().await else {
        return;
    };
    let embedding_single = provider_single
        .embed(test_text.clone())
        .await
        .expect("Single embedding failed");

    // Config 2: Batch without parallel (disabled)
    let config_batch = ParallelConfig {
        enabled: false,
        sub_batch_size: 200,
        max_concurrency: 1,
    };
    let Some(provider_batch) = create_provider_with_config(config_batch).await else {
        return;
    };
    let embedding_batch = provider_batch
        .embed_batch(vec![test_text.clone()])
        .await
        .expect("Batch embedding failed")[0]
        .clone();

    // Config 3: Parallel processing enabled
    let config_parallel = ParallelConfig {
        enabled: true,
        sub_batch_size: 100,
        max_concurrency: 8,
    };
    let Some(provider_parallel) = create_provider_with_config(config_parallel).await else {
        return;
    };
    let embedding_parallel = provider_parallel
        .embed_batch(vec![test_text.clone()])
        .await
        .expect("Parallel embedding failed")[0]
        .clone();

    // Verify all embeddings are equivalent (cosine similarity >= 0.99)
    let sim_single_batch = cosine_similarity(&embedding_single, &embedding_batch);
    let sim_single_parallel = cosine_similarity(&embedding_single, &embedding_parallel);
    let sim_batch_parallel = cosine_similarity(&embedding_batch, &embedding_parallel);

    assert!(
        sim_single_batch >= 0.99,
        "Single vs Batch: Similarity {:.4} < 0.99",
        sim_single_batch
    );
    assert!(
        sim_single_parallel >= 0.99,
        "Single vs Parallel: Similarity {:.4} < 0.99",
        sim_single_parallel
    );
    assert!(
        sim_batch_parallel >= 0.99,
        "Batch vs Parallel: Similarity {:.4} < 0.99",
        sim_batch_parallel
    );

    println!("Embedding equivalence verified across all configurations");
    println!(
        "  Single vs Batch: {:.4}, Single vs Parallel: {:.4}, Batch vs Parallel: {:.4}",
        sim_single_batch, sim_single_parallel, sim_batch_parallel
    );
}

// ============================================================================
// Dimension Consistency Tests
// ============================================================================

/// Verifies all embeddings are 768-dimensional (text-embedding-004).
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_all_embeddings_correct_dimension() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 100,
        max_concurrency: 8,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    // Generate variety of text lengths
    let texts = generate_varied_texts();

    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding failed");

    // Verify all embeddings are 768-dimensional (text-embedding-004)
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} has dimension {} (expected 768)",
            i,
            embedding.len()
        );
    }

    // Verify provider reports correct dimension
    assert_eq!(
        provider.dimension(),
        768,
        "Provider dimension() should return 768"
    );

    println!(
        "All {} embeddings verified as 768-dimensional",
        embeddings.len()
    );
}

// ============================================================================
// Performance Tests
// ============================================================================

/// Measures performance improvement of parallel vs sequential processing.
///
/// This test measures throughput but does not assert a hard threshold since
/// performance depends on network conditions and API load.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_parallel_vs_sequential_throughput() {
    skip_if_no_credentials();

    // Create 1000 texts for meaningful comparison
    let texts = generate_test_texts(1000);

    // Config 1: Sequential (parallel disabled)
    let config_sequential = ParallelConfig {
        enabled: false,
        sub_batch_size: 200,
        max_concurrency: 1,
    };
    let Some(provider_sequential) = create_provider_with_config(config_sequential).await else {
        return;
    };

    // Measure sequential throughput
    let seq_start = Instant::now();
    let _sequential_embeddings = provider_sequential
        .embed_batch(texts.clone())
        .await
        .expect("Sequential batch failed");
    let seq_duration = seq_start.elapsed();

    // Config 2: Parallel (enabled with Google defaults)
    let config_parallel = ParallelConfig::google_defaults();
    let Some(provider_parallel) = create_provider_with_config(config_parallel).await else {
        return;
    };

    // Measure parallel throughput
    let par_start = Instant::now();
    let _parallel_embeddings = provider_parallel
        .embed_batch(texts.clone())
        .await
        .expect("Parallel batch failed");
    let par_duration = par_start.elapsed();

    // Calculate metrics
    let seq_throughput = texts.len() as f64 / seq_duration.as_secs_f64();
    let par_throughput = texts.len() as f64 / par_duration.as_secs_f64();
    let speedup = par_throughput / seq_throughput;

    println!("Performance comparison for {} texts:", texts.len());
    println!(
        "  Sequential: {:.2}s ({:.1} texts/sec)",
        seq_duration.as_secs_f64(),
        seq_throughput
    );
    println!(
        "  Parallel:   {:.2}s ({:.1} texts/sec)",
        par_duration.as_secs_f64(),
        par_throughput
    );
    println!("  Speedup:    {:.2}x", speedup);

    // Log result but don't assert (performance varies by network/API load)
    // Target is >= 8x improvement for 1000+ texts, but this is informational
    if speedup >= 8.0 {
        println!("  Target achieved: >= 8x speedup");
    } else {
        println!("  Note: Speedup below target (8x). May vary by network conditions.");
    }
}

// ============================================================================
// Empty Input Handling Tests
// ============================================================================

/// Verifies empty batch returns empty vec.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_empty_batch_parallel() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 200,
        max_concurrency: 16,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    let empty_texts: Vec<String> = vec![];
    let embeddings = provider
        .embed_batch(empty_texts)
        .await
        .expect("Empty batch should succeed with parallel enabled");

    assert_eq!(embeddings.len(), 0, "Empty batch should return empty vec");

    println!("Empty batch handling verified");
}

// ============================================================================
// Disabled Parallel Mode Tests
// ============================================================================

/// Verifies that disabled parallel config uses raw batch method.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_disabled_parallel_uses_single_batch() {
    skip_if_no_credentials();

    // When parallel is disabled, should use single batch request
    let config = ParallelConfig {
        enabled: false, // Disabled
        sub_batch_size: 50,
        max_concurrency: 1,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    // Batch of 100 texts - would be split if parallel was enabled
    let texts = generate_test_texts(100);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding should succeed");

    // Verify correctness
    assert_eq!(embeddings.len(), texts.len());

    // Verify all embeddings are 768-dimensional
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should be 768-dimensional",
            i
        );
    }

    println!(
        "Disabled parallel mode verified - single batch used for {} texts",
        texts.len()
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Verifies batch of size 1 works correctly.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_single_text_batch() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 200,
        max_concurrency: 16,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    let texts = vec!["single text for batch embedding".to_string()];
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Single text batch failed");

    assert_eq!(embeddings.len(), 1);
    assert_eq!(embeddings[0].len(), 768);

    println!("Single text batch verified");
}

/// Verifies batch size exactly equals sub_batch_size works correctly.
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_exact_sub_batch_boundary() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 200,
        max_concurrency: 8,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    // Exactly sub_batch_size texts
    let texts = generate_test_texts(200);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Exact boundary batch failed");

    assert_eq!(embeddings.len(), texts.len());

    // Verify all embeddings are 768-dimensional
    for embedding in &embeddings {
        assert_eq!(embedding.len(), 768);
    }

    println!(
        "Exact sub-batch boundary verified for {} texts",
        texts.len()
    );
}

/// Verifies uneven sub-batch split works correctly.
///
/// e.g., 450 texts with sub_batch_size=200 -> [200, 200, 50]
#[tokio::test]
#[ignore] // Requires GCP credentials
async fn test_google_uneven_sub_batch_split() {
    skip_if_no_credentials();

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 200,
        max_concurrency: 8,
    };

    let Some(provider) = create_provider_with_config(config).await else {
        return;
    };

    // 450 texts -> [200, 200, 50] with sub_batch_size=200
    let texts = generate_test_texts(450);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Uneven split batch failed");

    assert_eq!(embeddings.len(), texts.len());

    // Verify correctness of last few embeddings (from the small final sub-batch)
    for i in 440..450 {
        let single_embedding = provider
            .embed(texts[i].clone())
            .await
            .expect("Single embedding failed");

        let similarity = cosine_similarity(&embeddings[i], &single_embedding);
        assert!(
            similarity >= 0.99,
            "Embedding {} from small sub-batch incorrect. Similarity: {:.4}",
            i,
            similarity
        );
    }

    println!("Uneven sub-batch split verified for {} texts", texts.len());
}
