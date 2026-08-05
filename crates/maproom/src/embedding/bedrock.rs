//! AWS Bedrock embedding provider.
//!
//! Generates embeddings through the Bedrock Runtime `InvokeModel` API, using the
//! standard AWS credential chain (see [`crate::embedding::aws::credentials`]) and
//! SigV4 request signing (see [`crate::embedding::aws::sigv4`]).
//!
//! # Supported models
//!
//! | Model id | Dimensions | Texts per request |
//! |----------|-----------:|------------------:|
//! | `amazon.titan-embed-text-v2:0` (default) | 1024 | 1 |
//! | `amazon.titan-embed-text-v1` | 1536 | 1 |
//! | `cohere.embed-english-v3` | 1024 | 96 |
//! | `cohere.embed-multilingual-v3` | 1024 | 96 |
//!
//! Any other model id is usable by setting `MAPROOM_EMBEDDING_DIMENSION`
//! explicitly, which covers provisioned-throughput ARNs and models released
//! after this crate.
//!
//! Titan v2 can also emit 512- and 256-dimensional vectors, but maproom stores
//! embeddings in per-dimension tables and has column sets only for 768, 1024,
//! and 1536. Those narrower widths are rejected at construction rather than
//! after a full scan — see [`validate_dimension`].
//!
//! # Batching
//!
//! Bedrock's `InvokeModel` is a single-document API for Titan: one text per
//! call. Cohere's Bedrock payload accepts up to 96 texts per call. Both are
//! driven through the same concurrent sub-batch pipeline as the Ollama and
//! Google providers, so a Titan scan of 10,000 chunks issues 10,000 requests
//! across a bounded number of in-flight connections rather than serially.
//!
//! # Retrieval quality
//!
//! Cohere models distinguish documents from queries (`search_document` vs
//! `search_query`) and produce measurably better retrieval when told which is
//! which. Indexing uses `search_document`; [`BedrockProvider::for_queries`]
//! returns a handle that uses `search_query`. Titan has no such distinction and
//! ignores the setting.
//!
//! # Examples
//!
//! ```no_run
//! use maproom::embedding::bedrock::BedrockProvider;
//! use maproom::embedding::provider::EmbeddingProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Region, model, and credentials all come from the environment.
//! let provider = BedrockProvider::from_env().await?;
//! let embedding = provider.embed("fn main() {}".to_string()).await?;
//! assert_eq!(embedding.len(), provider.dimension());
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{RwLock, Semaphore};

use crate::context::TokenCounter;
use crate::embedding::aws::sigv4::{self, SignableRequest};
use crate::embedding::aws::{self, CredentialsProvider};
use crate::embedding::config::ParallelConfig;
use crate::embedding::error::{ApiError, ConfigError, DimensionMismatchError, EmbeddingError};
use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics, Vector};

/// The Bedrock model family, which determines request shape and batch limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    /// `amazon.titan-embed-text-v1` — fixed 1536 dimensions.
    TitanV1,
    /// `amazon.titan-embed-text-v2` — selectable 256/512/1024 dimensions.
    TitanV2,
    /// `cohere.embed-*-v3` — 1024 dimensions, native batching.
    CohereV3,
}

impl ModelFamily {
    /// Texts accepted in a single `InvokeModel` call.
    pub fn max_batch_size(&self) -> usize {
        match self {
            // Titan's payload has a single `inputText` field — there is no
            // batch form of the request.
            Self::TitanV1 | Self::TitanV2 => 1,
            Self::CohereV3 => 96,
        }
    }

    /// Native dimension when the model id implies one.
    fn default_dimension(&self) -> usize {
        match self {
            Self::TitanV1 => 1536,
            Self::TitanV2 => 1024,
            Self::CohereV3 => 1024,
        }
    }

    /// Token ceiling per text, with margin below the documented limit.
    fn max_tokens_per_text(&self) -> usize {
        match self {
            // Titan v1/v2 accept 8k tokens; leave room for tokenizer drift
            // between tiktoken (what we count with) and Titan's own tokenizer.
            Self::TitanV1 | Self::TitanV2 => 7_500,
            // Cohere v3 embeds at most 512 tokens and silently truncates.
            // Truncating deliberately keeps the cost predictable.
            Self::CohereV3 => 500,
        }
    }

    /// On-demand price per 1,000 input tokens, in USD (us-east-1).
    ///
    /// Used only for the cost estimate surfaced in metrics; billing is whatever
    /// AWS charges. Regional and provisioned pricing differ.
    fn price_per_1k_tokens(&self) -> f64 {
        match self {
            Self::TitanV2 => 0.00002,
            Self::TitanV1 => 0.0001,
            Self::CohereV3 => 0.0001,
        }
    }

    /// Human-readable name for diagnostics.
    fn label(&self) -> &'static str {
        match self {
            Self::TitanV1 => "Amazon Titan Text Embeddings v1",
            Self::TitanV2 => "Amazon Titan Text Embeddings v2",
            Self::CohereV3 => "Cohere Embed v3",
        }
    }
}

/// Whether a text is being embedded for storage or as a search query.
///
/// Only Cohere models act on this; Titan ignores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Text being indexed.
    Document,
    /// Text being used to search.
    Query,
}

impl InputType {
    /// The value Cohere's Bedrock payload expects.
    fn as_cohere_str(&self) -> &'static str {
        match self {
            Self::Document => "search_document",
            Self::Query => "search_query",
        }
    }
}

