# Ticket: MPEMBED-3001: Implement GoogleProvider for Vertex AI embeddings

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- google-cloud-integration-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement GoogleProvider struct that calls Google Cloud Vertex AI's text embedding predict endpoint (REST API) to generate 768-dimensional embeddings using service account authentication.

## Background
This ticket implements Phase 3 (Google Vertex AI Integration) from the MPEMBED multi-provider embeddings plan. Google Vertex AI provides enterprise-grade embeddings with strong privacy and compliance guarantees, making it suitable for organizations already using GCP infrastructure. This provider uses the textembedding-gecko model which outputs 768-dimensional vectors that will be stored in the new *_ollama columns created in Phase 1.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-3-google-vertex-ai.md

## Acceptance Criteria
- [x] GoogleProvider struct implements EmbeddingProvider trait
- [x] Service account JSON key authentication working
- [x] Regional endpoint support (us-central1 as default)
- [x] Task type parameter support (RETRIEVAL_DOCUMENT, RETRIEVAL_QUERY)
- [x] Native batch embedding support (up to 250 texts per request)
- [x] 768-dimensional embedding output verified
- [x] Error handling for API errors (auth, quota, network)
- [x] Rate limiting and retry logic implemented
- [x] Unit tests with mocked HTTP responses

## Technical Requirements
- Use reqwest crate for HTTP client with async support
- Parse GOOGLE_APPLICATION_CREDENTIALS env var for service account JSON path
- Implement OAuth 2.0 JWT bearer token authentication flow
- Support configurable regional endpoints (default: us-central1-aiplatform.googleapis.com)
- Use textembedding-gecko@003 model by default
- Batch API requests: POST to /v1/projects/{project}/locations/{region}/publishers/google/models/{model}:predict
- Response parsing: extract embeddings.values array (768 floats)
- Implement exponential backoff for transient errors (429, 503)
- Timeout: 30s per request, 90s for batch requests

## Implementation Notes
**File Structure:**
```rust
// crates/maproom/src/embedding/google.rs
pub struct GoogleProvider {
    client: reqwest::Client,
    project_id: String,
    region: String,
    model: String,
    credentials: ServiceAccountKey,
}

impl GoogleProvider {
    pub fn new(project_id: String, credentials_path: PathBuf) -> Result<Self>;
    async fn get_access_token(&self) -> Result<String>;
    async fn predict(&self, texts: Vec<String>, task_type: TaskType) -> Result<Vec<Vec<f32>>>;
}

#[async_trait]
impl EmbeddingProvider for GoogleProvider {
    fn name(&self) -> &str { "google" }
    fn dimension(&self) -> usize { 768 }
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}
```

**Authentication Flow:**
1. Read service account JSON from GOOGLE_APPLICATION_CREDENTIALS path
2. Create JWT with aud="https://oauth2.googleapis.com/token"
3. Sign JWT with private key from service account
4. Exchange JWT for access token (1h TTL)
5. Cache token and refresh before expiry

**API Request Example:**
```json
POST https://us-central1-aiplatform.googleapis.com/v1/projects/{project}/locations/us-central1/publishers/google/models/textembedding-gecko@003:predict
Authorization: Bearer {access_token}

{
  "instances": [
    {"content": "text to embed", "task_type": "RETRIEVAL_DOCUMENT"}
  ]
}
```

**Error Handling:**
- 401 Unauthorized → Invalid credentials or expired token
- 403 Forbidden → Insufficient IAM permissions
- 429 Too Many Requests → Rate limit, exponential backoff
- 503 Service Unavailable → Transient error, retry

## Dependencies
- MPEMBED-2001 (EmbeddingProvider trait must be defined)
- External: GOOGLE_APPLICATION_CREDENTIALS environment variable
- External: GCP project with Vertex AI API enabled
- External: Service account with roles/aiplatform.user IAM role

## Risk Assessment
- **Risk**: Service account credentials management complexity
  - **Mitigation**: Provide clear documentation, validate credentials early with helpful error messages
- **Risk**: GCP API quota limits during bulk indexing
  - **Mitigation**: Implement rate limiting (60 req/min default), respect Retry-After headers
- **Risk**: Regional endpoint availability varies by GCP region
  - **Mitigation**: Default to us-central1 (most stable), allow configuration override
- **Risk**: JWT signing complexity may introduce bugs
  - **Mitigation**: Use well-tested jsonwebtoken crate, add comprehensive unit tests

## Files/Packages Affected
- crates/maproom/src/embedding/google.rs (create)
- crates/maproom/src/embedding/mod.rs (modify - add pub mod google)
- crates/maproom/Cargo.toml (add dependencies: reqwest, jsonwebtoken, serde_json)
- crates/maproom/tests/unit/google_provider_test.rs (create)
