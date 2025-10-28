//! Google Cloud Vertex AI embedding provider implementation.
//!
//! This module provides integration with Google Cloud Vertex AI for enterprise-grade
//! embedding generation. Uses the textembedding-gecko@003 model which produces
//! 768-dimensional embeddings suitable for semantic search and retrieval.
//!
//! # Features
//!
//! - Service account JSON key authentication
//! - Regional endpoint support (us-central1, europe-west1, asia-southeast1, etc.)
//! - Task type optimization (RETRIEVAL_DOCUMENT, RETRIEVAL_QUERY, SEMANTIC_SIMILARITY)
//! - Native batch processing (up to 250 texts per request)
//! - 768-dimensional vectors (textembedding-gecko@003)
//! - Exponential backoff retry logic for transient errors
//! - OAuth 2.0 JWT bearer token authentication
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
use google_cloud_auth::token::DefaultTokenSourceProvider;
use google_cloud_token::TokenSourceProvider;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

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

/// Service account credentials file path.
#[derive(Debug, Clone)]
struct ServiceAccountInfo {
    /// Path to service account JSON key file
    credentials_path: PathBuf,
}

/// OAuth 2.0 access token with expiry tracking.
#[derive(Debug, Clone)]
struct AccessToken {
    /// The bearer token
    token: String,
    /// Unix timestamp when token expires
    expires_at: u64,
}

impl AccessToken {
    /// Check if token is expired or will expire within 5 minutes.
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Add 5-minute buffer to refresh before actual expiry
        now + 300 >= self.expires_at
    }
}

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
/// using the textembedding-gecko@003 model (768 dimensions). It handles OAuth 2.0
/// authentication with service accounts, regional endpoints, and native batch processing.
///
/// # Configuration
///
/// - **Model**: Default `textembedding-gecko@003`
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
    /// Service account credentials info
    credentials_info: Arc<ServiceAccountInfo>,
    /// Cached access token with expiry tracking
    token_cache: Arc<RwLock<Option<AccessToken>>>,
    /// Metrics tracking
    metrics: Arc<RwLock<ProviderMetrics>>,
}

impl GoogleProvider {
    /// Default model for embeddings.
    pub const DEFAULT_MODEL: &'static str = "textembedding-gecko@003";

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

    /// JWT token lifetime (1 hour in seconds).
    const JWT_LIFETIME_SECS: u64 = 3600;

    /// Create a new GoogleProvider with explicit configuration.
    ///
    /// # Arguments
    ///
    /// * `project_id` - GCP project ID
    /// * `credentials_path` - Path to service account JSON key file
    /// * `region` - GCP region (e.g., "us-central1", "europe-west1")
    /// * `model` - Model name (default: "textembedding-gecko@003")
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
    ///     "textembedding-gecko@003".to_string(),
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
        // Validate credentials file exists
        if !credentials_path.exists() {
            return Err(EmbeddingError::Config(ConfigError::FileError(format!(
                "Credentials file not found: {}",
                credentials_path.display()
            ))));
        }

