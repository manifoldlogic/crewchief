//! Search configuration structs and loading logic.

use crate::cache::CacheConfig;
use crate::config::FeatureFlags;
use crate::search::fusion::FusionWeights;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum SearchConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid YAML syntax: {0}")]
    InvalidYaml(String),

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),

    #[error("Environment variable parsing error: {0}")]
    EnvVarError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Complete search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SearchConfig {
    /// Embedding configuration
    pub embedding: EmbeddingConfig,

    /// Fusion configuration
    pub fusion: FusionConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,

    /// Index configuration
    pub index: IndexConfig,

    /// Feature flags
    pub feature_flags: FeatureFlags,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,

    /// Indexing configuration (PERF_OPT-5002)
    #[serde(default)]
    pub indexing: IndexingConfig,

    /// Database configuration (PERF_OPT-5002)
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Runtime configuration (PERF_OPT-5002)
    #[serde(default)]
    pub runtime: RuntimeConfig,

    /// Buffer configuration (PERF_OPT-5002)
    #[serde(default)]
    pub buffers: BufferConfig,
}

impl SearchConfig {
    /// Load configuration from the default path.
    ///
    /// Searches for configuration file in:
    /// 1. `./config/maproom-search.yml` (relative to current directory)
    /// 2. `../config/maproom-search.yml` (relative to binary location)
    /// 3. `/etc/maproom/maproom-search.yml` (system-wide)
    ///
    /// Environment variables override file values.
    pub async fn load_default() -> Result<Self> {
        let default_paths = vec![
            PathBuf::from("config/maproom-search.yml"),
            PathBuf::from("../config/maproom-search.yml"),
            PathBuf::from("/etc/maproom/maproom-search.yml"),
        ];

        for path in default_paths {
            if path.exists() {
                info!("Loading configuration from: {}", path.display());
                return Self::load_from_file(&path).await;
            }
        }

        warn!("No configuration file found, using defaults");
        Ok(Self::default())
    }