/// Embedding provider backed by AWS Bedrock.
///
/// Cloning is cheap and shares credentials, metrics, and the concurrency
/// semaphore.
#[derive(Clone)]
pub struct BedrockProvider {
    http: Client,
    credentials: CredentialsProvider,
    /// Model id exactly as it will be sent to Bedrock.
    model: String,
    family: ModelFamily,
    dimension: usize,
    region: String,
    /// Base URL, e.g. `https://bedrock-runtime.us-east-1.amazonaws.com`.
    endpoint: String,
    input_type: InputType,
    /// Whether to ask Titan v2 to L2-normalize its output.
    normalize: bool,
    metrics: Arc<RwLock<ProviderMetrics>>,
    parallel_config: ParallelConfig,
    semaphore: Arc<Semaphore>,
}

// Written by hand rather than derived: the provider holds a credentials
// provider, and a derived Debug would risk rendering secrets into a log line.
impl std::fmt::Debug for BedrockProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockProvider")
            .field("model", &self.model)
            .field("family", &self.family)
            .field("dimension", &self.dimension)
            .field("region", &self.region)
            .field("endpoint", &self.endpoint)
            .field("input_type", &self.input_type)
            .finish_non_exhaustive()
    }
}

impl BedrockProvider {
    /// Default model: the current-generation Titan text embedding model.
    ///
    /// Chosen because it is available in every Bedrock region, is the cheapest
    /// of the supported models, and its 1024 dimensions match the existing
    /// `vec_code_1024` table used by `mxbai-embed-large`.
    pub const DEFAULT_MODEL: &'static str = "amazon.titan-embed-text-v2:0";

    /// Request timeout for a single-text embedding.
    const REQUEST_TIMEOUT_SECS: u64 = 30;

    /// Request timeout for a batched embedding call.
    const BATCH_TIMEOUT_SECS: u64 = 90;

    /// Attempts per request, including the first.
    const MAX_RETRIES: u32 = 4;

    /// Base delay for exponential backoff.
    const BASE_RETRY_DELAY_MS: u64 = 500;

    /// Build a provider from environment configuration.
    ///
    /// Reads `MAPROOM_EMBEDDING_MODEL`, region from
    /// [`aws::resolve_region`], and credentials from the standard chain. The
    /// dimension is inferred from the model id unless
    /// `MAPROOM_EMBEDDING_DIMENSION` overrides it.
    pub async fn from_env() -> Result<Self, EmbeddingError> {
        let config = crate::embedding::config::EmbeddingConfig::from_env_with_provider(Some(
            crate::embedding::config::Provider::Bedrock,
        ))?;
        Self::new(config.model, config.dimension, config.parallel).await
    }

