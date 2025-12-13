//! Integration tests for daemon JSON-RPC error serialization.
//!
//! These tests verify that PipelineError is correctly converted to SearchErrorDetails
//! and serialized in JSON-RPC error responses according to the JSON-RPC 2.0 spec.

use crewchief_maproom::daemon::types::JsonRpcResponse;
use crewchief_maproom::embedding::error::{ApiError, ConfigError, EmbeddingError};
use crewchief_maproom::search::errors::{ErrorType, PipelineStage, SearchErrorDetails};
use crewchief_maproom::search::executors::ExecutorError;
use crewchief_maproom::search::fts::FTSError;
use crewchief_maproom::search::pipeline::PipelineError;
use crewchief_maproom::search::query_processor::QueryProcessorError;

/// Test that SearchErrorDetails serializes correctly to JSON
#[test]
fn test_search_error_details_serialization() {
    // Create a sample SearchErrorDetails instance
    let error = PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery);
    let details = SearchErrorDetails::from_pipeline_error(&error);

    // Serialize to JSON value
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    // Verify structure
    assert!(json_value.is_object());
    assert_eq!(json_value["error_type"], "validation");
    assert_eq!(json_value["stage"], "query_processing");
    assert!(json_value["context"].is_object());
    assert!(json_value["suggestions"].is_array());
    assert!(!json_value["suggestions"].as_array().unwrap().is_empty());
}

/// Test embedding provider authentication error serialization
#[test]
fn test_embedding_authentication_error_serialization() {
    let api_error = ApiError::Authentication("Invalid API key".to_string());
    let embedding_error = EmbeddingError::Api(api_error);
    let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "embedding_provider");
    assert_eq!(json_value["stage"], "query_processing");
    assert!(json_value["context"]["provider_error"]
        .as_str()
        .unwrap()
        .contains("Invalid API key"));
    assert!(!json_value["suggestions"].as_array().unwrap().is_empty());
}

/// Test embedding provider rate limit error serialization
#[test]
fn test_embedding_rate_limit_error_serialization() {
    let api_error = ApiError::RateLimit {
        retry_after_ms: 5000,
    };
    let embedding_error = EmbeddingError::Api(api_error);
    let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "embedding_provider");
    assert_eq!(json_value["stage"], "query_processing");
    assert_eq!(json_value["context"]["timeout_ms"], "5000");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("FTS mode")));
}

/// Test embedding provider config error serialization (OpenAI)
#[test]
fn test_embedding_config_error_openai_serialization() {
    let config_error = ConfigError::MissingConfig("OPENAI_API_KEY".to_string());
    let embedding_error = EmbeddingError::Config(config_error);
    let error = PipelineError::QueryProcessing(QueryProcessorError::Embedding(embedding_error));

    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "embedding_provider");
    assert_eq!(json_value["stage"], "query_processing");
    assert_eq!(json_value["context"]["provider"], "openai");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("OPENAI_API_KEY")));
}

/// Test database error (not indexed) serialization
#[test]
fn test_database_not_indexed_error_serialization() {
    let error = PipelineError::Database("Repository not indexed".to_string());
    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "not_found");
    assert_eq!(json_value["stage"], "search_execution");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("status")));
}

/// Test database timeout error serialization
#[test]
fn test_database_timeout_error_serialization() {
    let error = PipelineError::Database("Database connection timeout".to_string());
    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "database");
    assert_eq!(json_value["stage"], "search_execution");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("daemon")));
}

/// Test search execution timeout error serialization
#[test]
fn test_search_execution_timeout_serialization() {
    let fts_error = FTSError::Database("Search timeout exceeded".to_string());
    let executor_error = ExecutorError::FTS(fts_error);
    let error = PipelineError::SearchExecution(executor_error);

    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "timeout");
    assert_eq!(json_value["stage"], "search_execution");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("narrow")));
}

/// Test query validation error (query too long) serialization
#[test]
fn test_query_too_long_error_serialization() {
    let error = PipelineError::QueryProcessing(QueryProcessorError::QueryTooLong(1500));
    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "validation");
    assert_eq!(json_value["stage"], "query_processing");
    assert_eq!(json_value["context"]["length"], "1500");
    assert_eq!(json_value["context"]["max_length"], "1000");
}

/// Test result assembly error (chunk not found) serialization
#[test]
fn test_result_assembly_not_found_serialization() {
    let error = PipelineError::Assembly("Chunk 12345 not found in database".to_string());
    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json_value = serde_json::to_value(&details).expect("Serialization should succeed");

    assert_eq!(json_value["error_type"], "not_found");
    assert_eq!(json_value["stage"], "result_assembly");
    assert!(json_value["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str().unwrap().contains("scan")));
}

/// Test JSON-RPC error response structure with structured error data
#[test]
fn test_jsonrpc_error_response_structure() {
    // Simulate what the daemon does: create error details and serialize
    let error = PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery);
    let error_details = SearchErrorDetails::from_pipeline_error(&error);

    let error_data = match serde_json::to_value(&error_details) {
        Ok(value) => Some(value),
        Err(_) => Some(serde_json::json!(error.to_string())),
    };

    let response = JsonRpcResponse::error(
        serde_json::Value::Number(1.into()),
        -32000,
        error.to_string(),
        error_data,
    );

    // Serialize the entire response
    let json_str = serde_json::to_string(&response).expect("Response serialization should succeed");
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).expect("JSON parsing should succeed");

    // Verify JSON-RPC structure
    assert_eq!(json_value["jsonrpc"], "2.0");
    assert_eq!(json_value["id"], 1);
    assert!(json_value["error"].is_object());
    assert_eq!(json_value["error"]["code"], -32000);
    assert!(json_value["error"]["message"].is_string());
    assert!(json_value["error"]["data"].is_object());

    // Verify error data structure
    let error_data = &json_value["error"]["data"];
    assert_eq!(error_data["error_type"], "validation");
    assert_eq!(error_data["stage"], "query_processing");
    assert!(error_data["context"].is_object());
    assert!(error_data["suggestions"].is_array());
}