    /// Load configuration from a specific file path.
    ///
    /// Environment variables override file values.
    pub async fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(SearchConfigError::FileNotFound(path.display().to_string()).into());
        }

        let contents = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read configuration file")?;

        let mut config: SearchConfig = serde_yaml::from_str(&contents)
            .map_err(|e| SearchConfigError::InvalidYaml(e.to_string()))?;

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        // Validate configuration
        config.validate()?;

        info!("Configuration loaded successfully from: {}", path.display());
        debug!("Active configuration: {:#?}", config);

        Ok(config)
    }

    /// Apply environment variable overrides.
    ///
    /// Environment variables follow the pattern: MAPROOM_SEARCH_<SECTION>_<KEY>
    fn apply_env_overrides(&mut self) -> Result<()> {
        // Embedding overrides
        if let Ok(provider) = std::env::var("MAPROOM_SEARCH_EMBEDDING_PROVIDER") {
            self.embedding.provider = provider;
            debug!("Override: embedding.provider = {}", self.embedding.provider);
        }
        if let Ok(model) = std::env::var("MAPROOM_SEARCH_EMBEDDING_MODEL_NAME") {
            self.embedding.model_name = model;
            debug!("Override: embedding.model_name = {}", self.embedding.model_name);
        }
        if let Ok(dim) = std::env::var("MAPROOM_SEARCH_EMBEDDING_DIMENSION") {
            self.embedding.dimension = dim
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_EMBEDDING_DIMENSION")?;
            debug!("Override: embedding.dimension = {}", self.embedding.dimension);
        }
        if let Ok(size) = std::env::var("MAPROOM_SEARCH_EMBEDDING_CACHE_SIZE") {
            self.embedding.cache_size = size
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_EMBEDDING_CACHE_SIZE")?;
            debug!("Override: embedding.cache_size = {}", self.embedding.cache_size);
        }
        if let Ok(ttl) = std::env::var("MAPROOM_SEARCH_EMBEDDING_CACHE_TTL_SECONDS") {
            self.embedding.cache_ttl_seconds = ttl
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_EMBEDDING_CACHE_TTL_SECONDS")?;
            debug!(
                "Override: embedding.cache_ttl_seconds = {}",
                self.embedding.cache_ttl_seconds
            );
        }

        // Fusion overrides
        if let Ok(method) = std::env::var("MAPROOM_SEARCH_FUSION_METHOD") {
            self.fusion.method = FusionMethod::from_str(&method)?;
            debug!("Override: fusion.method = {:?}", self.fusion.method);
        }
        if let Ok(k) = std::env::var("MAPROOM_SEARCH_FUSION_RRF_K") {
            self.fusion.rrf_k = k
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_RRF_K")?;
            debug!("Override: fusion.rrf_k = {}", self.fusion.rrf_k);
        }

        // Fusion weight overrides
        if let Ok(fts) = std::env::var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS") {
            self.fusion.weights.fts = fts
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_WEIGHTS_FTS")?;
            debug!("Override: fusion.weights.fts = {}", self.fusion.weights.fts);
        }
        if let Ok(vector) = std::env::var("MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR") {
            self.fusion.weights.vector = vector
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR")?;
            debug!(
                "Override: fusion.weights.vector = {}",
                self.fusion.weights.vector
            );
        }
        if let Ok(graph) = std::env::var("MAPROOM_SEARCH_FUSION_WEIGHTS_GRAPH") {
            self.fusion.weights.graph = graph
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_WEIGHTS_GRAPH")?;
            debug!(
                "Override: fusion.weights.graph = {}",
                self.fusion.weights.graph
            );
        }
        if let Ok(recency) = std::env::var("MAPROOM_SEARCH_FUSION_WEIGHTS_RECENCY") {
            self.fusion.weights.recency = recency
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_WEIGHTS_RECENCY")?;
            debug!(
                "Override: fusion.weights.recency = {}",
                self.fusion.weights.recency
            );
        }
        if let Ok(churn) = std::env::var("MAPROOM_SEARCH_FUSION_WEIGHTS_CHURN") {
            self.fusion.weights.churn = churn
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FUSION_WEIGHTS_CHURN")?;
            debug!(
                "Override: fusion.weights.churn = {}",
                self.fusion.weights.churn
            );
        }

        // Performance overrides
        if let Ok(max_candidates) =
            std::env::var("MAPROOM_SEARCH_PERFORMANCE_MAX_CANDIDATES_PER_METHOD")
        {
            self.performance.max_candidates_per_method = max_candidates
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_PERFORMANCE_MAX_CANDIDATES_PER_METHOD")?;
            debug!(
                "Override: performance.max_candidates_per_method = {}",
                self.performance.max_candidates_per_method
            );
        }
        if let Ok(final_limit) = std::env::var("MAPROOM_SEARCH_PERFORMANCE_FINAL_RESULT_LIMIT") {
            self.performance.final_result_limit = final_limit
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_PERFORMANCE_FINAL_RESULT_LIMIT")?;
            debug!(
                "Override: performance.final_result_limit = {}",
                self.performance.final_result_limit
            );
        }
        if let Ok(timeout) = std::env::var("MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS") {
            self.performance.timeout_ms = timeout
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS")?;
            debug!(
                "Override: performance.timeout_ms = {}",
                self.performance.timeout_ms
            );
        }
        if let Ok(parallel) = std::env::var("MAPROOM_SEARCH_PERFORMANCE_PARALLEL_EXECUTION") {
            self.performance.parallel_execution = parallel
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_PERFORMANCE_PARALLEL_EXECUTION")?;
            debug!(
                "Override: performance.parallel_execution = {}",
                self.performance.parallel_execution
            );
        }

        // Index overrides
        if let Ok(lists) = std::env::var("MAPROOM_SEARCH_INDEX_IVFFLAT_LISTS") {
            self.index.ivfflat_lists = lists
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_INDEX_IVFFLAT_LISTS")?;
            debug!("Override: index.ivfflat_lists = {}", self.index.ivfflat_lists);
        }
        if let Ok(probes) = std::env::var("MAPROOM_SEARCH_INDEX_IVFFLAT_PROBES") {
            self.index.ivfflat_probes = probes
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_INDEX_IVFFLAT_PROBES")?;
            debug!(
                "Override: index.ivfflat_probes = {}",
                self.index.ivfflat_probes
            );
        }
        if let Ok(refresh) = std::env::var("MAPROOM_SEARCH_INDEX_REFRESH_INTERVAL_SECONDS") {
            self.index.refresh_interval_seconds = refresh
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_INDEX_REFRESH_INTERVAL_SECONDS")?;
            debug!(
                "Override: index.refresh_interval_seconds = {}",
                self.index.refresh_interval_seconds
            );
        }

        // Feature flag overrides
        if let Ok(vector) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH") {
            self.feature_flags.enable_vector_search = vector
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH")?;
            debug!(
                "Override: feature_flags.enable_vector_search = {}",
                self.feature_flags.enable_vector_search
            );
        }
        if let Ok(hybrid) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HYBRID_FUSION") {
            self.feature_flags.enable_hybrid_fusion = hybrid
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HYBRID_FUSION")?;
            debug!(
                "Override: feature_flags.enable_hybrid_fusion = {}",
                self.feature_flags.enable_hybrid_fusion
            );
        }
        if let Ok(graph) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS") {
            self.feature_flags.enable_graph_signals = graph
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS")?;
            debug!(
                "Override: feature_flags.enable_graph_signals = {}",
                self.feature_flags.enable_graph_signals
            );
        }
        if let Ok(temporal) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_TEMPORAL_SIGNALS")
        {
            self.feature_flags.enable_temporal_signals = temporal
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_TEMPORAL_SIGNALS")?;
            debug!(
                "Override: feature_flags.enable_temporal_signals = {}",
                self.feature_flags.enable_temporal_signals
            );
        }
        if let Ok(cache) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUERY_CACHE") {
            self.feature_flags.enable_query_cache = cache
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUERY_CACHE")?;
            debug!(
                "Override: feature_flags.enable_query_cache = {}",
                self.feature_flags.enable_query_cache
            );
        }
        if let Ok(hot_reload) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HOT_RELOAD") {
            self.feature_flags.enable_hot_reload = hot_reload
                .parse()
                .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HOT_RELOAD")?;
            debug!(
                "Override: feature_flags.enable_hot_reload = {}",
                self.feature_flags.enable_hot_reload
            );
        }

        Ok(())
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate embedding config
        self.embedding.validate()?;

        // Validate fusion config
        self.fusion.validate()?;

        // Validate performance config
        self.performance.validate()?;

        // Validate index config
        self.index.validate()?;

        // Validate indexing config (PERF_OPT-5002)
        self.indexing.validate()?;

        // Validate database config (PERF_OPT-5002)
        self.database.validate()?;

        // Validate runtime config (PERF_OPT-5002)
        self.runtime.validate()?;

        // Validate buffer config (PERF_OPT-5002)
        self.buffers.validate()?;

        Ok(())
    }

    /// Get a summary of active environment variable overrides.
    pub fn get_env_overrides() -> Vec<(String, String)> {
        std::env::vars()
            .filter(|(k, _)| k.starts_with("MAPROOM_SEARCH_"))
            .collect()
    }
}