    /// Build a provider with an explicit model, dimension, and parallel config.
    pub async fn new(
        model: String,
        dimension: usize,
        parallel_config: ParallelConfig,
    ) -> Result<Self, EmbeddingError> {
        let family = infer_model_family(&model).ok_or_else(|| unknown_model_error(&model))?;
        validate_dimension(family, &model, dimension)?;

        let http = Client::builder()
            .timeout(Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()?;

        let profiles = crate::embedding::aws::profile::ProfileSet::load();
        let profile_name = std::env::var("MAPROOM_AWS_PROFILE")
            .or_else(|_| std::env::var("AWS_PROFILE"))
            .ok()
            .filter(|value| !value.trim().is_empty());
        let region = aws::resolve_region(&profiles, profile_name.as_deref());
        let endpoint = resolve_endpoint(&region);

        let credentials = CredentialsProvider::new(http.clone(), region.clone());
        let semaphore = Arc::new(Semaphore::new(parallel_config.max_concurrency.max(1)));

        tracing::info!(
            "Using provider: bedrock ({}, model: {}, dimension: {}, region: {}, endpoint: {}, \
             parallel: enabled={}, sub_batch={}, concurrency={})",
            family.label(),
            model,
            dimension,
            region,
            endpoint,
            parallel_config.enabled,
            parallel_config.sub_batch_size.min(family.max_batch_size()),
            parallel_config.max_concurrency,
        );

        Ok(Self {
            http,
            credentials,
            model,
            family,
            dimension,
            region,
            endpoint,
            input_type: InputType::Document,
            normalize: true,
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
            parallel_config,
            semaphore,
        })
    }

    /// A clone of this provider that embeds search queries rather than documents.
    ///
    /// Meaningful for Cohere models, which encode queries and documents into a
    /// shared space using different prompts. A no-op for Titan.
    pub fn for_queries(&self) -> Self {
        let mut clone = self.clone();
        clone.input_type = InputType::Query;
        clone
    }

    /// The resolved AWS region.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// The model family in use.
    pub fn family(&self) -> ModelFamily {
        self.family
    }

    /// The resolved Bedrock Runtime endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Build the `InvokeModel` request body for a batch of texts.
    ///
    /// Titan takes exactly one text; Cohere takes the whole slice.
    fn request_body(&self, texts: &[String]) -> Result<Vec<u8>, EmbeddingError> {
        let body = match self.family {
            ModelFamily::TitanV1 => json!({ "inputText": texts[0] }),
            ModelFamily::TitanV2 => json!({
                "inputText": texts[0],
                "dimensions": self.dimension,
                "normalize": self.normalize,
            }),
            ModelFamily::CohereV3 => json!({
                "texts": texts,
                "input_type": self.input_type.as_cohere_str(),
                // Embed the head of anything overlong rather than erroring; we
                // also pre-truncate, so this is a backstop for tokenizer drift.
                "truncate": "END",
            }),
        };
        serde_json::to_vec(&body).map_err(EmbeddingError::Json)
    }

    /// Parse an `InvokeModel` response body into embeddings.
    ///
    /// Returns the embeddings plus the token count Bedrock reported, when it
    /// reports one (Titan does, Cohere does not).
    fn parse_response(&self, body: &[u8]) -> Result<(Vec<Vector>, Option<u64>), EmbeddingError> {
        match self.family {
            ModelFamily::TitanV1 | ModelFamily::TitanV2 => {
                #[derive(Deserialize)]
                struct TitanResponse {
                    embedding: Vec<f32>,
                    #[serde(rename = "inputTextTokenCount")]
                    input_text_token_count: Option<u64>,
                }
                let parsed: TitanResponse = serde_json::from_slice(body).map_err(|error| {
                    EmbeddingError::Api(ApiError::InvalidResponse(format!(
                        "Titan response was not in the expected shape: {error}"
                    )))
                })?;
                Ok((vec![parsed.embedding], parsed.input_text_token_count))
            }
            ModelFamily::CohereV3 => {
                // Cohere returns a bare array of vectors by default, but an
                // object keyed by embedding type when `embedding_types` is set.
                // Accept both so a future request-shape change cannot silently
                // produce zero embeddings.
                #[derive(Deserialize)]
                #[serde(untagged)]
                enum CohereEmbeddings {
                    Float(Vec<Vec<f32>>),
                    ByType {
                        #[serde(rename = "float")]
                        float: Vec<Vec<f32>>,
                    },
                }
                #[derive(Deserialize)]
                struct CohereResponse {
                    embeddings: CohereEmbeddings,
                }
                let parsed: CohereResponse = serde_json::from_slice(body).map_err(|error| {
                    EmbeddingError::Api(ApiError::InvalidResponse(format!(
                        "Cohere response was not in the expected shape: {error}"
                    )))
                })?;
                let embeddings = match parsed.embeddings {
                    CohereEmbeddings::Float(vectors) => vectors,
                    CohereEmbeddings::ByType { float } => float,
                };
                Ok((embeddings, None))
            }
        }
    }

    /// Issue one signed `InvokeModel` call, with retries for transient failures.
    async fn invoke_with_retry(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        let mut last_error: Option<EmbeddingError> = None;

        for attempt in 0..Self::MAX_RETRIES {
            match self.invoke(&texts).await {
                Ok(embeddings) => {
                    let mut metrics = self.metrics.write().await;
                    metrics.total_requests += 1;
                    return Ok(embeddings);
                }
                Err(error) => {
                    {
                        let mut metrics = self.metrics.write().await;
                        metrics.total_requests += 1;
                        metrics.failed_requests += 1;
                    }

                    let retryable = match &error {
                        EmbeddingError::Network(_) => true,
                        EmbeddingError::Api(api_error) => api_error.is_retryable(),
                        _ => false,
                    };
                    if !retryable || attempt == Self::MAX_RETRIES - 1 {
                        return Err(error);
                    }

                    // Bedrock throttles aggressively on burst scans, so back off
                    // with the delay the error suggests when it supplies one.
                    let suggested = match &error {
                        EmbeddingError::Api(api_error) => api_error.retry_delay_ms(),
                        _ => None,
                    };
                    let delay = suggested
                        .unwrap_or(Self::BASE_RETRY_DELAY_MS * 2u64.pow(attempt))
                        .min(30_000);
                    tracing::debug!(
                        "Bedrock request failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        Self::MAX_RETRIES,
                        delay,
                        error,
                    );
                    last_error = Some(error);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            EmbeddingError::Other("All Bedrock retry attempts failed".to_string())
        }))
    }

    /// Issue one signed `InvokeModel` call.
    async fn invoke(&self, texts: &[String]) -> Result<Vec<Vector>, EmbeddingError> {
        let body = self.request_body(texts)?;

        // The model id goes in the path, so reserved characters — the `:0`
        // suffix on Titan v2, and every `:` and `/` in an ARN — must be encoded.
        // An endpoint override may carry a path prefix (an egress proxy that
        // mounts Bedrock under a subpath). The prefix must appear in the signed
        // canonical URI and must NOT appear in the Host header, so derive both
        // from one split rather than trimming the scheme off in place.
        let (host, base_path) = split_endpoint(&self.endpoint);
        let path = format!(
            "{base_path}/model/{}/invoke",
            sigv4::encode_path_segment(&self.model)
        );

        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("accept".to_string(), "application/json".to_string()),
        ];
        let signable = SignableRequest {
            method: "POST",
            host: &host,
            path: &path,
            query: "",
            headers: &headers,
            body: &body,
        };
        let signing_credentials = self.credentials.signing_credentials().await?;
        let signed = sigv4::sign_request(
            &signable,
            &signing_credentials,
            &self.region,
            "bedrock",
            &sigv4::format_amz_date(SystemTime::now()),
        );

        let timeout = if texts.len() > 1 {
            Duration::from_secs(Self::BATCH_TIMEOUT_SECS)
        } else {
            Duration::from_secs(Self::REQUEST_TIMEOUT_SECS)
        };

        let mut request = self
            .http
            // `path` already carries the endpoint's base path, so the URL is
            // rebuilt from scheme + host rather than the full endpoint string.
            .post(format!("{}{}{}", endpoint_scheme(&self.endpoint), host, path))
            .timeout(timeout)
            .body(body.clone());
        for (name, value) in &signed.headers {
            request = request.header(name, value);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_type = response
                .headers()
                .get("x-amzn-errortype")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .split(':')
                .next()
                .unwrap_or_default()
                .to_string();
            let retry_after_ms = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(|seconds| seconds * 1000);
            let text = response.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &error_type, &text, retry_after_ms));
        }

        let bytes = response.bytes().await?;
        let (embeddings, reported_tokens) = self.parse_response(&bytes)?;

        if embeddings.len() != texts.len() {
            return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                "Bedrock returned {} embeddings for {} inputs",
                embeddings.len(),
                texts.len(),
            ))));
        }

        for embedding in &embeddings {
            if embedding.len() != self.dimension {
                return Err(EmbeddingError::DimensionMismatch(
                    DimensionMismatchError::new(
                        self.dimension,
                        embedding.len(),
                        "Bedrock".to_string(),
                        self.model.clone(),
                        self.dimension,
                    ),
                ));
            }
        }

        // Titan reports exact token counts; for Cohere, estimate so cost
        // reporting is not silently zero.
        let tokens = reported_tokens.unwrap_or_else(|| estimate_tokens(texts));
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_tokens += tokens;
            metrics.estimated_cost_usd +=
                (tokens as f64 / 1000.0) * self.family.price_per_1k_tokens();
        }

        Ok(embeddings)
    }

    /// Turn a Bedrock HTTP failure into a typed error with actionable guidance.
    ///
    /// The mapping matters operationally: `is_retryable` drives the backoff loop,
    /// and the message is what an operator sees when a scan stops.
    fn map_error(
        &self,
        status: u16,
        error_type: &str,
        body: &str,
        retry_after_ms: Option<u64>,
    ) -> EmbeddingError {
        let detail = extract_bedrock_message(body);

        // Prefer the modeled exception name — Bedrock returns 400 for several
        // conditions that need different operator responses.
        match error_type {
            "ThrottlingException" | "TooManyRequestsException" => {
                return EmbeddingError::Api(ApiError::RateLimit {
                    retry_after_ms: retry_after_ms.unwrap_or(1000),
                })
            }
            "ServiceQuotaExceededException" => {
                return EmbeddingError::Api(ApiError::QuotaExceeded(format!(
                    "Bedrock service quota exceeded for {}: {detail}\n\
                     Request a quota increase for 'InvokeModel requests per minute' in the \
                     Service Quotas console for region {}.",
                    self.model, self.region,
                )))
            }
            "AccessDeniedException" => {
                return EmbeddingError::Api(ApiError::Authentication(format!(
                    "Access denied invoking {} in {}: {detail}\n\
                     The caller needs the bedrock:InvokeModel action on this model, and the \
                     model must be enabled for the account under Bedrock > Model access.",
                    self.model, self.region,
                )))
            }
            "ResourceNotFoundException" => {
                return EmbeddingError::Api(ApiError::ModelUnavailable(format!(
                    "Model '{}' was not found in region {}: {detail}\n\
                     Check the model id, and confirm the model is offered in this region \
                     (availability differs per region).",
                    self.model, self.region,
                )))
            }
            "ModelNotReadyException" => {
                return EmbeddingError::Api(ApiError::ServerError {
                    status,
                    message: format!("Model {} is not ready yet: {detail}", self.model),
                })
            }
            "ValidationException" => {
                return EmbeddingError::Api(ApiError::BadRequest(format!(
                    "Bedrock rejected the request for {}: {detail}\n\
                     If this mentions dimensions, check MAPROOM_EMBEDDING_DIMENSION against \
                     what the model supports.",
                    self.model,
                )))
            }
            _ => {}
        }

        match status {
            401 | 403 => EmbeddingError::Api(ApiError::Authentication(format!(
                "Bedrock authentication failed (HTTP {status}): {detail}\n\
                 Verify credentials with: aws sts get-caller-identity"
            ))),
            404 => EmbeddingError::Api(ApiError::ModelUnavailable(format!(
                "Model '{}' not found in region {}: {detail}",
                self.model, self.region
            ))),
            429 => EmbeddingError::Api(ApiError::RateLimit {
                retry_after_ms: retry_after_ms.unwrap_or(1000),
            }),
            400 => EmbeddingError::Api(ApiError::BadRequest(detail)),
            500..=599 => EmbeddingError::Api(ApiError::ServerError {
                status,
                message: detail,
            }),
            _ => EmbeddingError::Api(ApiError::InvalidResponse(format!(
                "Bedrock returned HTTP {status}: {detail}"
            ))),
        }
    }

    /// Truncate texts to the family's token ceiling.
    fn truncate(&self, texts: Vec<String>) -> Vec<String> {
        let counter = TokenCounter::new();
        let limit = self.family.max_tokens_per_text();
        texts
            .into_iter()
            .map(|text| {
                let truncated = counter.truncate_to_limit(&text, limit);
                if truncated.len() < text.len() {
                    tracing::warn!(
                        "Truncated embedding text from {} to {} chars (max {} tokens for {})",
                        text.len(),
                        truncated.len(),
                        limit,
                        self.model,
                    );
                }
                truncated
            })
            .collect()
    }

    /// Split a batch across concurrent `InvokeModel` calls, preserving order.
    ///
    /// Sub-batch size is capped by the family's per-request limit — 1 for Titan,
    /// 96 for Cohere — so this is the only path that turns a large scan into
    /// bounded concurrency rather than a serial walk.
    async fn embed_batch_parallel(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        let total = texts.len();
        let sub_batch_size = self
            .parallel_config
            .sub_batch_size
            .min(self.family.max_batch_size())
            .max(1);

        let sub_batches: Vec<Vec<String>> = texts
            .chunks(sub_batch_size)
            .map(<[String]>::to_vec)
            .collect();
        let batch_count = sub_batches.len();

        tracing::info!(
            "Bedrock batch embedding: {} texts in {} requests (size: {}, concurrency: {})",
            total,
            batch_count,
            sub_batch_size,
            self.parallel_config.max_concurrency,
        );
        let started = std::time::Instant::now();

        let handles: Vec<_> = sub_batches
            .into_iter()
            .enumerate()
            .map(|(index, batch)| {
                let semaphore = self.semaphore.clone();
                let this = self.clone();
                tokio::spawn(async move {
                    let _permit = match semaphore.acquire().await {
                        Ok(permit) => permit,
                        // Only reachable if the semaphore is closed, which we
                        // never do; treat as a hard failure rather than a panic.
                        Err(error) => {
                            return (
                                index,
                                Err(EmbeddingError::Other(format!(
                                    "Bedrock concurrency semaphore closed: {error}"
                                ))),
                            )
                        }
                    };
                    (index, this.invoke_with_retry(batch).await)
                })
            })
            .collect();

        let mut results: Vec<(usize, Result<Vec<Vector>, EmbeddingError>)> =
            Vec::with_capacity(batch_count);
        for handle in handles {
            let (index, result) = handle.await.map_err(|error| {
                EmbeddingError::Api(ApiError::InvalidResponse(format!(
                    "Bedrock task join error: {error}"
                )))
            })?;
            results.push((index, result));
        }
        results.sort_by_key(|(index, _)| *index);

        let mut embeddings = Vec::with_capacity(total);
        for (index, result) in results {
            embeddings.extend(result.map_err(|error| {
                // Preserve the typed error so callers can still classify it;
                // only annotate which sub-batch failed.
                tracing::error!("Bedrock sub-batch {index} failed: {error}");
                error
            })?);
        }

        let elapsed = started.elapsed();
        tracing::info!(
            "Bedrock batch completed in {:.2}s ({:.1} texts/sec)",
            elapsed.as_secs_f64(),
            total as f64 / elapsed.as_secs_f64().max(f64::EPSILON),
        );

        Ok(embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for BedrockProvider {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::InvalidInput(
                "Cannot embed empty text".to_string(),
            ));
        }
        let texts = self.truncate(vec![text]);
        let mut embeddings = self.invoke_with_retry(texts).await?;
        embeddings.pop().ok_or_else(|| {
            EmbeddingError::Api(ApiError::InvalidResponse(
                "Bedrock returned no embedding".to_string(),
            ))
        })
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let texts = self.truncate(texts);

        // A batch that fits one request skips the spawn/semaphore machinery.
        if !self.parallel_config.enabled || texts.len() <= self.family.max_batch_size() {
            return self.invoke_with_retry(texts).await;
        }
        self.embed_batch_parallel(texts).await
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn provider_name(&self) -> &'static str {
        "bedrock"
    }

    fn distinguishes_queries(&self) -> bool {
        // Only Cohere's Bedrock payload carries an input_type. Titan has no
        // such parameter, so claiming the distinction would only cost callers
        // a redundant cache namespace for identical vectors.
        matches!(self.family, ModelFamily::CohereV3)
    }

    async fn embed_query(&self, text: String) -> Result<Vector, EmbeddingError> {
        if !self.distinguishes_queries() {
            return self.embed(text).await;
        }
        self.for_queries().embed(text).await
    }

    fn metrics(&self) -> Option<ProviderMetrics> {
        // `metrics()` is sync but the state is behind an async lock; a blocking
        // read here would deadlock inside the runtime, so use the non-blocking
        // path and report nothing if a writer holds it at this instant.
        self.metrics.try_read().ok().map(|metrics| metrics.clone())
    }
}

