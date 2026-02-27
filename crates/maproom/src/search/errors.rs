//! Structured error taxonomy for search error diagnostics.
//!
//! This module provides a structured error taxonomy that maps internal `PipelineError`
//! types into actionable error diagnostics with context and suggestions. This enables
//! better error reporting to clients and debugging transparency.
//!
//! # Error Type Taxonomy
//!
//! The module defines 6 high-level error types that map to 13+ observed error scenarios:
//!
//! - **EmbeddingProvider**: Embedding provider failures (OpenAI timeout, credentials, Ollama down)
//! - **Database**: Database issues (not indexed, worktree not found, corruption, timeout)
//! - **Validation**: Query validation failures (empty query, too long)
//! - **Timeout**: Search execution timeouts
//! - **NotFound**: Repository or meaningful content not found
//! - **Unknown**: Fallback for unexpected errors
//!
//! # Usage
//!
//! ```no_run
//! use maproom::search::errors::SearchErrorDetails;
//! use maproom::search::PipelineError;
//!
//! fn handle_search_error(error: &PipelineError) {
//!     let details = SearchErrorDetails::from_pipeline_error(error);
//!     println!("Error type: {:?}", details.error_type);
//!     println!("Stage: {:?}", details.stage);
//!     println!("Context: {:?}", details.context);
//!     for suggestion in details.suggestions {
//!         println!("  - {}", suggestion);
//!     }
//! }
//! ```

use crate::embedding::error::{ApiError, ConfigError, EmbeddingError};
use crate::search::pipeline::PipelineError;
use crate::search::query_processor::QueryProcessorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured error details with actionable suggestions.
///
/// This struct provides a structured representation of search errors that can be
/// serialized to JSON and returned to clients for display or logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchErrorDetails {
    /// High-level error type category
    pub error_type: ErrorType,
    /// Pipeline stage where the error occurred
    pub stage: PipelineStage,
    /// Whitelisted context information extracted from the error
    pub context: HashMap<String, String>,
    /// 1-2 actionable suggestions for resolving the error
    pub suggestions: Vec<String>,
}

/// High-level error type categories.
///
/// Maps 13+ observed error scenarios to 6 actionable error types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// Embedding provider failures (OpenAI, Google, Ollama)
    EmbeddingProvider,
    /// Database-related errors (connection, not indexed, corruption)
    Database,
    /// Query validation errors (empty, too long, no content)
    Validation,
    /// Search execution timeout
    Timeout,
    /// Repository or content not found
    NotFound,
    /// Unknown or unexpected errors
    Unknown,
}

/// Pipeline stage where error occurred.
///
/// Helps identify which part of the search pipeline failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    /// Query processing (tokenization, embedding, expansion)
    QueryProcessing,
    /// Search execution (FTS, vector, graph, signals)
    SearchExecution,
    /// Score fusion (combining results from multiple strategies)
    ScoreFusion,
    /// Result assembly (fetching chunk details)
    ResultAssembly,
}