/// Embedding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Embedding provider (openai, cohere, local)
    pub provider: String,

    /// Model name
    pub model_name: String,

    /// Embedding dimension
    pub dimension: usize,

    /// Cache size (number of embeddings)
    pub cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model_name: "text-embedding-3-small".to_string(),
            dimension: 1536,
            cache_size: 10000,
            cache_ttl_seconds: 3600,
        }
    }
}

impl EmbeddingConfig {
    /// Validate embedding configuration.
    pub fn validate(&self) -> Result<()> {
        if self.provider.is_empty() {
            return Err(SearchConfigError::ValidationError(
                "Embedding provider cannot be empty".to_string(),
            )
            .into());
        }

        if self.model_name.is_empty() {
            return Err(SearchConfigError::ValidationError(
                "Embedding model name cannot be empty".to_string(),
            )
            .into());
        }

        if self.dimension == 0 {
            return Err(SearchConfigError::ValidationError(
                "Embedding dimension must be greater than 0".to_string(),
            )
            .into());
        }

        if self.cache_size == 0 {
            warn!("Embedding cache size is 0, caching is disabled");
        }

        Ok(())
    }
}

/// Fusion configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// Fusion method
    pub method: FusionMethod,

    /// RRF k parameter
    pub rrf_k: u32,

    /// Signal weights
    pub weights: FusionWeights,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            method: FusionMethod::RRF,
            rrf_k: 60,
            weights: FusionWeights::default(),
        }
    }
}