/// Strip inference-profile and ARN decoration from a model id.
///
/// Bedrock accepts several forms for the same underlying model:
/// - `amazon.titan-embed-text-v2:0` — the plain model id
/// - `us.amazon.titan-embed-text-v2:0` — a cross-region inference profile
/// - `arn:aws:bedrock:us-east-1:123:inference-profile/us.amazon.titan-…` — an ARN
///
/// Family detection needs the bare id, so this reduces all three to the same
/// string. The *original* id is still what gets sent to Bedrock.
fn normalize_model_id(model: &str) -> String {
    // For an ARN, everything through the last '/' is resource-path decoration.
    let without_arn = model.rsplit('/').next().unwrap_or(model);

    // Cross-region inference profiles prefix a geography to the model id.
    for prefix in ["us.", "eu.", "apac.", "us-gov.", "ca.", "sa.", "jp.", "au."] {
        if let Some(stripped) = without_arn.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    without_arn.to_string()
}

/// Determine the model family from a model id, if it is one we know.
pub fn infer_model_family(model: &str) -> Option<ModelFamily> {
    let normalized = normalize_model_id(model);
    if normalized.starts_with("amazon.titan-embed-text-v2") {
        Some(ModelFamily::TitanV2)
    } else if normalized.starts_with("amazon.titan-embed-text-v1")
        || normalized.starts_with("amazon.titan-embed-g1-text")
    {
        Some(ModelFamily::TitanV1)
    } else if normalized.starts_with("cohere.embed-") {
        Some(ModelFamily::CohereV3)
    } else {
        None
    }
}

/// Infer the embedding dimension from a Bedrock model id.
///
/// Returns `None` for unrecognized ids, which forces the operator to set
/// `MAPROOM_EMBEDDING_DIMENSION` rather than silently indexing at the wrong
/// width — a mistake that only surfaces later as empty vector-search results.
pub fn infer_bedrock_dimension(model: &str) -> Option<usize> {
    infer_model_family(model).map(|family| family.default_dimension())
}

/// Widths the model itself can emit.
///
/// Titan v2 is the only Bedrock embedding model with a selectable output width
/// (Matryoshka truncation); the others are fixed.
fn model_supported_dimensions(family: ModelFamily) -> &'static [usize] {
    match family {
        ModelFamily::TitanV2 => &[256, 512, 1024],
        ModelFamily::TitanV1 => &[1536],
        ModelFamily::CohereV3 => &[1024],
    }
}

