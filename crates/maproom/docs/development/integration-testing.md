# Integration Testing Guide

This guide covers running integration tests for Maproom, with special focus on Google Cloud Vertex AI provider integration tests that require real cloud credentials.

## Overview

Maproom includes several types of integration tests:

1. **Database Integration Tests** - Require PostgreSQL connection
2. **Provider Integration Tests** - Test embedding providers with real APIs
3. **Performance Tests** - Large-scale batch processing tests
4. **End-to-End Workflow Tests** - Complete indexing and search pipelines

All integration tests that require external credentials or resources are marked with `#[ignore]` attribute to prevent accidental execution during normal test runs.

## Google Cloud Vertex AI Integration Tests

The Google provider integration tests validate end-to-end functionality with Google Cloud's Vertex AI service. These tests make real API calls and require proper authentication.

### Prerequisites

#### 1. GCP Project Setup

You need a Google Cloud project with the Vertex AI API enabled:

```bash
# Create a new project (or use existing)
gcloud projects create my-maproom-test-project --name="Maproom Testing"

# Set as default project
gcloud config set project my-maproom-test-project

# Enable Vertex AI API
gcloud services enable aiplatform.googleapis.com
```

#### 2. Service Account Creation

Create a service account with minimal required permissions:

```bash
# Create service account
gcloud iam service-accounts create maproom-test-sa \
    --display-name="Maproom Integration Test Service Account" \
    --description="Service account for running Maproom integration tests"

# Grant Vertex AI User role (least-privilege IAM role)
gcloud projects add-iam-policy-binding my-maproom-test-project \
    --member="serviceAccount:maproom-test-sa@my-maproom-test-project.iam.gserviceaccount.com" \
    --role="roles/aiplatform.user"

# Create and download JSON key file
gcloud iam service-accounts keys create ~/maproom-test-key.json \
    --iam-account=maproom-test-sa@my-maproom-test-project.iam.gserviceaccount.com

# Secure the key file (important!)
chmod 600 ~/maproom-test-key.json
```

#### 3. IAM Permissions Reference

The `roles/aiplatform.user` role includes these permissions:
- `aiplatform.endpoints.predict` - Required for embedding generation
- `aiplatform.endpoints.get` - Required for endpoint discovery
- `aiplatform.models.predict` - Required for model prediction

This is the minimum required role for the integration tests. Do NOT use `roles/owner` or `roles/editor` as they grant excessive permissions.

### Environment Configuration

Set up environment variables for the integration tests:

```bash
# Required: Enable Google integration tests
export GCP_INTEGRATION_TESTS=1

# Required: Your GCP project ID
export GOOGLE_PROJECT_ID=my-maproom-test-project

# Required: Path to service account JSON key
export GOOGLE_APPLICATION_CREDENTIALS=~/maproom-test-key.json

# Optional: Override default region (defaults to us-central1)
export GOOGLE_REGION=us-central1

# Optional: Override model (defaults to textembedding-gecko@003)
export GOOGLE_MODEL=textembedding-gecko@003
```

You can also create a `.env.test` file in the maproom directory:

```bash
# crates/maproom/.env.test
GCP_INTEGRATION_TESTS=1
GOOGLE_PROJECT_ID=my-maproom-test-project
GOOGLE_APPLICATION_CREDENTIALS=/home/username/maproom-test-key.json
GOOGLE_REGION=us-central1
```

Then load it before running tests:

```bash
export $(cat crates/maproom/.env.test | xargs)
```

### Running the Tests

#### Run All Google Integration Tests

```bash
cargo test --test google_provider_integration -- --ignored
```

#### Run Specific Test

```bash
# Single embedding test
cargo test --test google_provider_integration test_google_provider_single_embed -- --ignored

# Batch embedding test
cargo test --test google_provider_integration test_google_provider_batch_embed -- --ignored

# Dimension verification
cargo test --test google_provider_integration test_google_provider_verify_768_dimensions -- --ignored

# Invalid credentials test (doesn't require real GCP)
cargo test --test google_provider_integration test_google_provider_invalid_credentials -- --ignored

# Regional endpoint test
cargo test --test google_provider_integration test_google_provider_regional_endpoint_us_central1 -- --ignored
```

#### Run with Verbose Output

```bash
cargo test --test google_provider_integration -- --ignored --nocapture
```

This shows detailed output including:
- Embedding dimensions
- Sum of embedding values (to verify not all zeros)
- Max absolute values
- Batch processing details