impl FusionConfig {
    /// Validate fusion configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate weights
        self.weights
            .validate()
            .context("Invalid fusion weights")?;

        // Warn if weights are not normalized
        if !self.weights.is_normalized() {
            warn!(
                "Fusion weights are not normalized (sum = {}), consider normalizing for predictable behavior",
                self.weights.sum()
            );
        }

        // Validate RRF k parameter
        if self.rrf_k == 0 {
            return Err(SearchConfigError::ValidationError(
                "RRF k parameter must be greater than 0".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

/// Fusion method enumeration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FusionMethod {
    /// Reciprocal Rank Fusion
    RRF,
    /// Weighted average fusion
    Weighted,
    /// Learned fusion (future)
    Learned,
}

impl FusionMethod {
    /// Parse fusion method from string.
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "rrf" => Ok(Self::RRF),
            "weighted" => Ok(Self::Weighted),
            "learned" => Ok(Self::Learned),
            _ => Err(SearchConfigError::ValidationError(format!(
                "Invalid fusion method: {}. Valid options: rrf, weighted, learned",
                s
            ))
            .into()),
        }
    }
}

/// Performance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum candidates per search method
    pub max_candidates_per_method: usize,

    /// Final result limit
    pub final_result_limit: usize,

    /// Query timeout in milliseconds
    pub timeout_ms: u64,

    /// Enable parallel query execution
    pub parallel_execution: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_candidates_per_method: 100,
            final_result_limit: 20,
            timeout_ms: 1000,
            parallel_execution: true,
        }
    }
}

impl PerformanceConfig {
    /// Validate performance configuration.
    pub fn validate(&self) -> Result<()> {
        if self.max_candidates_per_method == 0 {
            return Err(SearchConfigError::ValidationError(
                "max_candidates_per_method must be greater than 0".to_string(),
            )
            .into());
        }

        if self.final_result_limit == 0 {
            return Err(SearchConfigError::ValidationError(
                "final_result_limit must be greater than 0".to_string(),
            )
            .into());
        }

        if self.timeout_ms == 0 {
            warn!("Query timeout is 0, queries will not timeout");
        }

        Ok(())
    }
}

/// Index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// IVFFlat list count
    pub ivfflat_lists: u32,

    /// IVFFlat probe count
    pub ivfflat_probes: u32,

    /// Index refresh interval in seconds
    pub refresh_interval_seconds: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            ivfflat_lists: 100,
            ivfflat_probes: 10,
            refresh_interval_seconds: 3600,
        }
    }
}

impl IndexConfig {
    /// Validate index configuration.
    pub fn validate(&self) -> Result<()> {
        if self.ivfflat_lists == 0 {
            return Err(SearchConfigError::ValidationError(
                "ivfflat_lists must be greater than 0".to_string(),
            )
            .into());
        }

        if self.ivfflat_probes == 0 {
            return Err(SearchConfigError::ValidationError(
                "ivfflat_probes must be greater than 0".to_string(),
            )
            .into());
        }

        if self.ivfflat_probes > self.ivfflat_lists {
            warn!(
                "ivfflat_probes ({}) is greater than ivfflat_lists ({}), this is inefficient",
                self.ivfflat_probes, self.ivfflat_lists
            );
        }

        Ok(())
    }
}