/// Widths maproom can actually persist.
///
/// Vector storage is per-dimension on both backends — SQLite needs a fixed
/// dimension at virtual-table creation (`vec_code`, `vec_code_1024`,
/// `vec_code_768`) and Postgres stores each width in its own typed column
/// (`embedding_768`/`embedding_1024`/`embedding_1536`). A dimension outside
/// this set embeds fine and then fails at insert.
///
/// Kept in sync with `db::select_columns_for_dimension` and
/// `db::postgres::embeddings`; `bedrock_dimensions_are_all_storable` pins it.
const STORABLE_DIMENSIONS: &[usize] = &[768, 1024, 1536];

/// Reject a dimension the model cannot produce, or that maproom cannot store.
///
/// Both checks happen here rather than at insert time. Titan v2 genuinely
/// supports 256 and 512, but maproom has no column set for them — accepting
/// such a config would embed an entire repository before failing on the first
/// write, which is a far worse experience than refusing at startup.
fn validate_dimension(
    family: ModelFamily,
    model: &str,
    dimension: usize,
) -> Result<(), EmbeddingError> {
    let model_widths = model_supported_dimensions(family);
    if !model_widths.contains(&dimension) {
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "MAPROOM_EMBEDDING_DIMENSION".to_string(),
            reason: format!(
                "{model} ({}) supports {:?} dimensions, not {dimension}. \
                 Unset MAPROOM_EMBEDDING_DIMENSION to use the model's default.",
                family.label(),
                model_widths,
            ),
        }));
    }

    if !STORABLE_DIMENSIONS.contains(&dimension) {
        let usable: Vec<usize> = model_widths
            .iter()
            .copied()
            .filter(|width| STORABLE_DIMENSIONS.contains(width))
            .collect();
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "MAPROOM_EMBEDDING_DIMENSION".to_string(),
            reason: format!(
                "{model} can emit {dimension}-dimensional vectors, but maproom stores \
                 embeddings in per-dimension tables and supports only {STORABLE_DIMENSIONS:?}. \
                 Usable widths for this model: {usable:?}. \
                 Rejecting now rather than after a full scan, since the failure would \
                 otherwise appear on the first database write."
            ),
        }));
    }

    Ok(())
}