### Test Coverage

The Google integration test suite includes:

1. **test_google_provider_single_embed**
   - Generates embedding for single text
   - Verifies 768-dimensional output
   - Checks embeddings are not all zeros
   - Validates reasonable value ranges

2. **test_google_provider_batch_embed**
   - Processes batch of 10 texts
   - Verifies all embeddings are 768-dimensional
   - Checks embeddings differ for different texts
   - Tests native batch processing API

3. **test_google_provider_verify_768_dimensions**
   - Confirms dimension() method returns 768
   - Verifies actual embeddings match reported dimension
   - Ensures consistency with database schema

4. **test_google_provider_invalid_credentials**
   - Tests authentication failure handling
   - Creates fake service account credentials
   - Verifies appropriate error messages
   - Confirms 401/403 errors are caught

5. **test_google_provider_regional_endpoint_us_central1**
   - Tests explicit region configuration
   - Verifies us-central1 endpoint works
   - Validates regional endpoint URL construction

6. **test_google_provider_task_type_configuration**
   - Tests RETRIEVAL_DOCUMENT task type
   - Tests RETRIEVAL_QUERY task type
   - Tests SEMANTIC_SIMILARITY task type
   - Verifies all produce 768-dimensional embeddings

7. **test_google_provider_database_persistence_code_embedding** (TODO)
   - Will test persistence to code_embedding_ollama column
   - Awaiting MPEMBED-4004 completion

8. **test_google_provider_database_persistence_doc_embedding** (TODO)
   - Will test persistence to doc_embedding_ollama column
   - Awaiting MPEMBED-4004 completion

9. **test_google_provider_empty_batch**
   - Tests handling of empty input
   - Verifies no API calls made
   - Checks graceful return

10. **test_google_provider_batch_size_limit**
    - Tests enforcement of 250-text limit
    - Verifies appropriate error message
    - Ensures quota protection

11. **test_google_provider_idempotency**
    - Generates embeddings for same text twice
    - Verifies embeddings are identical
    - Tests deterministic behavior

### Expected Behavior

When tests run successfully, you should see output like:

```
running 11 tests
test test_google_provider_single_embed ... ok
✓ Single embedding generated successfully
  Dimension: 768
  Sum: 123.4567
  Max absolute value: 2.3456

test test_google_provider_batch_embed ... ok
✓ Batch embedding generated successfully
  Batch size: 10
  Dimension per embedding: 768
  Difference between first two embeddings: 45.6789

test test_google_provider_verify_768_dimensions ... ok
✓ 768-dimensional output verified
  Provider dimension(): 768
  Actual embedding length: 768

... (additional tests)

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Troubleshooting

#### "Skipping Google integration test - GCP_INTEGRATION_TESTS not set"

The environment variable `GCP_INTEGRATION_TESTS` is not set. Export it:

```bash
export GCP_INTEGRATION_TESTS=1
```

#### "Skipping test - GOOGLE_PROJECT_ID not set"

Set your GCP project ID:

```bash
export GOOGLE_PROJECT_ID=your-project-id
```

#### "Skipping test - GOOGLE_APPLICATION_CREDENTIALS not set"

Set the path to your service account key file:

```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
```

#### "credentials file not found"

Verify the file exists at the path specified:

```bash
ls -la $GOOGLE_APPLICATION_CREDENTIALS
```

#### "Failed to create GoogleProvider"

Check the credentials file is valid JSON:

```bash
cat $GOOGLE_APPLICATION_CREDENTIALS | jq .
```

#### "Invalid credentials or expired token" / "401 Unauthorized"

Common causes:
- Service account doesn't have `roles/aiplatform.user` role
- Service account key has been deleted or revoked
- Wrong project ID in environment variable
- Credentials file corrupted

Verify IAM role assignment:

```bash
gcloud projects get-iam-policy $GOOGLE_PROJECT_ID \
    --flatten="bindings[].members" \
    --filter="bindings.role:roles/aiplatform.user"
```

#### "Insufficient IAM permissions" / "403 Forbidden"

The service account needs the `roles/aiplatform.user` role. Grant it:

```bash
gcloud projects add-iam-policy-binding $GOOGLE_PROJECT_ID \
    --member="serviceAccount:SA_EMAIL" \
    --role="roles/aiplatform.user"