impl SearchErrorDetails {
    /// Convert a `PipelineError` into structured error details with suggestions.
    ///
    /// This function pattern-matches on the error type, extracts whitelisted context,
    /// and generates 1-2 actionable suggestions for each error scenario.
    ///
    /// # Security
    ///
    /// Only whitelisted context keys are extracted to prevent accidental exposure
    /// of sensitive data (API keys, tokens, file paths, user data).
    ///
    /// Whitelisted context keys:
    /// - `provider_error`: Embedding provider error details
    /// - `provider`: Provider name (OpenAI, Google, Ollama)
    /// - `error`: Generic error message
    /// - `message`: Human-readable message
    /// - `length`: Query length for validation errors
    /// - `max_length`: Maximum allowed query length
    /// - `repo_name`: Repository name for not found errors
    /// - `worktree_id`: Worktree identifier
    /// - `timeout_ms`: Timeout duration
    pub fn from_pipeline_error(error: &PipelineError) -> Self {
        match error {
            // Query Processing Errors
            PipelineError::QueryProcessing(query_error) => {
                Self::from_query_processor_error(query_error)
            }

            // Search Execution Errors
            PipelineError::SearchExecution(executor_error) => {
                let error_str = executor_error.to_string();

                // Check for specific error patterns
                if error_str.contains("timeout") || error_str.contains("Timeout") {
                    Self {
                        error_type: ErrorType::Timeout,
                        stage: PipelineStage::SearchExecution,
                        context: HashMap::from([(
                            "message".to_string(),
                            "Search execution timeout".to_string(),
                        )]),
                        suggestions: vec![
                            "Try narrowing your search scope with more specific terms".to_string(),
                            "Use a simpler query or reduce the result limit".to_string(),
                        ],
                    }
                } else if error_str.contains("not indexed")
                    || error_str.contains("not found")
                    || error_str.contains("No such")
                {
                    Self {
                        error_type: ErrorType::NotFound,
                        stage: PipelineStage::SearchExecution,
                        context: HashMap::from([("error".to_string(), error_str.clone())]),
                        suggestions: vec![
                            "Check that the repository is indexed: maproom status".to_string(),
                            "Run a scan to index the repository: maproom scan".to_string(),
                        ],
                    }
                } else {
                    // Generic database error
                    Self {
                        error_type: ErrorType::Database,
                        stage: PipelineStage::SearchExecution,
                        context: HashMap::from([("error".to_string(), error_str)]),
                        suggestions: vec![
                            "Check database connectivity and permissions".to_string(),
                            "Verify repository is indexed: maproom status".to_string(),
                        ],
                    }
                }
            }

            // Database Errors
            PipelineError::Database(db_error) => {
                let db_error_lower = db_error.to_lowercase();

                // Check if this is a "not indexed" error (specific case)
                if db_error_lower.contains("not indexed") {
                    let (context, suggestions) = Self::database_error_details(db_error);
                    Self {
                        error_type: ErrorType::NotFound,
                        stage: PipelineStage::SearchExecution,
                        context,
                        suggestions,
                    }
                } else {
                    let (context, suggestions) = Self::database_error_details(db_error);
                    Self {
                        error_type: ErrorType::Database,
                        stage: PipelineStage::SearchExecution,
                        context,
                        suggestions,
                    }
                }
            }

            // Result Assembly Errors
            PipelineError::Assembly(assembly_error) => {
                let error_str = assembly_error.to_string();

                if error_str.contains("not found") || error_str.contains("missing") {
                    Self {
                        error_type: ErrorType::NotFound,
                        stage: PipelineStage::ResultAssembly,
                        context: HashMap::from([("message".to_string(), error_str)]),
                        suggestions: vec![
                            "The search index may be stale or corrupted".to_string(),
                            "Try re-scanning the repository: maproom scan".to_string(),
                        ],
                    }
                } else {
                    Self {
                        error_type: ErrorType::Database,
                        stage: PipelineStage::ResultAssembly,
                        context: HashMap::from([("error".to_string(), error_str)]),
                        suggestions: vec![
                            "Check database connectivity".to_string(),
                            "Verify chunk details are available in the database".to_string(),
                        ],
                    }
                }
            }
        }
    }

