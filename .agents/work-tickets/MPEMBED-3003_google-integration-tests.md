# Ticket: MPEMBED-3003: Google provider integration tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- google-cloud-integration-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Create integration tests for GoogleProvider that use a real GCP project and service account credentials. Verify 768-dimensional embeddings are generated and persisted to the correct *_ollama columns. Test IAM permission errors and regional endpoint behavior.

## Background
This ticket implements integration testing for the Google Vertex AI provider to ensure it works correctly with real GCP infrastructure. Unlike unit tests, these tests require actual GCP credentials and will be marked with #[ignore] to run only in CI environments or when explicitly requested. The tests verify end-to-end functionality including authentication, API calls, and database persistence.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-3-google-vertex-ai.md

## Acceptance Criteria
- [ ] Integration test file created with #[ignore] attribute
- [ ] Test: embed single text with GoogleProvider
- [ ] Test: embed batch of 10 texts
- [ ] Test: verify 768-dimensional output
- [ ] Test: verify embeddings persist to code_embedding_ollama column
- [ ] Test: verify embeddings persist to doc_embedding_ollama column
- [ ] Test: test with invalid service account (expect auth error)
- [ ] Test: test regional endpoint switching (us-central1 vs us-west1)
- [ ] CI configuration includes GCP_INTEGRATION_TESTS=1 flag
- [ ] README section documents how to run integration tests locally

## Technical Requirements
- Use test GCP project with Vertex AI API enabled
- Use CI service account with minimal IAM permissions (roles/aiplatform.user)
- Store service account key securely in GitHub Secrets or CI vault
- Tests must clean up any created resources
- Tests must be idempotent (can run multiple times safely)
- Use tokio test runtime for async tests
- Assert embedding dimensions match provider.dimension()
- Verify database persistence by querying chunks table after embedding

## Implementation Notes
**Test Structure:**
```rust
// crates/maproom/tests/google_provider_integration.rs
#![cfg(test)]

use maproom::embedding::google::GoogleProvider;
use maproom::embedding::EmbeddingProvider;
use std::env;
use std::path::PathBuf;

// Helper to check if integration tests should run
fn should_run_integration_tests() -> bool {
    env::var("GCP_INTEGRATION_TESTS").is_ok()
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_google_provider_single_embed() -> anyhow::Result<()> {
    if !should_run_integration_tests() {
        return Ok(());
    }

    let project_id = env::var("GOOGLE_PROJECT_ID")?;
    let creds_path = PathBuf::from(env::var("GOOGLE_APPLICATION_CREDENTIALS")?);

    let provider = GoogleProvider::new(project_id, creds_path)?;
    assert_eq!(provider.dimension(), 768);

    let texts = vec!["Test embedding text".to_string()];
    let embeddings = provider.embed(texts).await?;

    assert_eq!(embeddings.len(), 1);
    assert_eq!(embeddings[0].len(), 768);

    // Verify values are not all zeros
    let sum: f32 = embeddings[0].iter().sum();
    assert!(sum.abs() > 0.01, "Embeddings appear to be all zeros");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_google_provider_batch_embed() -> anyhow::Result<()> {
    if !should_run_integration_tests() {
        return Ok(());
    }

    let project_id = env::var("GOOGLE_PROJECT_ID")?;
    let creds_path = PathBuf::from(env::var("GOOGLE_APPLICATION_CREDENTIALS")?);

    let provider = GoogleProvider::new(project_id, creds_path)?;

    let texts: Vec<String> = (0..10)
        .map(|i| format!("Test text number {}", i))
        .collect();

    let embeddings = provider.embed(texts.clone()).await?;

    assert_eq!(embeddings.len(), 10);
    for (i, emb) in embeddings.iter().enumerate() {
        assert_eq!(emb.len(), 768, "Embedding {} has wrong dimension", i);
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_google_provider_database_persistence() -> anyhow::Result<()> {
    if !should_run_integration_tests() {
        return Ok(());
    }

    // This test requires database connection
    // TODO: Implement once database integration is ready (MPEMBED-4004)

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_google_provider_invalid_credentials() {
    let project_id = "fake-project".to_string();
    let creds_path = PathBuf::from("/tmp/fake-creds.json");

    // Write fake credentials file
    std::fs::write(&creds_path, r#"{"type":"service_account","project_id":"fake","private_key":"fake","client_email":"fake@fake.com"}"#).unwrap();

    let result = GoogleProvider::new(project_id, creds_path);
    // Should fail during authentication, not instantiation
    assert!(result.is_ok(), "Provider should instantiate with fake creds");

    let provider = result.unwrap();
    let embed_result = provider.embed(vec!["test".to_string()]).await;
    assert!(embed_result.is_err(), "Should fail with invalid credentials");

    let err_msg = format!("{}", embed_result.unwrap_err());
    assert!(err_msg.contains("401") || err_msg.contains("Unauthorized") || err_msg.contains("authentication"));
}

#[tokio::test]
#[ignore]
async fn test_google_provider_regional_endpoint() -> anyhow::Result<()> {
    if !should_run_integration_tests() {
        return Ok(());
    }

    // Test that regional endpoints work (some models only in certain regions)
    let project_id = env::var("GOOGLE_PROJECT_ID")?;
    let creds_path = PathBuf::from(env::var("GOOGLE_APPLICATION_CREDENTIALS")?);

    // Test us-central1 (default)
    let provider = GoogleProvider::new(project_id.clone(), creds_path.clone())?;
    let result = provider.embed(vec!["test".to_string()]).await;
    assert!(result.is_ok(), "us-central1 should work");

    Ok(())
}
```

**CI Configuration (GitHub Actions):**
```yaml
# .github/workflows/rust-tests.yml
- name: Run Google integration tests
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  env:
    GCP_INTEGRATION_TESTS: "1"
    GOOGLE_PROJECT_ID: ${{ secrets.GCP_TEST_PROJECT_ID }}
    GOOGLE_APPLICATION_CREDENTIALS: /tmp/gcp-key.json
  run: |
    echo "${{ secrets.GCP_SERVICE_ACCOUNT_KEY }}" > /tmp/gcp-key.json
    cargo test --ignored google_provider_integration
    rm /tmp/gcp-key.json
```

**Local Testing Instructions:**
```bash
# Set up environment
export GCP_INTEGRATION_TESTS=1
export GOOGLE_PROJECT_ID=your-test-project-id
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json

# Run integration tests
cargo test --ignored google_provider_integration
```

## Dependencies
- MPEMBED-3001 (GoogleProvider implementation must exist)
- External: GCP test project with Vertex AI API enabled
- External: CI service account with roles/aiplatform.user
- External: GitHub Secrets or CI vault for service account key

## Risk Assessment
- **Risk**: Integration tests may fail due to GCP quota limits in CI
  - **Mitigation**: Run tests only on main branch, use separate test project with dedicated quotas
- **Risk**: Service account credentials in CI may expire or be rotated
  - **Mitigation**: Document key rotation procedure, use long-lived service accounts (not user accounts)
- **Risk**: Tests may incur GCP costs
  - **Mitigation**: Use minimal test data, set budget alerts on test project
- **Risk**: Parallel test execution may hit rate limits
  - **Mitigation**: Run Google integration tests serially, separate from other test suites

## Files/Packages Affected
- crates/maproom/tests/google_provider_integration.rs (create)
- .github/workflows/rust-tests.yml (modify - add GCP integration test step)
- README.md (modify - add integration test instructions)
- docs/development/integration-testing.md (create)