```

Replace `SA_EMAIL` with your service account email (from the JSON key file).

#### "Vertex AI API not enabled"

Enable the API:

```bash
gcloud services enable aiplatform.googleapis.com --project=$GOOGLE_PROJECT_ID
```

#### "Batch size exceeds maximum of 250"

The test is working correctly - this validates the batch size limit enforcement.

### Cost Considerations

Each integration test run makes real API calls to Google Vertex AI, which incurs costs:

- **textembedding-gecko@003 pricing**: ~$0.00025 per 1,000 characters
- **Typical test run**: ~20-30 embedding requests
- **Average test run cost**: < $0.01 USD

To minimize costs:
1. Run tests only when necessary (not on every commit)
2. Use a dedicated test project with budget alerts
3. Set up billing alerts for the test project
4. Clean up old service accounts and keys

Set up a budget alert:

```bash
# Set budget alert at $5/month
gcloud billing budgets create \
    --billing-account=BILLING_ACCOUNT_ID \
    --display-name="Maproom Test Budget" \
    --budget-amount=5.00 \
    --threshold-rule=percent=50 \
    --threshold-rule=percent=90 \
    --threshold-rule=percent=100
```

### Security Best Practices

1. **Never commit service account keys to git**
   - Add `*.json` to `.gitignore` for credentials
   - Use environment variables or secure vaults

2. **Use least-privilege IAM roles**
   - Use `roles/aiplatform.user`, not `roles/owner`
   - Create dedicated service accounts per application

3. **Rotate service account keys regularly**
   - Set calendar reminder for quarterly rotation
   - Delete old keys after creating new ones

4. **Set appropriate file permissions**
   - Credentials file should be `chmod 600` (owner read/write only)
   - Never share credentials via insecure channels

5. **Use separate projects for testing**
   - Isolate test resources from production
   - Makes cleanup and cost tracking easier

6. **Monitor API usage**
   - Set up Cloud Monitoring alerts
   - Review API usage in Cloud Console regularly

### CI/CD Integration

For GitHub Actions or other CI systems:

#### GitHub Actions Example

```yaml
# .github/workflows/google-integration-tests.yml
name: Google Integration Tests

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * 1' # Weekly on Monday at 2 AM

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Write credentials file
        env:
          GCP_SA_KEY: ${{ secrets.GCP_SERVICE_ACCOUNT_KEY }}
        run: |
          echo "$GCP_SA_KEY" > /tmp/gcp-key.json
          chmod 600 /tmp/gcp-key.json

      - name: Run Google integration tests
        env:
          GCP_INTEGRATION_TESTS: 1
          GOOGLE_PROJECT_ID: ${{ secrets.GCP_TEST_PROJECT_ID }}
          GOOGLE_APPLICATION_CREDENTIALS: /tmp/gcp-key.json
        run: |
          cargo test --test google_provider_integration -- --ignored --nocapture

      - name: Clean up credentials
        if: always()
        run: |
          rm -f /tmp/gcp-key.json
```

#### Required GitHub Secrets

- `GCP_SERVICE_ACCOUNT_KEY`: Full JSON content of service account key
- `GCP_TEST_PROJECT_ID`: GCP project ID

### Database Persistence Tests

The database persistence tests (currently TODO) will require:

1. PostgreSQL connection with vector extension
2. Maproom database schema applied
3. Test data cleanup after execution

These will be implemented in Phase 4 (MPEMBED-4004) and will test:
- Storing 768-dimensional embeddings in `code_embedding_ollama` column
- Storing 768-dimensional embeddings in `doc_embedding_ollama` column
- Querying and retrieving persisted embeddings
- Verifying dimension constraints in database

## Other Integration Tests

### Ollama Integration Tests

For local Ollama embedding provider tests:

```bash
# Start Ollama (if using Docker)
docker-compose up -d ollama

# Pull the model
ollama pull nomic-embed-text

# Run tests
cargo test --test ollama_integration_test
```

### Database Integration Tests

Most integration tests require PostgreSQL:

```bash
# Set database URL
export MAPROOM_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/maproom_test

# Run all integration tests
cargo test --test '*'
```

## Summary

The integration test suite ensures Maproom works correctly with real cloud services. The Google Vertex AI integration tests specifically validate:

- ✅ Authentication with service account credentials
- ✅ 768-dimensional embedding generation
- ✅ Batch processing up to 250 texts
- ✅ Regional endpoint configuration
- ✅ Task type optimization
- ✅ Error handling for invalid credentials
- ✅ Idempotent behavior
- ⏳ Database persistence (coming in Phase 4)

Follow this guide to run tests safely, securely, and cost-effectively.