        // Create HTTP client with appropriate timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            project_id,
            region,
            model,
            task_type: TaskType::RetrievalDocument,
            credentials_info: Arc::new(ServiceAccountInfo { credentials_path }),
            token_cache: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
        })
    }

    /// Create a new GoogleProvider from environment variables.
    ///
    /// Reads configuration from:
    /// - `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account JSON key (required)
    /// - `GOOGLE_PROJECT_ID`: GCP project ID (required)
    /// - `GOOGLE_REGION`: GCP region (optional, defaults to "us-central1")
    /// - `GOOGLE_MODEL`: Model name (optional, defaults to "textembedding-gecko@003")
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
        let credentials_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
            EmbeddingError::Config(ConfigError::EnvVarNotFound(
                "GOOGLE_APPLICATION_CREDENTIALS".to_string(),
            ))
        })?;

        let project_id = std::env::var("GOOGLE_PROJECT_ID").map_err(|_| {
            EmbeddingError::Config(ConfigError::EnvVarNotFound(
                "GOOGLE_PROJECT_ID".to_string(),
            ))
        })?;

        let region = std::env::var("GOOGLE_REGION").unwrap_or_else(|_| Self::DEFAULT_REGION.to_string());
        let model = std::env::var("GOOGLE_MODEL").unwrap_or_else(|_| Self::DEFAULT_MODEL.to_string());

        Self::new(project_id, PathBuf::from(credentials_path), region, model).await
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
    /// This method implements token caching with automatic refresh. Tokens are
    /// refreshed if expired or missing. Uses google-cloud-auth crate for proper
    /// OAuth 2.0 JWT bearer token flow with RSA signing.
    async fn get_access_token(&self) -> Result<String, EmbeddingError> {
        // Check if we have a valid cached token
        {
            let cache = self.token_cache.read().await;
            if let Some(token) = cache.as_ref() {
                if !token.is_expired() {
                    return Ok(token.token.clone());
                }
            }
        }

        // Token expired or missing, acquire write lock and refresh
        let mut cache = self.token_cache.write().await;

        // Double-check after acquiring write lock (another task may have refreshed)
        if let Some(token) = cache.as_ref() {
            if !token.is_expired() {
                return Ok(token.token.clone());
            }
        }

        // Use google-cloud-auth crate for proper token generation
        // Set GOOGLE_APPLICATION_CREDENTIALS env var temporarily for this call
        let original_env = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok();
        std::env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            &self.credentials_info.credentials_path,
        );

        // Create token source provider from credentials file
        let ts_provider = DefaultTokenSourceProvider::new(google_cloud_auth::project::Config {
            audience: None,
            scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
            sub: None,
        })
        .await
        .map_err(|e| {
            EmbeddingError::Config(ConfigError::InvalidValue {
                field: "credentials".to_string(),
                reason: format!("Failed to create token source: {}", e),
            })
        })?;

        // Restore original env var
        if let Some(original) = original_env {
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", original);
        } else {
            std::env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        }

        // Get token from the provider's token source
        let token_source = ts_provider.token_source();
        let token_result = token_source.token().await.map_err(|e| {
            EmbeddingError::Api(ApiError::Authentication(format!(
                "Failed to obtain access token: {}",
                e
            )))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Cache the new token (default expiry is 3600 seconds)
        let access_token = AccessToken {
            token: token_result.clone(),
            expires_at: now + Self::JWT_LIFETIME_SECS,
        };

        *cache = Some(access_token);

        Ok(token_result)
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

        Err(last_error.unwrap_or_else(|| {
            EmbeddingError::Other("All retry attempts failed".to_string())
        }))
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
        for (idx, embedding) in embeddings.iter().enumerate() {
            if embedding.len() != expected_dim {
                return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                    "Dimension mismatch at index {}: expected {} dimensions but got {}",
                    idx,
                    expected_dim,
                    embedding.len()
                ))));
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
    /// Google Vertex AI's textembedding-gecko@003 model produces 768-dimensional embeddings.
    ///
    /// # Returns
    ///
    /// Always returns 768.
    fn dimension(&self) -> usize {
        768 // textembedding-gecko@003 fixed dimension
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
        // Create a blocking task to read metrics
        // This is safe because the lock is held very briefly
        let metrics = self.metrics.blocking_read();
        Some(metrics.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::RetrievalDocument.as_str(), "RETRIEVAL_DOCUMENT");
        assert_eq!(TaskType::RetrievalQuery.as_str(), "RETRIEVAL_QUERY");
        assert_eq!(
            TaskType::SemanticSimilarity.as_str(),
            "SEMANTIC_SIMILARITY"
        );
    }

    #[test]
    fn test_access_token_expiry() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Token that expires in 10 minutes (should not be expired due to 5-min buffer)
        let token = AccessToken {
            token: "test".to_string(),
            expires_at: now + 600,
        };
        assert!(!token.is_expired());

        // Token that expires in 4 minutes (should be expired due to 5-min buffer)
        let token = AccessToken {
            token: "test".to_string(),
            expires_at: now + 240,
        };
        assert!(token.is_expired());

        // Already expired token
        let token = AccessToken {
            token: "test".to_string(),
            expires_at: now - 100,
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_predict_url_construction() {
        let credentials_info = ServiceAccountInfo {
            credentials_path: PathBuf::from("/tmp/test-credentials.json"),
        };

        let provider = GoogleProvider {
            client: Client::new(),
            project_id: "my-project".to_string(),
            region: "us-central1".to_string(),
            model: "textembedding-gecko@003".to_string(),
            task_type: TaskType::RetrievalDocument,
            credentials_info: Arc::new(credentials_info),
            token_cache: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
        };

        let url = provider.predict_url();
        assert!(url.contains("us-central1-aiplatform.googleapis.com"));
        assert!(url.contains("my-project"));
        assert!(url.contains("textembedding-gecko@003"));
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

    #[test]
    fn test_service_account_info() {
        let info = ServiceAccountInfo {
            credentials_path: PathBuf::from("/path/to/credentials.json"),
        };

        assert_eq!(
            info.credentials_path.to_str().unwrap(),
            "/path/to/credentials.json"
        );
    }

    #[tokio::test]
    async fn test_dimension_and_provider_name() {
        let credentials_info = ServiceAccountInfo {
            credentials_path: PathBuf::from("/tmp/test-credentials.json"),
        };

        let provider = GoogleProvider {
            client: Client::new(),
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            model: "textembedding-gecko@003".to_string(),
            task_type: TaskType::RetrievalDocument,
            credentials_info: Arc::new(credentials_info),
            token_cache: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
        };

        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.provider_name(), "google");
    }

    #[test]
    fn test_max_batch_size_constant() {
        assert_eq!(GoogleProvider::MAX_BATCH_SIZE, 250);
    }
}
