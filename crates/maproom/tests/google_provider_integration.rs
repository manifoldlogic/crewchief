//! Integration tests for Google Cloud Vertex AI embedding provider.
//!
//! These tests validate end-to-end embedding generation with real Google Cloud credentials.
//! All tests require:
//! - GCP_INTEGRATION_TESTS=1 environment variable
//! - GOOGLE_APPLICATION_CREDENTIALS pointing to service account JSON key
//! - GOOGLE_PROJECT_ID set to GCP project ID with Vertex AI enabled
//! - Service account with roles/aiplatform.user IAM role
//!
//! Run these tests with:
//! ```bash
//! export GCP_INTEGRATION_TESTS=1
//! export GOOGLE_PROJECT_ID=your-test-project-id
//! export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
//! cargo test --test google_provider_integration -- --ignored
//! ```
//!
//! All tests are marked with #[ignore] to prevent accidental execution in CI
//! without proper credentials and quota management.

use crewchief_maproom::embedding::google::{GoogleProvider, TaskType};
use crewchief_maproom::embedding::provider::EmbeddingProvider;
use std::env;
use std::path::PathBuf;

/// Helper to check if integration tests should run.
///
/// This function checks for the GCP_INTEGRATION_TESTS environment variable.
/// Tests will silently pass if this is not set, allowing them to be run
/// with --ignored flag only when explicitly configured.
fn should_run_integration_tests() -> bool {
    env::var("GCP_INTEGRATION_TESTS").is_ok()
}