/// Indexing configuration for parallel file processing (PERF_OPT-5002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Number of parallel workers for file indexing
    pub parallel_workers: usize,

    /// Batch size for file processing
    pub batch_size: usize,

    /// Maximum file size to index (bytes)
    pub max_file_size: usize,

    /// Batch size for chunk inserts
    pub chunk_insert_batch_size: usize,

    /// Batch size for edge inserts
    pub edge_insert_batch_size: usize,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            parallel_workers: 8,         // Tuned for 8-core CPU
            batch_size: 50,               // Optimal throughput/memory balance
            max_file_size: 10 * 1024 * 1024, // 10MB
            chunk_insert_batch_size: 100, // Database INSERT batch
            edge_insert_batch_size: 500,  // Edge INSERT batch
        }
    }
}

impl IndexingConfig {
    /// Validate indexing configuration.
    pub fn validate(&self) -> Result<()> {
        if self.parallel_workers == 0 {
            return Err(SearchConfigError::ValidationError(
                "parallel_workers must be greater than 0".to_string(),
            )
            .into());
        }

        if self.batch_size == 0 {
            return Err(SearchConfigError::ValidationError(
                "batch_size must be greater than 0".to_string(),
            )
            .into());
        }

        if self.max_file_size == 0 {
            warn!("max_file_size is 0, no files will be indexed");
        }

        if self.chunk_insert_batch_size == 0 {
            return Err(SearchConfigError::ValidationError(
                "chunk_insert_batch_size must be greater than 0".to_string(),
            )
            .into());
        }

        if self.edge_insert_batch_size == 0 {
            return Err(SearchConfigError::ValidationError(
                "edge_insert_batch_size must be greater than 0".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

/// Database configuration for connection pooling and query tuning (PERF_OPT-5002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Maximum connection pool size
    pub pool_size: usize,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,

    /// Query statement timeout in milliseconds
    pub statement_timeout_ms: u64,

    /// Lock timeout in milliseconds
    pub lock_timeout_ms: u64,

    /// Idle in transaction session timeout in milliseconds
    pub idle_in_transaction_timeout_ms: u64,

    /// PostgreSQL work_mem setting (per-operation memory)
    pub work_mem: String,

    /// Maximum lifetime of a connection in seconds
    pub max_connection_lifetime_secs: u64,

    /// Idle connection timeout in seconds
    pub idle_connection_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            pool_size: 20,                    // Handles concurrent operations
            connection_timeout_ms: 5000,      // 5s to acquire connection
            statement_timeout_ms: 5000,       // 5s query timeout
            lock_timeout_ms: 1000,            // 1s lock wait
            idle_in_transaction_timeout_ms: 30000, // 30s idle in transaction
            work_mem: "256MB".to_string(),    // Per-operation memory
            max_connection_lifetime_secs: 1800, // 30 minutes
            idle_connection_timeout_secs: 600,  // 10 minutes
        }
    }
}

impl DatabaseConfig {
    /// Validate database configuration.
    pub fn validate(&self) -> Result<()> {
        if self.pool_size == 0 {
            return Err(SearchConfigError::ValidationError(
                "pool_size must be greater than 0".to_string(),
            )
            .into());
        }

        if self.pool_size > 100 {
            warn!(
                "pool_size ({}) is very large, this may cause PostgreSQL overhead",
                self.pool_size
            );
        }

        if self.statement_timeout_ms == 0 {
            warn!("statement_timeout_ms is 0, queries will not timeout");
        }

        // Validate work_mem format (e.g., "256MB", "1GB")
        if !self.work_mem.ends_with("MB") && !self.work_mem.ends_with("GB") {
            return Err(SearchConfigError::ValidationError(
                "work_mem must end with 'MB' or 'GB' (e.g., '256MB')".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

/// Runtime configuration for thread pools and async runtime (PERF_OPT-5002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Number of Tokio worker threads
    pub worker_threads: usize,

    /// Maximum blocking threads for spawn_blocking
    pub max_blocking_threads: usize,

    /// Thread stack size in bytes
    pub thread_stack_size: usize,

    /// Enable thread name for debugging
    pub enable_thread_names: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: 8,              // Number of CPU cores
            max_blocking_threads: 16,       // For blocking operations
            thread_stack_size: 2 * 1024 * 1024, // 2MB stack
            enable_thread_names: true,
        }
    }
}

impl RuntimeConfig {
    /// Validate runtime configuration.
    pub fn validate(&self) -> Result<()> {
        if self.worker_threads == 0 {
            return Err(SearchConfigError::ValidationError(
                "worker_threads must be greater than 0".to_string(),
            )
            .into());
        }

        if self.max_blocking_threads == 0 {
            return Err(SearchConfigError::ValidationError(
                "max_blocking_threads must be greater than 0".to_string(),
            )
            .into());
        }

        if self.thread_stack_size < 256 * 1024 {
            warn!(
                "thread_stack_size ({} bytes) is very small, this may cause stack overflows",
                self.thread_stack_size
            );
        }

        Ok(())
    }
}

/// Buffer configuration for I/O operations (PERF_OPT-5002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// File read buffer size in bytes
    pub file_read_buffer: usize,

    /// Database buffer size in bytes
    pub db_buffer: usize,

    /// Parse buffer size in bytes
    pub parse_buffer: usize,

    /// Maximum number of buffers in pool
    pub buffer_pool_size: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            file_read_buffer: 64 * 1024,    // 64KB
            db_buffer: 32 * 1024,           // 32KB
            parse_buffer: 1024 * 1024,      // 1MB
            buffer_pool_size: 100,          // Max pooled buffers
        }
    }
}

impl BufferConfig {
    /// Validate buffer configuration.
    pub fn validate(&self) -> Result<()> {
        if self.file_read_buffer == 0 {
            return Err(SearchConfigError::ValidationError(
                "file_read_buffer must be greater than 0".to_string(),
            )
            .into());
        }

        if self.db_buffer == 0 {
            return Err(SearchConfigError::ValidationError(
                "db_buffer must be greater than 0".to_string(),
            )
            .into());
        }

        if self.parse_buffer == 0 {
            return Err(SearchConfigError::ValidationError(
                "parse_buffer must be greater than 0".to_string(),
            )
            .into());
        }

        if self.buffer_pool_size == 0 {
            warn!("buffer_pool_size is 0, buffer pooling is disabled");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SearchConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.embedding.provider, "openai");
        assert_eq!(config.fusion.method, FusionMethod::RRF);
        assert!(config.feature_flags.enable_vector_search);
    }

