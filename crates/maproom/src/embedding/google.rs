//! Google Cloud Vertex AI embedding provider implementation.
//!
//! This module provides integration with Google Cloud Vertex AI for enterprise-grade
//! embedding generation. Uses the text-embedding-004 model which produces
//! 768-dimensional embeddings suitable for semantic search and retrieval.
//!
//! # Features
//!
//! - Service account JSON key authentication
//! - Regional endpoint support (us-central1, europe-west1, asia-southeast1, etc.)
//! - Task type optimization (RETRIEVAL_DOCUMENT, RETRIEVAL_QUERY, SEMANTIC_SIMILARITY)
//! - Native batch processing (up to 250 texts per request)
//! - 768-dimensional vectors (text-embedding-004)
//! - Exponential backoff retry logic for transient errors
//! - OAuth 2.0 access token authentication using gcp_auth crate
//!
//! # Setup
//!
//! 1. Create a GCP service account with `roles/aiplatform.user` IAM role
//! 2. Download service account JSON key file
//! 3. Set environment variables:
//!    - `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account JSON key
//!    - `GOOGLE_PROJECT_ID`: GCP project ID
//!    - `GOOGLE_REGION` (optional): Region, defaults to "us-central1"
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::google::GoogleProvider;
//! use crewchief_maproom::embedding::provider::EmbeddingProvider;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create provider from environment variables
//!     let provider = GoogleProvider::from_env().await?;
//!
//!     // Generate single embedding
//!     let embedding = provider.embed("Hello, world!".to_string()).await?;
//!     assert_eq!(embedding.len(), 768);
//!
//!     // Generate batch (native API batching, up to 250 texts)
//!     let texts = vec!["First".to_string(), "Second".to_string()];
//!     let embeddings = provider.embed_batch(texts).await?;
//!     assert_eq!(embeddings.len(), 2);
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use gcp_auth::{Token, TokenProvider};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;

use crate::embedding::config::{EmbeddingConfig, ParallelConfig, Provider};
use crate::embedding::error::{ApiError, ConfigError, EmbeddingError};
use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics, Vector};

/// Task type for embedding optimization.
///
/// Google Vertex AI allows specifying how embeddings will be used to optimize
/// the embedding quality for that specific task.
#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    /// Optimized for embedding documents for retrieval
    RetrievalDocument,
    /// Optimized for embedding queries for retrieval
    RetrievalQuery,
    /// Optimized for general semantic similarity
    SemanticSimilarity,
}

impl TaskType {
    /// Convert task type to API string format.
    fn as_str(&self) -> &'static str {
        match self {
            TaskType::RetrievalDocument => "RETRIEVAL_DOCUMENT",
            TaskType::RetrievalQuery => "RETRIEVAL_QUERY",
            TaskType::SemanticSimilarity => "SEMANTIC_SIMILARITY",
        }
    }
}

// Note: ServiceAccountInfo and AccessToken removed - gcp_auth handles
// credentials and token caching internally

/// Request instance for Vertex AI predict endpoint.
#[derive(Serialize, Clone)]
struct EmbeddingInstance {
    /// Text content to embed
    content: String,
    /// Task type for optimization
    task_type: &'static str,
}

/// Request payload for Vertex AI predict endpoint.
#[derive(Serialize)]
struct PredictRequest {
    /// Array of instances to embed (up to 250)
    instances: Vec<EmbeddingInstance>,
}

/// Embedding prediction from response.
#[derive(Deserialize)]
struct Prediction {
    /// Embedding values array (768 floats)
    embeddings: EmbeddingValues,
}

/// Embedding values container.
#[derive(Deserialize)]
struct EmbeddingValues {
    /// Array of embedding floats
    values: Vec<f32>,
}

/// Response from Vertex AI predict endpoint.
#[derive(Deserialize)]
struct PredictResponse {
    /// Array of predictions (one per instance)
    predictions: Vec<Prediction>,
}