    /// Convert a `QueryProcessorError` into structured error details.
    fn from_query_processor_error(error: &QueryProcessorError) -> Self {
        match error {
            // Embedding provider errors
            QueryProcessorError::Embedding(embedding_error) => {
                Self::from_embedding_error(embedding_error)
            }

            // Validation: Empty query
            QueryProcessorError::EmptyQuery => Self {
                error_type: ErrorType::Validation,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::new(),
                suggestions: vec!["Provide a non-empty search query".to_string()],
            },

            // Validation: Query too long
            QueryProcessorError::QueryTooLong(length) => Self {
                error_type: ErrorType::Validation,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([
                    ("length".to_string(), length.to_string()),
                    ("max_length".to_string(), "1000".to_string()),
                ]),
                suggestions: vec!["Shorten your query to less than 1000 characters".to_string()],
            },

            // Validation: No meaningful content
            QueryProcessorError::NoMeaningfulContent => Self {
                error_type: ErrorType::Validation,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::new(),
                suggestions: vec![
                    "Provide a query with at least one alphanumeric character".to_string()
                ],
            },

            // Generic query processing error
            QueryProcessorError::Other(msg) => Self {
                error_type: ErrorType::Unknown,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("error".to_string(), msg.clone())]),
                suggestions: vec!["Please report this error with full details".to_string()],
            },
        }
    }

    /// Convert an `EmbeddingError` into structured error details.
    fn from_embedding_error(error: &EmbeddingError) -> Self {
        match error {
            // API errors (timeout, auth, quota, etc.)
            EmbeddingError::Api(_) => {
                let (context, suggestions) = Self::embedding_error_details(error);
                Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context,
                    suggestions,
                }
            }

            // Configuration errors (missing API key, invalid config)
            EmbeddingError::Config(config_error) => Self::from_config_error(config_error),

            // Network errors
            EmbeddingError::Network(_) => {
                let (context, suggestions) = Self::embedding_error_details(error);
                Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context,
                    suggestions,
                }
            }

            // Cache, JSON, InvalidInput, Other
            _ => {
                let (context, suggestions) = Self::embedding_error_details(error);
                Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context,
                    suggestions,
                }
            }
        }
    }

    /// Convert an `ApiError` into structured error details.
    #[allow(dead_code)]
    fn from_api_error(error: &ApiError) -> Self {
        match error {
            ApiError::RateLimit { retry_after_ms } => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([
                    (
                        "provider_error".to_string(),
                        "Rate limit exceeded".to_string(),
                    ),
                    ("timeout_ms".to_string(), retry_after_ms.to_string()),
                ]),
                suggestions: vec![
                    format!("Wait {} seconds before retrying", retry_after_ms / 1000),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            },

            ApiError::ServerError { status, message } => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([
                    (
                        "provider_error".to_string(),
                        format!("Server error ({})", status),
                    ),
                    ("message".to_string(), message.clone()),
                ]),
                suggestions: vec![
                    "The embedding provider is experiencing issues, try again later".to_string(),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            },

            ApiError::Authentication(msg) => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("provider_error".to_string(), msg.clone())]),
                suggestions: vec![
                    "Check your API credentials (OPENAI_API_KEY, GOOGLE_API_KEY, etc.)".to_string(),
                    "Verify your API key is valid and has not expired".to_string(),
                ],
            },

            ApiError::BadRequest(msg) => Self {
                error_type: ErrorType::Validation,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("provider_error".to_string(), msg.clone())]),
                suggestions: vec![
                    "The query format is invalid for the embedding provider".to_string(),
                    "Try a simpler query".to_string(),
                ],
            },

            ApiError::QuotaExceeded(msg) => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("provider_error".to_string(), msg.clone())]),
                suggestions: vec![
                    "Your API quota has been exceeded".to_string(),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            },

            ApiError::ModelUnavailable(msg) => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([
                    ("provider_error".to_string(), msg.clone()),
                    ("provider".to_string(), "unknown".to_string()),
                ]),
                suggestions: vec![
                    "The requested embedding model is not available".to_string(),
                    "Check your MAPROOM_EMBEDDING_MODEL configuration".to_string(),
                ],
            },

            ApiError::InvalidResponse(msg) => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("provider_error".to_string(), msg.clone())]),
                suggestions: vec![
                    "The embedding provider returned an invalid response".to_string(),
                    "Try again or use FTS mode: --mode fts".to_string(),
                ],
            },
        }
    }

    /// Convert a `ConfigError` into structured error details.
    fn from_config_error(error: &ConfigError) -> Self {
        match error {
            ConfigError::MissingConfig(field) => {
                // Detect provider from field name
                let (provider, suggestion) = if field.contains("OPENAI")
                    || field.contains("OpenAI")
                    || field.contains("openai")
                {
                    (
                        "openai",
                        "Set OPENAI_API_KEY environment variable".to_string(),
                    )
                } else if field.contains("GOOGLE")
                    || field.contains("Google")
                    || field.contains("google")
                {
                    (
                        "google",
                        "Set GOOGLE_API_KEY and GOOGLE_PROJECT_ID environment variables"
                            .to_string(),
                    )
                } else if field.contains("OLLAMA")
                    || field.contains("Ollama")
                    || field.contains("ollama")
                {
                    ("ollama", "Start Ollama service: ollama serve".to_string())
                } else {
                    ("unknown", format!("Set {} environment variable", field))
                };

                Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context: HashMap::from([
                        ("provider".to_string(), provider.to_string()),
                        ("message".to_string(), field.clone()),
                    ]),
                    suggestions: vec![
                        suggestion,
                        "Check your embedding provider configuration".to_string(),
                    ],
                }
            }

            ConfigError::InvalidValue { field, reason } => Self {
                error_type: ErrorType::Validation,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([(
                    "message".to_string(),
                    format!("Invalid {}: {}", field, reason),
                )]),
                suggestions: vec![format!("Check your {} configuration value", field)],
            },

            ConfigError::EnvVarNotFound(var_name) => {
                let (provider, suggestion) = if var_name.contains("OPENAI") {
                    (
                        "openai",
                        "Set OPENAI_API_KEY environment variable".to_string(),
                    )
                } else if var_name.contains("GOOGLE") {
                    (
                        "google",
                        "Set GOOGLE_API_KEY and GOOGLE_PROJECT_ID environment variables"
                            .to_string(),
                    )
                } else if var_name.contains("OLLAMA") {
                    (
                        "ollama",
                        "Start Ollama service or check OLLAMA_URL".to_string(),
                    )
                } else {
                    ("unknown", format!("Set {} environment variable", var_name))
                };

                Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context: HashMap::from([
                        ("provider".to_string(), provider.to_string()),
                        ("message".to_string(), var_name.clone()),
                    ]),
                    suggestions: vec![suggestion],
                }
            }

            ConfigError::FileError(msg) => Self {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("error".to_string(), msg.clone())]),
                suggestions: vec!["Check your configuration file path and permissions".to_string()],
            },
        }
    }

    /// Extract provider-specific error details and suggestions from embedding errors.
    ///
    /// Uses simple string matching on error messages to detect provider type and
    /// error patterns, then provides 3-4 specific actionable suggestions.
    fn embedding_error_details(error: &EmbeddingError) -> (HashMap<String, String>, Vec<String>) {
        let mut context = HashMap::new();
        let error_str = error.to_string();
        let error_lower = error_str.to_lowercase();

        // Special handling for ApiError::RateLimit to extract timeout_ms
        if let EmbeddingError::Api(ApiError::RateLimit { retry_after_ms }) = error {
            context.insert(
                "provider_error".to_string(),
                "Rate limit exceeded".to_string(),
            );
            context.insert("timeout_ms".to_string(), retry_after_ms.to_string());

            return (
                context,
                vec![
                    format!("Wait {} seconds before retrying", retry_after_ms / 1000),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            );
        }

        // Detect provider from error message patterns
        let provider = if error_lower.contains("openai") {
            Some("OpenAI")
        } else if error_lower.contains("ollama") {
            Some("Ollama")
        } else if error_lower.contains("google") {
            Some("Google")
        } else {
            None
        };

        if let Some(p) = provider {
            context.insert("provider".to_string(), p.to_string());
        }
        context.insert("provider_error".to_string(), error_str.clone());

        // Provider-specific suggestions based on error patterns
        let suggestions = if let Some(p) = provider {
            match p {
                "OpenAI" => {
                    if error_lower.contains("timeout") {
                        vec![
                            "Check your network connectivity".to_string(),
                            "Verify OpenAI API status: https://status.openai.com".to_string(),
                            "Try increasing timeout in config".to_string(),
                            "Fallback to FTS mode: --mode fts".to_string(),
                        ]
                    } else if error_lower.contains("unauthorized")
                        || error_lower.contains("invalid")
                        || error_lower.contains("authentication")
                    {
                        vec![
                            "Check OPENAI_API_KEY environment variable".to_string(),
                            "Verify API key is valid and not expired".to_string(),
                            "Check account billing status".to_string(),
                        ]
                    } else {
                        vec![
                            "Check OpenAI API credentials".to_string(),
                            "Try FTS mode: --mode fts".to_string(),
                        ]
                    }
                }
                "Ollama" => {
                    if error_lower.contains("connection") || error_lower.contains("refused") {
                        vec![
                            "Start Ollama service: ollama serve".to_string(),
                            "Verify Ollama is running: curl http://localhost:11434".to_string(),
                            "Check OLLAMA_HOST environment variable".to_string(),
                        ]
                    } else if error_lower.contains("model") {
                        vec![
                            "Pull required model: ollama pull mxbai-embed-large".to_string(),
                            "List available models: ollama list".to_string(),
                            "Check model name in config".to_string(),
                        ]
                    } else {
                        vec![
                            "Check Ollama service status".to_string(),
                            "Try FTS mode: --mode fts".to_string(),
                        ]
                    }
                }
                "Google" => {
                    vec![
                        "Check GOOGLE_API_KEY environment variable".to_string(),
                        "Verify Google Cloud credentials".to_string(),
                        "Check API quota limits".to_string(),
                    ]
                }
                _ => {
                    // Unknown provider - use generic suggestions
                    vec![
                        "Check embedding provider configuration".to_string(),
                        "Try FTS mode: --mode fts".to_string(),
                    ]
                }
            }
        } else {
            // Generic suggestions when provider cannot be detected
            if error_lower.contains("timeout") {
                vec![
                    "Check your network connectivity".to_string(),
                    "Verify embedding provider is reachable".to_string(),
                    "Try FTS mode: --mode fts".to_string(),
                ]
            } else if error_lower.contains("unauthorized")
                || error_lower.contains("authentication")
                || error_lower.contains("credentials")
            {
                vec![
                    "Check embedding provider API credentials".to_string(),
                    "Verify API key environment variables are set".to_string(),
                    "Try FTS mode: --mode fts".to_string(),
                ]
            } else {
                vec![
                    "Check embedding provider configuration".to_string(),
                    "Try FTS mode: --mode fts".to_string(),
                ]
            }
        };

        (context, suggestions)
    }

    /// Extract database-specific error details and suggestions.
    ///
    /// Uses simple string matching to detect common database error patterns
    /// and provides 3-4 specific actionable suggestions.
    fn database_error_details(error: &str) -> (HashMap<String, String>, Vec<String>) {
        let mut context = HashMap::new();
        context.insert("message".to_string(), error.to_string());

        let error_lower = error.to_lowercase();
        let suggestions = if error_lower.contains("not found")
            || error_lower.contains("does not exist")
            || error_lower.contains("not indexed")
        {
            vec![
                "Check repository name and path".to_string(),
                "Run 'maproom status' to list indexed repositories".to_string(),
                "Index repository: 'maproom scan <path>'".to_string(),
            ]
        } else if error_lower.contains("connection") || error_lower.contains("timeout") {
            vec![
                "Check database file exists: ~/.maproom/maproom.db".to_string(),
                "Verify database is not locked by another process".to_string(),
                "Restart daemon: maproom serve".to_string(),
            ]
        } else if error_lower.contains("corrupt") || error_lower.contains("malformed") {
            vec![
                "Backup database: cp ~/.maproom/maproom.db ~/.maproom/maproom.db.backup"
                    .to_string(),
                "Rebuild index: maproom scan --rebuild".to_string(),
                "Check disk space".to_string(),
            ]
        } else {
            vec![
                "Check database connectivity".to_string(),
                "Verify repository is indexed".to_string(),
            ]
        };

        (context, suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::error::{ApiError, CacheError, ConfigError, EmbeddingError};
    use crate::search::executors::ExecutorError;
    use crate::search::fts::FTSError;
    use crate::search::query_processor::QueryProcessorError;

    #[test]
    fn test_empty_query_error_conversion() {
        let error = PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery);
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Validation);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(!details.suggestions.is_empty());
        assert!(details.suggestions.iter().any(|s| s.contains("non-empty")));
    }

    #[test]
    fn test_query_too_long_error_conversion() {
        let error = PipelineError::QueryProcessing(QueryProcessorError::QueryTooLong(1500));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Validation);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert_eq!(details.context.get("length"), Some(&"1500".to_string()));
        assert_eq!(details.context.get("max_length"), Some(&"1000".to_string()));
        assert!(details.suggestions.iter().any(|s| s.contains("1000")));
    }

    #[test]
    fn test_no_meaningful_content_error_conversion() {
        let error = PipelineError::QueryProcessing(QueryProcessorError::NoMeaningfulContent);
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Validation);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("alphanumeric")));
    }

    #[test]
    fn test_embedding_provider_authentication_error() {
        let api_error = ApiError::Authentication("Invalid API key".to_string());
        let embedding_error = EmbeddingError::Api(api_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details.suggestions.len() >= 1);
        assert!(details.suggestions.iter().any(|s| s.contains("API")));
    }

    #[test]
    fn test_embedding_provider_rate_limit_error() {
        let api_error = ApiError::RateLimit {
            retry_after_ms: 5000,
        };
        let embedding_error = EmbeddingError::Api(api_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert_eq!(details.context.get("timeout_ms"), Some(&"5000".to_string()));
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_embedding_provider_server_error() {
        let api_error = ApiError::ServerError {
            status: 503,
            message: "Service unavailable".to_string(),
        };
        let embedding_error = EmbeddingError::Api(api_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details.suggestions.len() >= 2);
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_embedding_provider_config_error_openai() {
        let config_error = ConfigError::MissingConfig("OPENAI_API_KEY".to_string());
        let embedding_error = EmbeddingError::Config(config_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert_eq!(details.context.get("provider"), Some(&"openai".to_string()));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("OPENAI_API_KEY")));
    }

    #[test]
    fn test_embedding_provider_config_error_google() {
        let config_error = ConfigError::MissingConfig("GOOGLE_API_KEY".to_string());
        let embedding_error = EmbeddingError::Config(config_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert_eq!(details.context.get("provider"), Some(&"google".to_string()));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("GOOGLE_API_KEY")));
    }

    #[test]
    fn test_embedding_provider_config_error_ollama() {
        let config_error = ConfigError::MissingConfig("OLLAMA service".to_string());
        let embedding_error = EmbeddingError::Config(config_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert_eq!(details.context.get("provider"), Some(&"ollama".to_string()));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("Ollama") || s.contains("ollama")));
    }

    #[test]
    fn test_embedding_provider_network_error() {
        // Create a mock network error using cache error as proxy
        let cache_error = CacheError::WriteFailed("network timeout".to_string());
        let embedding_error = EmbeddingError::Cache(cache_error);
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(!details.suggestions.is_empty());
    }

    #[test]
    fn test_database_error_not_indexed() {
        let error = PipelineError::Database("Repository not indexed".to_string());
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::NotFound);
        assert_eq!(details.stage, PipelineStage::SearchExecution);
        assert!(details.suggestions.iter().any(|s| s.contains("status")));
    }

    #[test]
    fn test_database_error_timeout() {
        let error = PipelineError::Database("Database connection timeout".to_string());
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert_eq!(details.stage, PipelineStage::SearchExecution);
        assert!(details.suggestions.iter().any(|s| s.contains("daemon")));
    }

    #[test]
    fn test_database_error_corrupted() {
        let error = PipelineError::Database("Database file is corrupted".to_string());
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert_eq!(details.stage, PipelineStage::SearchExecution);
        assert!(details.suggestions.iter().any(|s| s.contains("rebuild")));
        assert!(details.suggestions.iter().any(|s| s.contains("disk space")));
    }

    #[test]
    fn test_search_execution_timeout() {
        let fts_error = FTSError::Database("Search timeout exceeded".to_string());
        let executor_error = ExecutorError::FTS(fts_error);
        let error = PipelineError::SearchExecution(executor_error);
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Timeout);
        assert_eq!(details.stage, PipelineStage::SearchExecution);
        assert!(details.suggestions.iter().any(|s| s.contains("narrow")));
    }

    #[test]
    fn test_result_assembly_not_found() {
        let error = PipelineError::Assembly("Chunk 12345 not found in database".to_string());
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::NotFound);
        assert_eq!(details.stage, PipelineStage::ResultAssembly);
        assert!(details.suggestions.iter().any(|s| s.contains("scan")));
    }

    #[test]
    fn test_unknown_error_fallback() {
        let error = PipelineError::QueryProcessing(QueryProcessorError::Other(
            "Unexpected error".to_string(),
        ));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Unknown);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details.suggestions.iter().any(|s| s.contains("report")));
    }

    #[test]
    fn test_all_error_types_have_suggestions() {
        // Test that every error type produces at least one suggestion
        let test_cases = vec![
            PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery),
            PipelineError::QueryProcessing(QueryProcessorError::QueryTooLong(1500)),
            PipelineError::QueryProcessing(QueryProcessorError::Embedding(EmbeddingError::Api(
                ApiError::Authentication("test".to_string()),
            ))),
            PipelineError::Database("test error".to_string()),
            PipelineError::SearchExecution(ExecutorError::FTS(FTSError::Database(
                "timeout".to_string(),
            ))),
            PipelineError::Assembly("not found".to_string()),
        ];

        for error in test_cases {
            let details = SearchErrorDetails::from_pipeline_error(&error);
            assert!(
                !details.suggestions.is_empty(),
                "Error {:?} has no suggestions",
                error
            );
            assert!(
                details.suggestions.len() >= 1,
                "Error {:?} has fewer than 1 suggestion",
                error
            );
        }
    }

    #[test]
    fn test_context_whitelist_enforced() {
        // Test that only whitelisted context keys are present
        let whitelisted_keys = vec![
            "provider_error",
            "provider",
            "error",
            "message",
            "length",
            "max_length",
            "repo_name",
            "worktree_id",
            "timeout_ms",
        ];

        let error = PipelineError::QueryProcessing(QueryProcessorError::QueryTooLong(1500));
        let details = SearchErrorDetails::from_pipeline_error(&error);

        for key in details.context.keys() {
            assert!(
                whitelisted_keys.contains(&key.as_str()),
                "Context key '{}' is not whitelisted",
                key
            );
        }
    }

    #[test]
    fn test_error_type_serialization() {
        // Test that ErrorType serializes to snake_case
        let error_type = ErrorType::EmbeddingProvider;
        let json = serde_json::to_string(&error_type).unwrap();
        assert_eq!(json, r#""embedding_provider""#);

        let error_type = ErrorType::NotFound;
        let json = serde_json::to_string(&error_type).unwrap();
        assert_eq!(json, r#""not_found""#);
    }

    #[test]
    fn test_pipeline_stage_serialization() {
        // Test that PipelineStage serializes to snake_case
        let stage = PipelineStage::QueryProcessing;
        let json = serde_json::to_string(&stage).unwrap();
        assert_eq!(json, r#""query_processing""#);

        let stage = PipelineStage::ResultAssembly;
        let json = serde_json::to_string(&stage).unwrap();
        assert_eq!(json, r#""result_assembly""#);
    }

    // SRCHTRN-3001: Enhanced Error Suggestions Tests

    #[test]
    fn test_openai_timeout_suggestions() {
        // Use Other error type which we can construct easily
        let embedding_error = EmbeddingError::Other("OpenAI request timeout".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.context.get("provider").unwrap(), "OpenAI");
        assert!(details.suggestions.len() >= 3);
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("status.openai.com")));
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_ollama_connection_suggestions() {
        // Use Other error type with connection refused message
        let embedding_error = EmbeddingError::Other("Ollama connection refused".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.context.get("provider").unwrap(), "Ollama");
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("ollama serve")));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("localhost:11434")));
    }

    #[test]
    fn test_database_not_found_suggestions() {
        let error = PipelineError::Database("repository not found".to_string());

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("maproom status")));
        assert!(details.suggestions.iter().any(|s| s.contains("scan")));
    }

    #[test]
    fn test_openai_unauthorized_suggestions() {
        // Use Other error with OpenAI authentication failure
        let embedding_error = EmbeddingError::Other("Invalid OpenAI API key".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.context.get("provider").unwrap(), "OpenAI");
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("OPENAI_API_KEY")));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("billing") || s.contains("expired")));
    }

    #[test]
    fn test_ollama_model_suggestions() {
        // Use Other error with Ollama model not found
        let embedding_error = EmbeddingError::Other("Ollama model not found".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.context.get("provider").unwrap(), "Ollama");
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("ollama pull")));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("ollama list")));
    }

    #[test]
    fn test_database_connection_timeout_suggestions() {
        let error = PipelineError::Database("Database connection timeout".to_string());

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert!(details.suggestions.iter().any(|s| s.contains("maproom.db")));
        assert!(details.suggestions.iter().any(|s| s.contains("locked")));
        assert!(details.suggestions.iter().any(|s| s.contains("serve")));
    }

    #[test]
    fn test_database_corrupt_suggestions() {
        let error = PipelineError::Database("Database file is corrupted".to_string());

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert!(details.suggestions.iter().any(|s| s.contains("Backup")));
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("rebuild") || s.contains("Rebuild")));
        assert!(details.suggestions.iter().any(|s| s.contains("disk space")));
    }

    #[test]
    fn test_google_provider_suggestions() {
        let embedding_error = EmbeddingError::Other("Google API error: quota exceeded".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.context.get("provider").unwrap(), "Google");
        assert!(details
            .suggestions
            .iter()
            .any(|s| s.contains("GOOGLE_API_KEY")));
        assert!(details.suggestions.iter().any(|s| s.contains("quota")));
    }

    #[test]
    fn test_generic_timeout_without_provider() {
        let embedding_error = EmbeddingError::Other("request timeout".to_string());
        let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert!(details.suggestions.iter().any(|s| s.contains("network")));
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }
}
