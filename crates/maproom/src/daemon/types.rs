use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String, // Must be "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>, // ID can be number, string, or null
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub repo: String,
    pub worktree: Option<String>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub mode: Option<String>, // "fts", "vector", or "hybrid"
    /// Deduplicate results across worktrees (default: true)
    #[serde(default = "default_deduplicate")]
    pub deduplicate: Option<bool>,
}

fn default_deduplicate() -> Option<bool> {
    Some(true)
}

/// Default budget for context assembly (6000 tokens).
fn default_budget() -> usize {
    6000
}

/// Default max depth for relationship traversal.
fn default_max_depth() -> i32 {
    2
}

/// Parameters for the context JSON-RPC method.
#[derive(Debug, Deserialize)]
pub struct ContextParams {
    /// Chunk ID to retrieve context for (String for JSON compatibility)
    pub chunk_id: String,
    /// Maximum tokens for the context bundle
    #[serde(default = "default_budget")]
    pub budget_tokens: usize,
    /// Expansion options for related chunks
    #[serde(default)]
    pub expand: ExpandConfig,
}

/// Configuration for expanding context beyond the primary chunk.
/// Mirrors `crates/maproom/src/context/types.rs::ExpandOptions`.
#[derive(Debug, Deserialize)]
pub struct ExpandConfig {
    /// Include caller chunks (functions that call the primary chunk)
    #[serde(default)]
    pub callers: bool,
    /// Include callee chunks (functions called by the primary chunk)
    #[serde(default)]
    pub callees: bool,
    /// Include test chunks
    #[serde(default)]
    pub tests: bool,
    /// Include documentation chunks
    #[serde(default)]
    pub docs: bool,
    /// Include configuration files
    #[serde(default)]
    pub config: bool,
    /// Maximum depth for relationship traversal
    #[serde(default = "default_max_depth")]
    pub max_depth: i32,
    /// React-specific: Include route definitions
    #[serde(default)]
    pub routes: bool,
    /// React-specific: Include hooks used by components
    #[serde(default)]
    pub hooks: bool,
    /// React-specific: Include JSX parent components
    #[serde(default)]
    pub jsx_parents: bool,
    /// React-specific: Include JSX child components
    #[serde(default)]
    pub jsx_children: bool,
}

impl Default for ExpandConfig {
    fn default() -> Self {
        Self {
            callers: false,
            callees: false,
            tests: false,
            docs: false,
            config: false,
            max_depth: 2, // Match serde default
            routes: false,
            hooks: false,
            jsx_parents: false,
            jsx_children: false,
        }
    }
}

/// Parameters for the status JSON-RPC method.
#[derive(Debug, Deserialize, Default)]
pub struct StatusParams {
    /// Optional repo name filter
    pub repo: Option<String>,
}

/// Worktree statistics in status response.
#[derive(Debug, Serialize)]
pub struct WorktreeStatus {
    pub name: String,
    pub path: String,
    pub file_count: i64,
    pub chunk_count: i64,
}

/// Repository statistics in status response.
#[derive(Debug, Serialize)]
pub struct RepoStatus {
    pub name: String,
    pub worktrees: Vec<WorktreeStatus>,
}

/// Response for the status JSON-RPC method.
/// Sync with: packages/daemon-client/src/client.ts StatusResult
#[derive(Debug, Serialize)]
pub struct StatusResult {
    pub repos: Vec<RepoStatus>,
    pub total_repos: usize,
    pub total_files: i64,
    pub total_chunks: i64,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(
        id: serde_json::Value,
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_params_deserialization_minimal() {
        // Minimal JSON with only required chunk_id field
        let json = r#"{"chunk_id": "12345"}"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.chunk_id, "12345");
        assert_eq!(params.budget_tokens, 6000); // Default
        assert!(!params.expand.callers); // Default false
        assert!(!params.expand.callees);
        assert!(!params.expand.tests);
        assert!(!params.expand.docs);
        assert!(!params.expand.config);
        assert_eq!(params.expand.max_depth, 2); // Default
        assert!(!params.expand.routes);
        assert!(!params.expand.hooks);
        assert!(!params.expand.jsx_parents);
        assert!(!params.expand.jsx_children);
    }

    #[test]
    fn test_context_params_deserialization_full() {
        // Full JSON with all fields
        let json = r#"{
            "chunk_id": "99999",
            "budget_tokens": 8000,
            "expand": {
                "callers": true,
                "callees": true,
                "tests": true,
                "docs": true,
                "config": true,
                "max_depth": 5,
                "routes": true,
                "hooks": true,
                "jsx_parents": true,
                "jsx_children": true
            }
        }"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.chunk_id, "99999");
        assert_eq!(params.budget_tokens, 8000);
        assert!(params.expand.callers);
        assert!(params.expand.callees);
        assert!(params.expand.tests);
        assert!(params.expand.docs);
        assert!(params.expand.config);
        assert_eq!(params.expand.max_depth, 5);
        assert!(params.expand.routes);
        assert!(params.expand.hooks);
        assert!(params.expand.jsx_parents);
        assert!(params.expand.jsx_children);
    }

    #[test]
    fn test_expand_config_defaults() {
        // Test the Default implementation
        let config = ExpandConfig::default();

        assert!(!config.callers);
        assert!(!config.callees);
        assert!(!config.tests);
        assert!(!config.docs);
        assert!(!config.config);
        assert_eq!(config.max_depth, 2); // Serde default
        assert!(!config.routes);
        assert!(!config.hooks);
        assert!(!config.jsx_parents);
        assert!(!config.jsx_children);
    }

    #[test]
    fn test_context_params_partial_expand() {
        // Partial expand options - only some fields set
        let json = r#"{
            "chunk_id": "42",
            "expand": {
                "callers": true,
                "tests": true
            }
        }"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.chunk_id, "42");
        assert_eq!(params.budget_tokens, 6000); // Default
        assert!(params.expand.callers);
        assert!(!params.expand.callees); // Default
        assert!(params.expand.tests);
        assert!(!params.expand.docs); // Default
        assert_eq!(params.expand.max_depth, 2); // Default
    }
}