/// Error for a model id that matches no known family.
fn unknown_model_error(model: &str) -> EmbeddingError {
    EmbeddingError::Config(ConfigError::InvalidValue {
        field: "MAPROOM_EMBEDDING_MODEL".to_string(),
        reason: format!(
            "Unrecognized Bedrock embedding model '{model}'.\n\
             Known models:\n\
             \x20 amazon.titan-embed-text-v2:0   (1024 dims, default)\n\
             \x20 amazon.titan-embed-text-v1     (1536 dims)\n\
             \x20 cohere.embed-english-v3        (1024 dims)\n\
             \x20 cohere.embed-multilingual-v3   (1024 dims)\n\
             To use a model this build does not know — including a \
             provisioned-throughput ARN — set MAPROOM_EMBEDDING_DIMENSION as well."
        ),
    })
}

/// Resolve the Bedrock Runtime endpoint for a region.
///
/// Honors, highest precedence first:
/// 1. `MAPROOM_BEDROCK_ENDPOINT_URL` — maproom-specific override.
/// 2. `AWS_ENDPOINT_URL_BEDROCK_RUNTIME` — the SDK-standard per-service override.
/// 3. `AWS_ENDPOINT_URL` — the SDK-standard global override.
/// 4. `MAPROOM_BEDROCK_USE_FIPS=true` — the FIPS 140-3 endpoint for the region.
/// 5. The regional public endpoint.
///
/// Overrides matter for enterprises: PrivateLink/VPC-endpoint DNS names and
/// egress-proxy URLs both arrive this way.
fn resolve_endpoint(region: &str) -> String {
    for variable in [
        "MAPROOM_BEDROCK_ENDPOINT_URL",
        "AWS_ENDPOINT_URL_BEDROCK_RUNTIME",
        "AWS_ENDPOINT_URL",
    ] {
        if let Ok(value) = std::env::var(variable) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                tracing::info!("Using Bedrock endpoint {trimmed} from {variable}");
                return trimmed.trim_end_matches('/').to_string();
            }
        }
    }

    if std::env::var("MAPROOM_BEDROCK_USE_FIPS")
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("true"))
    {
        return format!("https://bedrock-runtime-fips.{region}.amazonaws.com");
    }

    format!("https://bedrock-runtime.{region}.amazonaws.com")
}

/// Split an endpoint URL into its host authority and any base path prefix.
///
/// `https://bedrock-runtime.us-east-1.amazonaws.com` splits into
/// `("bedrock-runtime.us-east-1.amazonaws.com", "")`, while an egress proxy at
/// `https://proxy.internal:8443/aws/bedrock` splits into
/// `("proxy.internal:8443", "/aws/bedrock")`.
///
/// Both halves are needed separately: the authority is what gets signed as the
/// `host` header, and the base path must prefix the canonical URI or the
/// signature will not match what the server computes.
fn split_endpoint(endpoint: &str) -> (String, String) {
    let without_scheme = endpoint
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    match without_scheme.find('/') {
        None => (without_scheme.to_string(), String::new()),
        Some(index) => {
            let (host, base_path) = without_scheme.split_at(index);
            (host.to_string(), base_path.trim_end_matches('/').to_string())
        }
    }
}