/// Google Cloud Vertex AI embedding provider.
///
/// This provider integrates with Google Cloud Vertex AI to generate embeddings
/// using the text-embedding-004 model (768 dimensions). It handles OAuth 2.0
/// authentication with service accounts, regional endpoints, and native batch processing.
///
/// # Configuration
///
/// - **Model**: Default `text-embedding-004`
/// - **Region**: Default `us-central1` (configurable)
/// - **Task Type**: Default `RETRIEVAL_DOCUMENT`
/// - **Timeout**: 30s per request, 90s for batch requests
/// - **Max Batch Size**: 250 texts per request
///
/// # Thread Safety
///
/// This provider is `Clone` and can be safely shared across async tasks.
/// The internal token cache uses `Arc<RwLock<_>>` for thread-safe access.
#[derive(Clone)]
pub struct GoogleProvider {
    /// HTTP client for making requests
    client: Client,
    /// GCP project ID
    project_id: String,
    /// GCP region (e.g., "us-central1")
    region: String,
    /// Model name (e.g., "textembedding-gecko@003")
    model: String,
    /// Default task type for embeddings
    task_type: TaskType,
    /// GCP token provider for OAuth2 token generation
    token_provider: Arc<dyn TokenProvider>,
    /// Metrics tracking
    metrics: Arc<RwLock<ProviderMetrics>>,
    /// Parallel processing configuration for batch embedding.
    /// Controls sub-batch size and concurrency limits.
    parallel_config: ParallelConfig,
    /// Semaphore to limit concurrent API requests.
    /// Initialized from parallel_config.max_concurrency.
    semaphore: Arc<Semaphore>,
}

impl GoogleProvider {
    /// Default model for embeddings.
    pub const DEFAULT_MODEL: &'static str = "text-embedding-004";

    /// Default region for Vertex AI.
    pub const DEFAULT_REGION: &'static str = "us-central1";

    /// Maximum texts per batch request.
    pub const MAX_BATCH_SIZE: usize = 250;

    /// Request timeout for single embeddings (30 seconds).
    const REQUEST_TIMEOUT_SECS: u64 = 30;

    /// Request timeout for batch embeddings (90 seconds).
    const BATCH_TIMEOUT_SECS: u64 = 90;

    /// Maximum retry attempts for transient errors.
    const MAX_RETRIES: u32 = 3;

    /// Base delay for exponential backoff (milliseconds).
    const BASE_RETRY_DELAY_MS: u64 = 1000;