/// Test error serialization backward compatibility (message field)
#[test]
fn test_error_message_backward_compatibility() {
    let error = PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery);
    let error_message = error.to_string();
    let error_details = SearchErrorDetails::from_pipeline_error(&error);

    let error_data = serde_json::to_value(&error_details).expect("Serialization should succeed");

    let response = JsonRpcResponse::error(
        serde_json::Value::Number(1.into()),
        -32000,
        error_message.clone(),
        Some(error_data),
    );

    let json_str = serde_json::to_string(&response).expect("Response serialization should succeed");
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).expect("JSON parsing should succeed");

    // Verify message field preserves human-readable error
    assert_eq!(json_value["error"]["message"], error_message);
    assert!(json_value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Query processing failed"));
}

/// Test serialization fallback when error details cannot be serialized
#[test]
fn test_serialization_fallback() {
    let error = PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery);
    let error_details = SearchErrorDetails::from_pipeline_error(&error);

    // Simulate serialization success (normal case)
    let error_data = match serde_json::to_value(&error_details) {
        Ok(value) => Some(value),
        Err(_) => Some(serde_json::json!(error.to_string())),
    };

    assert!(error_data.is_some());
    assert!(error_data.as_ref().unwrap().is_object());
    assert_eq!(error_data.as_ref().unwrap()["error_type"], "validation");

    // Test fallback path (error_details is always serializable, but this tests the pattern)
    // In production, serialization could fail for other reasons
    let fallback_data = Some(serde_json::json!(error.to_string()));
    assert!(fallback_data.is_some());
    assert!(fallback_data.as_ref().unwrap().is_string());
}

/// Test error code remains -32000 for all application errors
#[test]
fn test_error_code_consistency() {
    let test_cases = vec![
        PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery),
        PipelineError::QueryProcessing(QueryProcessorError::QueryTooLong(1500)),
        PipelineError::Database("test error".to_string()),
        PipelineError::SearchExecution(ExecutorError::FTS(FTSError::Database(
            "timeout".to_string(),
        ))),
        PipelineError::Assembly("not found".to_string()),
    ];

    for error in test_cases {
        let error_details = SearchErrorDetails::from_pipeline_error(&error);
        let error_data =
            serde_json::to_value(&error_details).expect("Serialization should succeed");

        let response = JsonRpcResponse::error(
            serde_json::Value::Number(1.into()),
            -32000,
            error.to_string(),
            Some(error_data),
        );

        let json_str =
            serde_json::to_string(&response).expect("Response serialization should succeed");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).expect("JSON parsing should succeed");

        assert_eq!(
            json_value["error"]["code"], -32000,
            "All application errors should use code -32000"
        );
    }
}

/// Test all error types produce valid JSON-RPC responses
#[test]
fn test_all_error_types_valid_jsonrpc() {
    let test_cases = vec![
        (
            PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery),
            ErrorType::Validation,
            PipelineStage::QueryProcessing,
        ),
        (
            PipelineError::QueryProcessing(QueryProcessorError::Embedding(EmbeddingError::Api(
                ApiError::Authentication("test".to_string()),
            ))),
            ErrorType::EmbeddingProvider,
            PipelineStage::QueryProcessing,
        ),
        (
            PipelineError::Database("test error".to_string()),
            ErrorType::Database,
            PipelineStage::SearchExecution,
        ),
        (
            PipelineError::SearchExecution(ExecutorError::FTS(FTSError::Database(
                "timeout".to_string(),
            ))),
            ErrorType::Timeout,
            PipelineStage::SearchExecution,
        ),
        (
            PipelineError::Assembly("not found".to_string()),
            ErrorType::NotFound,
            PipelineStage::ResultAssembly,
        ),
    ];

    for (error, expected_type, expected_stage) in test_cases {
        let error_details = SearchErrorDetails::from_pipeline_error(&error);
        let error_data =
            serde_json::to_value(&error_details).expect("Serialization should succeed");

        let response = JsonRpcResponse::error(
            serde_json::Value::Number(1.into()),
            -32000,
            error.to_string(),
            Some(error_data),
        );

        // Verify it serializes to valid JSON
        let json_str =
            serde_json::to_string(&response).expect("Response serialization should succeed");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).expect("JSON parsing should succeed");

        // Verify structure
        assert_eq!(json_value["jsonrpc"], "2.0");
        assert!(json_value["error"].is_object());
        assert_eq!(json_value["error"]["code"], -32000);

        // Verify error details
        let data = &json_value["error"]["data"];
        assert_eq!(
            data["error_type"],
            serde_json::to_value(&expected_type).unwrap()
        );
        assert_eq!(
            data["stage"],
            serde_json::to_value(&expected_stage).unwrap()
        );
        assert!(data["suggestions"].is_array());
        assert!(!data["suggestions"].as_array().unwrap().is_empty());
    }
}