/// The scheme prefix of an endpoint, defaulting to HTTPS.
///
/// Only a plaintext override — a local mock or an in-cluster proxy — is
/// `http://`; anything unmarked is treated as TLS.
fn endpoint_scheme(endpoint: &str) -> &'static str {
    if endpoint.starts_with("http://") {
        "http://"
    } else {
        "https://"
    }
}

/// Estimate token usage for models that do not report it.
///
/// Uses the same cl100k tokenizer the context assembler uses. This only feeds
/// the cost estimate, never a request.
fn estimate_tokens(texts: &[String]) -> u64 {
    let counter = TokenCounter::new();
    texts
        .iter()
        .map(|text| counter.count(text).unwrap_or(text.len() / 4) as u64)
        .sum()
}

/// Pull the human-readable message out of a Bedrock error body.
fn extract_bedrock_message(body: &str) -> String {
    #[derive(Deserialize)]
    struct BedrockError {
        message: Option<String>,
        #[serde(rename = "Message")]
        message_capitalized: Option<String>,
    }
    serde_json::from_str::<BedrockError>(body)
        .ok()
        .and_then(|parsed| parsed.message.or(parsed.message_capitalized))
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                "no error detail returned".to_string()
            } else {
                trimmed.chars().take(500).collect()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_families_are_inferred_from_plain_ids() {
        assert_eq!(
            infer_model_family("amazon.titan-embed-text-v2:0"),
            Some(ModelFamily::TitanV2)
        );
        assert_eq!(
            infer_model_family("amazon.titan-embed-text-v1"),
            Some(ModelFamily::TitanV1)
        );
        assert_eq!(
            infer_model_family("amazon.titan-embed-g1-text-02"),
            Some(ModelFamily::TitanV1)
        );
        assert_eq!(
            infer_model_family("cohere.embed-english-v3"),
            Some(ModelFamily::CohereV3)
        );
        assert_eq!(
            infer_model_family("cohere.embed-multilingual-v3"),
            Some(ModelFamily::CohereV3)
        );
        assert_eq!(infer_model_family("anthropic.claude-3-sonnet"), None);
    }

    #[test]
    fn cross_region_inference_profiles_resolve_to_the_base_model() {
        // `us.` and friends prefix the model id when routing through a
        // cross-region inference profile.
        assert_eq!(
            infer_model_family("us.amazon.titan-embed-text-v2:0"),
            Some(ModelFamily::TitanV2)
        );
        assert_eq!(
            infer_model_family("eu.cohere.embed-multilingual-v3"),
            Some(ModelFamily::CohereV3)
        );
        assert_eq!(
            infer_model_family("apac.amazon.titan-embed-text-v2:0"),
            Some(ModelFamily::TitanV2)
        );
    }

    #[test]
    fn arn_model_ids_resolve_to_the_base_model() {
        assert_eq!(
            infer_model_family(
                "arn:aws:bedrock:us-east-1:123456789012:inference-profile/us.amazon.titan-embed-text-v2:0"
            ),
            Some(ModelFamily::TitanV2)
        );
    }

    #[test]
    fn dimension_inference_matches_documented_model_widths() {
        assert_eq!(
            infer_bedrock_dimension("amazon.titan-embed-text-v2:0"),
            Some(1024)
        );
        assert_eq!(
            infer_bedrock_dimension("amazon.titan-embed-text-v1"),
            Some(1536)
        );
        assert_eq!(infer_bedrock_dimension("cohere.embed-english-v3"), Some(1024));
        assert_eq!(
            infer_bedrock_dimension("some.future-model"),
            None,
            "an unknown model must force an explicit dimension, not guess"
        );
    }

    #[test]
    fn batch_limits_match_each_api_shape() {
        // Titan's payload has one `inputText`; Cohere's takes a `texts` array.
        assert_eq!(ModelFamily::TitanV1.max_batch_size(), 1);
        assert_eq!(ModelFamily::TitanV2.max_batch_size(), 1);
        assert_eq!(ModelFamily::CohereV3.max_batch_size(), 96);
    }

    #[test]
    fn titan_v2_rejects_widths_the_model_cannot_emit() {
        // 768 is a perfectly storable width, but Titan v2 does not produce it.
        let error = validate_dimension(ModelFamily::TitanV2, "m", 768).unwrap_err();
        assert!(error.to_string().contains("768"));
        assert!(
            error.to_string().contains("supports"),
            "should be reported as a model-capability problem"
        );
    }

    #[test]
    fn titan_v2_matryoshka_widths_maproom_cannot_store_are_rejected_up_front() {
        // Titan v2 really does emit 256 and 512, but maproom's vector storage
        // is per-dimension and has no table for either. Accepting the config
        // would embed a whole repository before failing on the first write.
        assert!(validate_dimension(ModelFamily::TitanV2, "m", 1024).is_ok());
        for dimension in [256, 512] {
            let error = validate_dimension(ModelFamily::TitanV2, "m", dimension).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains("maproom stores"),
                "should be reported as a storage limit, not a model limit: {message}"
            );
            assert!(
                message.contains("[1024]"),
                "should name the width that does work: {message}"
            );
        }
    }

    #[test]
    fn bedrock_dimensions_are_all_storable() {
        // Every default width this provider can select must map to a real
        // column set. If someone adds a model family with a novel dimension,
        // this fails here rather than at insert time.
        for family in [
            ModelFamily::TitanV1,
            ModelFamily::TitanV2,
            ModelFamily::CohereV3,
        ] {
            let dimension = family.default_dimension();
            assert!(
                STORABLE_DIMENSIONS.contains(&dimension),
                "{} defaults to {dimension}, which maproom cannot store",
                family.label()
            );
            assert!(
                crate::db::select_columns_for_dimension(dimension).is_ok(),
                "{dimension} must resolve to a column set in db::columns"
            );
        }
    }

    #[test]
    fn fixed_width_models_reject_other_dimensions() {
        assert!(validate_dimension(ModelFamily::TitanV1, "m", 1536).is_ok());
        assert!(validate_dimension(ModelFamily::TitanV1, "m", 1024).is_err());
        assert!(validate_dimension(ModelFamily::CohereV3, "m", 1024).is_ok());
        assert!(validate_dimension(ModelFamily::CohereV3, "m", 512).is_err());
    }

    #[test]
    fn unknown_model_error_lists_the_supported_models() {
        let message = unknown_model_error("mistral.embed").to_string();
        assert!(message.contains("amazon.titan-embed-text-v2:0"));
        assert!(message.contains("cohere.embed-english-v3"));
        assert!(message.contains("MAPROOM_EMBEDDING_DIMENSION"));
    }

    #[test]
    fn cohere_input_type_distinguishes_documents_from_queries() {
        assert_eq!(InputType::Document.as_cohere_str(), "search_document");
        assert_eq!(InputType::Query.as_cohere_str(), "search_query");
    }

    #[test]
    fn endpoint_splits_into_host_and_base_path() {
        assert_eq!(
            split_endpoint("https://bedrock-runtime.us-east-1.amazonaws.com"),
            (
                "bedrock-runtime.us-east-1.amazonaws.com".to_string(),
                String::new()
            )
        );
        // An egress proxy may mount Bedrock under a subpath. The prefix belongs
        // in the signed canonical URI, never in the Host header — getting this
        // wrong yields an opaque SignatureDoesNotMatch.
        assert_eq!(
            split_endpoint("https://proxy.internal:8443/aws/bedrock"),
            ("proxy.internal:8443".to_string(), "/aws/bedrock".to_string())
        );
        // A plaintext local mock keeps its port in the authority.
        assert_eq!(
            split_endpoint("http://127.0.0.1:41723"),
            ("127.0.0.1:41723".to_string(), String::new())
        );
        // A trailing slash must not survive as an empty path segment, which
        // would change the canonical URI and break the signature.
        assert_eq!(
            split_endpoint("https://host.example/base/"),
            ("host.example".to_string(), "/base".to_string())
        );
    }

    #[test]
    fn endpoint_scheme_defaults_to_https() {
        assert_eq!(endpoint_scheme("http://127.0.0.1:1"), "http://");
        assert_eq!(endpoint_scheme("https://x.example"), "https://");
        assert_eq!(endpoint_scheme("x.example"), "https://");
    }

    #[test]
    fn bedrock_error_message_is_extracted_from_json() {
        assert_eq!(
            extract_bedrock_message(r#"{"message":"Model access denied"}"#),
            "Model access denied"
        );
        assert_eq!(
            extract_bedrock_message(r#"{"Message":"Capitalized form"}"#),
            "Capitalized form"
        );
        assert_eq!(
            extract_bedrock_message("plain text failure"),
            "plain text failure"
        );
        assert_eq!(
            extract_bedrock_message(""),
            "no error detail returned"
        );
    }

    #[test]
    fn titan_prices_below_cohere() {
        // Guards the cost table against a copy-paste swap.
        assert!(
            ModelFamily::TitanV2.price_per_1k_tokens() < ModelFamily::CohereV3.price_per_1k_tokens()
        );
        assert_eq!(ModelFamily::TitanV2.price_per_1k_tokens(), 0.00002);
    }

    mod endpoint {
        use super::*;
        use serial_test::serial;

        fn clear() {
            for variable in [
                "MAPROOM_BEDROCK_ENDPOINT_URL",
                "AWS_ENDPOINT_URL_BEDROCK_RUNTIME",
                "AWS_ENDPOINT_URL",
                "MAPROOM_BEDROCK_USE_FIPS",
            ] {
                std::env::remove_var(variable);
            }
        }

        #[test]
        #[serial]
        fn defaults_to_the_regional_public_endpoint() {
            clear();
            assert_eq!(
                resolve_endpoint("eu-west-1"),
                "https://bedrock-runtime.eu-west-1.amazonaws.com"
            );
        }

        #[test]
        #[serial]
        fn maproom_override_beats_the_sdk_variables() {
            clear();
            std::env::set_var("AWS_ENDPOINT_URL", "https://global.example");
            std::env::set_var(
                "AWS_ENDPOINT_URL_BEDROCK_RUNTIME",
                "https://service.example",
            );
            std::env::set_var("MAPROOM_BEDROCK_ENDPOINT_URL", "https://vpce.example/");

            assert_eq!(
                resolve_endpoint("us-east-1"),
                "https://vpce.example",
                "trailing slash must be trimmed so path joining stays correct"
            );
            clear();
        }

        #[test]
        #[serial]
        fn service_specific_override_beats_the_global_one() {
            clear();
            std::env::set_var("AWS_ENDPOINT_URL", "https://global.example");
            std::env::set_var("AWS_ENDPOINT_URL_BEDROCK_RUNTIME", "https://svc.example");
            assert_eq!(resolve_endpoint("us-east-1"), "https://svc.example");
            clear();
        }

        #[test]
        #[serial]
        fn fips_endpoint_is_opt_in() {
            clear();
            std::env::set_var("MAPROOM_BEDROCK_USE_FIPS", "true");
            assert_eq!(
                resolve_endpoint("us-east-1"),
                "https://bedrock-runtime-fips.us-east-1.amazonaws.com"
            );
            clear();
        }
    }
}