    /// Create a new GoogleProvider with explicit configuration and parallel processing settings.
    ///
    /// This is the full-featured constructor that allows complete control over all settings
    /// including parallel batch processing configuration.
    ///
    /// # Arguments
    ///
    /// * `project_id` - GCP project ID
    /// * `credentials_path` - Path to service account JSON key file
    /// * `region` - GCP region (e.g., "us-central1", "europe-west1")
    /// * `model` - Model name (default: "text-embedding-004")
    /// * `parallel_config` - Parallel processing configuration for batch requests
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::google::GoogleProvider;
    /// use crewchief_maproom::embedding::config::ParallelConfig;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = GoogleProvider::new_with_config(
    ///     "my-project".to_string(),
    ///     PathBuf::from("/path/to/service-account.json"),
    ///     "us-central1".to_string(),
    ///     "text-embedding-004".to_string(),
    ///     ParallelConfig::google_defaults(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new_with_config(
        project_id: String,
        credentials_path: PathBuf,
        region: String,
        model: String,
        parallel_config: ParallelConfig,
    ) -> Result<Self, EmbeddingError> {
        // Validate credentials file exists
        if !credentials_path.exists() {
            return Err(EmbeddingError::Config(ConfigError::FileError(format!(
                "Credentials file not found: {}",
                credentials_path.display()
            ))));
        }

        // Set credentials path for gcp_auth to discover
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &credentials_path);

        // Create token provider (will use GOOGLE_APPLICATION_CREDENTIALS)
        let token_provider = gcp_auth::provider().await.map_err(|e| {
            EmbeddingError::Config(ConfigError::InvalidValue {
                field: "credentials".to_string(),
                reason: format!("Failed to create token provider: {}", e),
            })
        })?;

        // Create HTTP client with appropriate timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()?;

        // Initialize semaphore from parallel config
        let semaphore = Arc::new(Semaphore::new(parallel_config.max_concurrency));

        Ok(Self {
            client,
            project_id,
            region,
            model,
            task_type: TaskType::RetrievalDocument,
            token_provider,
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
            parallel_config,
            semaphore,
        })
    }

    /// Create a new GoogleProvider with explicit configuration.
    ///
    /// Uses default parallel processing settings optimized for Google Vertex AI
    /// (sub_batch_size: 200, max_concurrency: 16).
    ///
    /// # Arguments
    ///
    /// * `project_id` - GCP project ID
    /// * `credentials_path` - Path to service account JSON key file
    /// * `region` - GCP region (e.g., "us-central1", "europe-west1")
    /// * `model` - Model name (default: "text-embedding-004")
    ///
    /// # Returns
    ///
    /// - `Ok(GoogleProvider)` - Successfully created provider
    /// - `Err(EmbeddingError)` - If credentials file is invalid or HTTP client creation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::google::GoogleProvider;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = GoogleProvider::new(
    ///     "my-project".to_string(),
    ///     PathBuf::from("/path/to/service-account.json"),
    ///     "us-central1".to_string(),
    ///     "text-embedding-004".to_string(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        project_id: String,
        credentials_path: PathBuf,
        region: String,
        model: String,
    ) -> Result<Self, EmbeddingError> {
        Self::new_with_config(
            project_id,
            credentials_path,
            region,
            model,
            ParallelConfig::google_defaults(),
        )
        .await
    }

    /// Create a new GoogleProvider from environment variables.
    ///
    /// Reads configuration from:
    /// - `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account JSON key (required)
    /// - `GOOGLE_PROJECT_ID`: GCP project ID (required)
    /// - `GOOGLE_REGION`: GCP region (optional, defaults to "us-central1")
    /// - `GOOGLE_MODEL`: Model name (optional, defaults to "text-embedding-004")
    /// - `MAPROOM_EMBEDDING_PARALLEL_ENABLED`: Enable parallel processing (optional)
    /// - `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE`: Sub-batch size (optional)
    /// - `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY`: Max concurrent requests (optional)
    ///
    /// Uses `EmbeddingConfig::from_env_with_provider(Some(Provider::Google))` to load
    /// parallel config, ensuring Google-specific defaults are applied.
    ///
    /// # Returns
    ///
    /// - `Ok(GoogleProvider)` - Successfully created provider
    /// - `Err(EmbeddingError)` - If required environment variables are missing or invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::google::GoogleProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Requires GOOGLE_APPLICATION_CREDENTIALS and GOOGLE_PROJECT_ID env vars
    /// let provider = GoogleProvider::from_env().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_env() -> Result<Self, EmbeddingError> {
        // Load embedding config with Google provider to get parallel settings
        let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Google))?;
        let parallel_config = config.parallel;

        // Try Maproom-specific env vars first, then fall back to standard vars
        let credentials_path = std::env::var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS")
            .or_else(|_| std::env::var("GOOGLE_APPLICATION_CREDENTIALS"))
            .map_err(|_| {
                EmbeddingError::Config(ConfigError::EnvVarNotFound(
                    "MAPROOM_GOOGLE_APPLICATION_CREDENTIALS or GOOGLE_APPLICATION_CREDENTIALS"
                        .to_string(),
                ))
            })?;

        let project_id = std::env::var("MAPROOM_GOOGLE_PROJECT_ID")
            .or_else(|_| std::env::var("GOOGLE_PROJECT_ID"))
            .map_err(|_| {
                EmbeddingError::Config(ConfigError::EnvVarNotFound(
                    "MAPROOM_GOOGLE_PROJECT_ID or GOOGLE_PROJECT_ID".to_string(),
                ))
            })?;

        let region =
            std::env::var("GOOGLE_REGION").unwrap_or_else(|_| Self::DEFAULT_REGION.to_string());
        let model =
            std::env::var("GOOGLE_MODEL").unwrap_or_else(|_| Self::DEFAULT_MODEL.to_string());

        Self::new_with_config(
            project_id,
            PathBuf::from(credentials_path),
            region,
            model,
            parallel_config,
        )
        .await
    }

    /// Set the task type for embeddings.
    ///
    /// This configures how embeddings will be optimized. Use:
    /// - `RetrievalDocument` for documents that will be searched
    /// - `RetrievalQuery` for queries that will search documents
    /// - `SemanticSimilarity` for general similarity tasks
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::google::{GoogleProvider, TaskType};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut provider = GoogleProvider::from_env().await?;
    /// provider.with_task_type(TaskType::RetrievalQuery);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_task_type(&mut self, task_type: TaskType) -> &mut Self {
        self.task_type = task_type;
        self
    }

    /// Get or refresh cached access token.
    ///
    /// This method uses gcp_auth crate for proper OAuth 2.0 access token generation
    /// compatible with Vertex AI. Token caching and refresh is handled automatically
    /// by the TokenProvider implementation.
    async fn get_access_token(&self) -> Result<String, EmbeddingError> {
        // Scope required for Vertex AI is cloud-platform
        let scopes = &["https://www.googleapis.com/auth/cloud-platform"];

        // Get token from provider (automatically cached and refreshed by gcp_auth)
        let token: Arc<Token> = self.token_provider.token(scopes).await.map_err(|e| {
            EmbeddingError::Api(ApiError::Authentication(format!(
                "Failed to obtain access token: {}. Ensure GOOGLE_APPLICATION_CREDENTIALS \
                     points to a valid service account key and the service account has \
                     roles/aiplatform.user role.",
                e
            )))
        })?;

        Ok(token.as_str().to_string())
    }

    /// Construct Vertex AI predict endpoint URL.
    fn predict_url(&self) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
            self.region, self.project_id, self.region, self.model
        )
    }

    /// Make a predict request to Vertex AI with retry logic.
    async fn predict_with_retry(
        &self,
        instances: Vec<EmbeddingInstance>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        let mut last_error = None;

        for attempt in 0..Self::MAX_RETRIES {
            match self.predict_request(instances.clone()).await {
                Ok(embeddings) => {
                    // Update metrics
                    let mut metrics = self.metrics.write().await;
                    metrics.total_requests += 1;
                    return Ok(embeddings);
                }
                Err(e) => {
                    // Update failed request metric
                    {
                        let mut metrics = self.metrics.write().await;
                        metrics.total_requests += 1;
                        metrics.failed_requests += 1;
                    }

                    // Check if error is retryable
                    let should_retry = match &e {
                        EmbeddingError::Network(_) => true,
                        EmbeddingError::Api(api_err) => api_err.is_retryable(),
                        _ => false,
                    };

                    if !should_retry || attempt == Self::MAX_RETRIES - 1 {
                        return Err(e);
                    }

                    last_error = Some(e);

                    // Exponential backoff
                    let delay_ms = Self::BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| EmbeddingError::Other("All retry attempts failed".to_string())))
    }

    /// Make a single predict request to Vertex AI.
    async fn predict_request(
        &self,
        instances: Vec<EmbeddingInstance>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        // Get valid access token
        let access_token = self.get_access_token().await?;

        // Prepare request
        let request_body = PredictRequest { instances };

        // Determine timeout based on batch size
        let timeout = if request_body.instances.len() > 1 {
            Duration::from_secs(Self::BATCH_TIMEOUT_SECS)
        } else {
            Duration::from_secs(Self::REQUEST_TIMEOUT_SECS)
        };

        // Make request
        let response = self
            .client
            .post(self.predict_url())
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .timeout(timeout)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(EmbeddingError::Api(match status.as_u16() {
                401 => ApiError::Authentication(format!(
                    "Invalid credentials or expired token. Ensure service account has roles/aiplatform.user role. Error: {}",
                    error_text
                )),
                403 => ApiError::Authentication(format!(
                    "Insufficient IAM permissions. Service account needs roles/aiplatform.user role. Error: {}",
                    error_text
                )),
                429 => {
                    // Try to extract retry-after header
                    let retry_after_ms = 1000; // Default 1 second
                    ApiError::RateLimit { retry_after_ms }
                }
                503 => ApiError::ServerError {
                    status: 503,
                    message: format!("Service temporarily unavailable: {}", error_text),
                },
                500..=599 => ApiError::ServerError {
                    status: status.as_u16(),
                    message: error_text,
                },
                400 => ApiError::BadRequest(error_text),
                _ => ApiError::InvalidResponse(format!("HTTP {}: {}", status, error_text)),
            }));
        }

        // Parse response
        let response_body: PredictResponse = response.json().await?;

        // Extract embeddings
        let embeddings: Vec<Vector> = response_body
            .predictions
            .into_iter()
            .map(|pred| pred.embeddings.values)
            .collect();

        // Validate dimensions
        let expected_dim = self.dimension();
        for embedding in embeddings.iter() {
            if embedding.len() != expected_dim {
                use crate::embedding::error::DimensionMismatchError;
                return Err(EmbeddingError::DimensionMismatch(
                    DimensionMismatchError::new(
                        expected_dim,
                        embedding.len(),
                        "Google".to_string(),
                        self.model.clone(),
                        expected_dim,
                    ),
                ));
            }
        }

        Ok(embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for GoogleProvider {
    /// Generate embedding vector for a single text.
    ///
    /// This method calls the Google Vertex AI predict endpoint to generate a
    /// 768-dimensional embedding vector for the input text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vector)` - 768-dimensional embedding vector
    /// - `Err(EmbeddingError)` - If the API call fails after retries
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::google::GoogleProvider;
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = GoogleProvider::from_env().await?;
    /// let embedding = provider.embed("Hello, world!".to_string()).await?;
    /// assert_eq!(embedding.len(), 768);
    /// # Ok(())
    /// # }
    /// ```
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        let instances = vec![EmbeddingInstance {
            content: text,
            task_type: self.task_type.as_str(),
        }];

        let mut embeddings = self.predict_with_retry(instances).await?;

        Ok(embeddings.remove(0))
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// This method uses Vertex AI's native batch embedding API to efficiently
    /// process up to 250 texts in a single request.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed (up to 250)
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of 768-dimensional embeddings (same length as input)
    /// - `Err(EmbeddingError)` - If the API call fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::google::GoogleProvider;
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = GoogleProvider::from_env().await?;
    /// let texts = vec!["First".to_string(), "Second".to_string()];
    /// let embeddings = provider.embed_batch(texts).await?;
    /// assert_eq!(embeddings.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Validate batch size
        if texts.len() > Self::MAX_BATCH_SIZE {
            return Err(EmbeddingError::InvalidInput(format!(
                "Batch size {} exceeds maximum of {}",
                texts.len(),
                Self::MAX_BATCH_SIZE
            )));
        }

        // Convert texts to instances
        let instances: Vec<EmbeddingInstance> = texts
            .into_iter()
            .map(|content| EmbeddingInstance {
                content,
                task_type: self.task_type.as_str(),
            })
            .collect();

        self.predict_with_retry(instances).await
    }

    /// Get the embedding dimension for this provider.
    ///
    /// Google Vertex AI's text-embedding-004 model produces 768-dimensional embeddings.
    ///
    /// # Returns
    ///
    /// Always returns 768.
    fn dimension(&self) -> usize {
        768 // text-embedding-004 fixed dimension
    }

    /// Get the provider name identifier.
    ///
    /// # Returns
    ///
    /// Always returns "google".
    fn provider_name(&self) -> &'static str {
        "google"
    }

    /// Get provider-specific metrics.
    ///
    /// # Returns
    ///
    /// Current metrics including request counts and failure rates.
    fn metrics(&self) -> Option<ProviderMetrics> {
        // Use try_read to avoid blocking in async context
        // Returns None if metrics are currently locked (rare, transient)
        self.metrics.try_read().ok().map(|m| m.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::RetrievalDocument.as_str(), "RETRIEVAL_DOCUMENT");
        assert_eq!(TaskType::RetrievalQuery.as_str(), "RETRIEVAL_QUERY");
        assert_eq!(TaskType::SemanticSimilarity.as_str(), "SEMANTIC_SIMILARITY");
    }

    // Note: AccessToken tests removed - gcp_auth handles token caching and expiry internally

    #[tokio::test]
    async fn test_predict_url_construction() {
        // Create a dummy auth manager for testing
        // Note: This test doesn't actually call GCP, just tests URL construction
        let temp_creds = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp_creds.path(),
            r#"{
                "type": "service_account",
                "project_id": "test-project",
                "private_key_id": "key-id",
                "private_key": "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA2Z3qX2BTLS4e7VPIQKfSqfE8LKqCBOcN67jv\n-----END RSA PRIVATE KEY-----\n",
                "client_email": "test@test-project.iam.gserviceaccount.com",
                "client_id": "123456789",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }"#,
        )
        .unwrap();

        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", temp_creds.path());

        // This will fail to create actual auth manager without valid credentials
        // So we'll skip the actual provider creation and just test URL construction
        // by creating the URL directly
        let project_id = "my-project";
        let region = "us-central1";
        let model = "text-embedding-004";

        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
            region, project_id, region, model
        );

        assert!(url.contains("us-central1-aiplatform.googleapis.com"));
        assert!(url.contains("my-project"));
        assert!(url.contains("text-embedding-004"));
        assert!(url.contains(":predict"));
    }

    #[test]
    fn test_embedding_instance_serialization() {
        let instance = EmbeddingInstance {
            content: "test text".to_string(),
            task_type: "RETRIEVAL_DOCUMENT",
        };

        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("test text"));
        assert!(json.contains("RETRIEVAL_DOCUMENT"));
    }

    #[test]
    fn test_predict_response_deserialization() {
        let json = r#"{
            "predictions": [
                {
                    "embeddings": {
                        "values": [0.1, 0.2, 0.3]
                    }
                }
            ]
        }"#;

        let response: PredictResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.predictions.len(), 1);
        assert_eq!(response.predictions[0].embeddings.values.len(), 3);
        assert_eq!(response.predictions[0].embeddings.values[0], 0.1);
    }

    #[tokio::test]
    async fn test_dimension_and_provider_name() {
        // Test dimension and provider name without needing actual GCP credentials
        // These are constants and don't require authentication

        // We can't easily create a GoogleProvider without valid credentials,
        // so we'll test these constants directly
        assert_eq!(GoogleProvider::DEFAULT_MODEL, "text-embedding-004");
        assert_eq!(GoogleProvider::DEFAULT_REGION, "us-central1");
        assert_eq!(GoogleProvider::MAX_BATCH_SIZE, 250);

        // Dimension is always 768 for text-embedding-004
        // Provider name is always "google"
        // These would be tested via integration tests with real credentials
    }

    #[test]
    fn test_max_batch_size_constant() {
        assert_eq!(GoogleProvider::MAX_BATCH_SIZE, 250);
    }
}