    #[test]
    fn test_fusion_method_parsing() {
        assert_eq!(FusionMethod::from_str("rrf").unwrap(), FusionMethod::RRF);
        assert_eq!(
            FusionMethod::from_str("weighted").unwrap(),
            FusionMethod::Weighted
        );
        assert_eq!(
            FusionMethod::from_str("learned").unwrap(),
            FusionMethod::Learned
        );
        assert_eq!(FusionMethod::from_str("RRF").unwrap(), FusionMethod::RRF);
        assert!(FusionMethod::from_str("invalid").is_err());
    }

    #[test]
    fn test_embedding_config_validation() {
        let mut config = EmbeddingConfig::default();
        assert!(config.validate().is_ok());

        config.provider = "".to_string();
        assert!(config.validate().is_err());

        config = EmbeddingConfig::default();
        config.dimension = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_fusion_config_validation() {
        let mut config = FusionConfig::default();
        assert!(config.validate().is_ok());

        config.rrf_k = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_performance_config_validation() {
        let mut config = PerformanceConfig::default();
        assert!(config.validate().is_ok());

        config.max_candidates_per_method = 0;
        assert!(config.validate().is_err());

        config = PerformanceConfig::default();
        config.final_result_limit = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_index_config_validation() {
        let mut config = IndexConfig::default();
        assert!(config.validate().is_ok());

        config.ivfflat_lists = 0;
        assert!(config.validate().is_err());

        config = IndexConfig::default();
        config.ivfflat_probes = 0;
        assert!(config.validate().is_err());
    }
}