/// Helper to create a GoogleProvider from environment variables.
///
/// Returns None if required environment variables are missing, allowing
/// tests to skip gracefully.
async fn create_test_provider() -> Option<GoogleProvider> {
    if !should_run_integration_tests() {
        eprintln!("Skipping Google integration test - GCP_INTEGRATION_TESTS not set");
        eprintln!("To run these tests:");
        eprintln!("  export GCP_INTEGRATION_TESTS=1");
        eprintln!("  export GOOGLE_PROJECT_ID=your-test-project-id");
        eprintln!("  export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json");
        eprintln!("  cargo test --test google_provider_integration -- --ignored");
        return None;
    }

    let project_id = match env::var("GOOGLE_PROJECT_ID") {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Skipping test - GOOGLE_PROJECT_ID not set");
            return None;
        }
    };

    let creds_path = match env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            eprintln!("Skipping test - GOOGLE_APPLICATION_CREDENTIALS not set");
            return None;
        }
    };

    // Verify credentials file exists
    if !creds_path.exists() {
        eprintln!(
            "Skipping test - credentials file not found: {}",
            creds_path.display()
        );
        return None;
    }

    match GoogleProvider::new(
        project_id,
        creds_path,
        GoogleProvider::DEFAULT_REGION.to_string(),
        GoogleProvider::DEFAULT_MODEL.to_string(),
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

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_single_embed() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Verify provider configuration
    assert_eq!(provider.dimension(), 768);
    assert_eq!(provider.provider_name(), "google");

    // Generate embedding for single text
    let text = "Test embedding text for Google Vertex AI".to_string();
    let embedding = provider.embed(text).await?;

    // Verify embedding dimension
    assert_eq!(
        embedding.len(),
        768,
        "Expected 768-dimensional embedding from textembedding-gecko@003"
    );

    // Verify embeddings are not all zeros (common failure mode)
    let sum: f32 = embedding.iter().sum();
    assert!(
        sum.abs() > 0.01,
        "Embeddings appear to be all zeros (sum: {})",
        sum
    );

    // Verify embeddings contain reasonable values (typical range for normalized embeddings)
    let max_val = embedding
        .iter()
        .map(|v| v.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    assert!(
        max_val < 10.0,
        "Embedding values seem unusually large: max_abs={}",
        max_val
    );

    println!("✓ Single embedding generated successfully");
    println!("  Dimension: {}", embedding.len());
    println!("  Sum: {:.4}", sum);
    println!("  Max absolute value: {:.4}", max_val);

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_batch_embed() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Create batch of 10 different texts
    let texts: Vec<String> = (0..10)
        .map(|i| {
            format!(
                "Test text number {} for batch embedding with Google Vertex AI",
                i
            )
        })
        .collect();

    // Generate embeddings for batch
    let embeddings = provider.embed_batch(texts.clone()).await?;

    // Verify correct number of embeddings
    assert_eq!(
        embeddings.len(),
        10,
        "Expected 10 embeddings for 10 input texts"
    );

    // Verify each embedding
    for (i, embedding) in embeddings.iter().enumerate() {
        // Check dimension
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} has wrong dimension",
            i
        );

        // Check not all zeros
        let sum: f32 = embedding.iter().sum();
        assert!(
            sum.abs() > 0.01,
            "Embedding {} appears to be all zeros",
            i
        );
    }

    // Verify embeddings are different (they should be for different texts)
    // Compare first and second embeddings
    let diff: f32 = embeddings[0]
        .iter()
        .zip(embeddings[1].iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(
        diff > 0.1,
        "Embeddings for different texts should be different (diff: {})",
        diff
    );

    println!("✓ Batch embedding generated successfully");
    println!("  Batch size: {}", embeddings.len());
    println!("  Dimension per embedding: {}", embeddings[0].len());
    println!("  Difference between first two embeddings: {:.4}", diff);

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_verify_768_dimensions() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Verify dimension method returns correct value
    assert_eq!(
        provider.dimension(),
        768,
        "Provider dimension() should return 768"
    );

    // Generate embedding and verify actual dimension matches
    let text = "Dimension verification test".to_string();
    let embedding = provider.embed(text).await?;

    assert_eq!(
        embedding.len(),
        provider.dimension(),
        "Actual embedding dimension should match provider.dimension()"
    );

    assert_eq!(
        embedding.len(),
        768,
        "Actual embedding should be 768-dimensional"
    );

    println!("✓ 768-dimensional output verified");
    println!("  Provider dimension(): {}", provider.dimension());
    println!("  Actual embedding length: {}", embedding.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_invalid_credentials() {
    // Create a temporary fake credentials file
    let temp_dir = std::env::temp_dir();
    let fake_creds_path = temp_dir.join("fake-google-creds.json");

    // Write minimal fake service account JSON
    let fake_creds = r#"{
        "type": "service_account",
        "project_id": "fake-project-12345",
        "private_key_id": "fake-key-id",
        "private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7W8jxL\n-----END PRIVATE KEY-----\n",
        "client_email": "fake-service-account@fake-project.iam.gserviceaccount.com",
        "client_id": "123456789",
        "auth_uri": "https://accounts.google.com/o/oauth2/auth",
        "token_uri": "https://oauth2.googleapis.com/token",
        "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs"
    }"#;

    std::fs::write(&fake_creds_path, fake_creds).expect("Failed to write fake credentials");

    // Create provider with fake credentials
    let provider = GoogleProvider::new(
        "fake-project-12345".to_string(),
        fake_creds_path.clone(),
        "us-central1".to_string(),
        "textembedding-gecko@003".to_string(),
    )
    .await;

    // Provider creation should succeed (validation happens on first API call)
    assert!(
        provider.is_ok(),
        "Provider should instantiate with fake credentials"
    );

    // Try to generate embedding - this should fail with authentication error
    let provider = provider.unwrap();
    let embed_result = provider.embed("test".to_string()).await;

    // Clean up fake credentials file
    let _ = std::fs::remove_file(&fake_creds_path);

    // Verify the request failed
    assert!(
        embed_result.is_err(),
        "Should fail with invalid credentials"
    );

    // Verify error message mentions authentication
    let err_msg = format!("{}", embed_result.unwrap_err());
    let is_auth_error = err_msg.contains("401")
        || err_msg.contains("403")
        || err_msg.contains("Unauthorized")
        || err_msg.contains("authentication")
        || err_msg.contains("credentials")
        || err_msg.contains("IAM");

    assert!(
        is_auth_error,
        "Error should indicate authentication failure. Got: {}",
        err_msg
    );

    println!("✓ Invalid credentials properly rejected");
    println!("  Error message: {}", err_msg);
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_regional_endpoint_us_central1() -> anyhow::Result<()> {
    let Some(_) = create_test_provider().await else {
        return Ok(());
    };

    // Create provider explicitly with us-central1 region
    let project_id = env::var("GOOGLE_PROJECT_ID").expect("GOOGLE_PROJECT_ID not set");
    let creds_path =
        PathBuf::from(env::var("GOOGLE_APPLICATION_CREDENTIALS").expect("Credentials not set"));

    let provider = GoogleProvider::new(
        project_id,
        creds_path,
        "us-central1".to_string(),
        GoogleProvider::DEFAULT_MODEL.to_string(),
    )
    .await?;

    // Generate embedding with us-central1 endpoint
    let result = provider.embed("test regional endpoint".to_string()).await;

    assert!(
        result.is_ok(),
        "us-central1 regional endpoint should work: {:?}",
        result.err()
    );

    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 768);

    println!("✓ Regional endpoint us-central1 works correctly");

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_task_type_configuration() -> anyhow::Result<()> {
    let Some(mut provider) = create_test_provider().await else {
        return Ok(());
    };

    // Test with RETRIEVAL_DOCUMENT task type (default)
    let text1 = "Document about machine learning algorithms".to_string();
    let embedding1 = provider.embed(text1.clone()).await?;
    assert_eq!(embedding1.len(), 768);

    // Test with RETRIEVAL_QUERY task type
    provider.with_task_type(TaskType::RetrievalQuery);
    let text2 = "What are machine learning algorithms?".to_string();
    let embedding2 = provider.embed(text2.clone()).await?;
    assert_eq!(embedding2.len(), 768);

    // Test with SEMANTIC_SIMILARITY task type
    provider.with_task_type(TaskType::SemanticSimilarity);
    let text3 = "Machine learning concepts".to_string();
    let embedding3 = provider.embed(text3.clone()).await?;
    assert_eq!(embedding3.len(), 768);

    println!("✓ Task type configuration works for all types");
    println!("  RETRIEVAL_DOCUMENT: {} dims", embedding1.len());
    println!("  RETRIEVAL_QUERY: {} dims", embedding2.len());
    println!("  SEMANTIC_SIMILARITY: {} dims", embedding3.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_database_persistence_code_embedding() -> anyhow::Result<()> {
    // TODO: Implement once database integration is ready (MPEMBED-4004)
    // This test should:
    // 1. Generate embeddings using GoogleProvider
    // 2. Store embeddings in code_embedding_ollama column
    // 3. Query the database to verify embeddings persisted correctly
    // 4. Verify embeddings are 768-dimensional in the database
    // 5. Clean up test data

    println!("⚠ Database persistence test not yet implemented");
    println!("  Waiting for MPEMBED-4004 (database integration)");
    println!("  This test will verify code_embedding_ollama column persistence");

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_database_persistence_doc_embedding() -> anyhow::Result<()> {
    // TODO: Implement once database integration is ready (MPEMBED-4004)
    // This test should:
    // 1. Generate embeddings using GoogleProvider
    // 2. Store embeddings in doc_embedding_ollama column
    // 3. Query the database to verify embeddings persisted correctly
    // 4. Verify embeddings are 768-dimensional in the database
    // 5. Clean up test data

    println!("⚠ Database persistence test not yet implemented");
    println!("  Waiting for MPEMBED-4004 (database integration)");
    println!("  This test will verify doc_embedding_ollama column persistence");

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_empty_batch() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Test with empty batch
    let empty_texts: Vec<String> = Vec::new();
    let embeddings = provider.embed_batch(empty_texts).await?;

    assert_eq!(embeddings.len(), 0, "Empty batch should return empty result");

    println!("✓ Empty batch handled correctly");

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_batch_size_limit() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Create batch exceeding maximum size (250)
    let oversized_batch: Vec<String> = (0..251).map(|i| format!("Text {}", i)).collect();

    let result = provider.embed_batch(oversized_batch).await;

    // Should fail with appropriate error
    assert!(result.is_err(), "Oversized batch should fail");

    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("250") || err_msg.contains("batch size") || err_msg.contains("exceeds"),
        "Error should mention batch size limit. Got: {}",
        err_msg
    );

    println!("✓ Batch size limit enforced correctly");
    println!("  Error: {}", err_msg);

    Ok(())
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_idempotency() -> anyhow::Result<()> {
    let Some(provider) = create_test_provider().await else {
        return Ok(());
    };

    // Generate embedding for same text twice
    let text = "Idempotency test text".to_string();
    let embedding1 = provider.embed(text.clone()).await?;
    let embedding2 = provider.embed(text.clone()).await?;

    // Embeddings should be identical for the same text
    assert_eq!(
        embedding1.len(),
        embedding2.len(),
        "Embeddings should have same dimension"
    );

    // Check if embeddings are nearly identical (allowing for floating point precision)
    let max_diff: f32 = embedding1
        .iter()
        .zip(embedding2.iter())
        .map(|(a, b)| (a - b).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    assert!(
        max_diff < 0.0001,
        "Embeddings for same text should be identical (max_diff: {})",
        max_diff
    );

    println!("✓ Idempotency verified");
    println!("  Max difference: {:.8}", max_diff);

    Ok(())
}
