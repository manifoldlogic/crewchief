use serde::{Deserialize, Serialize};

/// R19 / R-RPC-1: deserializer for the `id` field that distinguishes ABSENT
/// (notification) from explicit `null` (a request whose id is null). This fn
/// only runs when the key is present, so it always wraps in `Some`; the
/// `#[serde(default)]` outer `None` can only mean "field absent".
fn deserialize_present_id<'de, D>(d: D) -> Result<Option<Option<serde_json::Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<serde_json::Value>::deserialize(d)?))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// R19 / R-RPC-2 (OD-10): `Option` so a missing version is an Invalid
    /// Request (-32600), not a parse error; validated == "2.0" in dispatch.
    #[serde(default)]
    pub jsonrpc: Option<String>,
    pub method: String,
    pub params: Option<serde_json::Value>,
    /// `None` = field absent (JSON-RPC NOTIFICATION — the server MUST NOT
    /// reply); `Some(None)` = explicit `"id": null` (a request; answered with
    /// `"id":null`); `Some(Some(v))` = normal id. Serde's nested-Option
    /// serialization does the right thing on the way out (inner `None` →
    /// `null`), and the skip attr keeps absent ids absent.
    #[serde(
        default,
        deserialize_with = "deserialize_present_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<Option<serde_json::Value>>,
}

/// D-8a: repo scope for a search request.
///
/// Exactly one of `repo` (single), `repos` (list), or `all_repos` (flag) MUST
/// be present; the server enforces this with a structured validation error
/// (JSON-RPC -32602) rather than a panic.  Old clients that always send `repo`
/// remain byte-compatible (D-8e additive change).
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: String,
    /// Single-repo scope (legacy / default).  Relaxed from `String` to
    /// `Option<String>` so new clients can omit it and use `repos`/`all_repos`
    /// instead.  Old clients always send this field, so backward compatibility
    /// is preserved (D-8e additive change).
    #[serde(default)]
    pub repo: Option<String>,
    /// Multi-repo scope: search these repos in one query (D-8a).
    #[serde(default)]
    pub repos: Option<Vec<String>>,
    /// All-repos scope: search every repo in the index (D-8a, D-8d).
    #[serde(default)]
    pub all_repos: Option<bool>,
    pub worktree: Option<String>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub mode: Option<String>, // "fts", "vector", or "hybrid"
    /// Deduplicate results across worktrees (default: true)
    #[serde(default = "default_deduplicate")]
    pub deduplicate: Option<bool>,
    /// Filter by chunk kind (e.g., ["func", "class"])
    #[serde(default)]
    pub kind: Option<Vec<String>>,
    /// Filter by file language (e.g., ["py", "ts"])
    #[serde(default)]
    pub lang: Option<Vec<String>>,
    /// Include confidence signals in results (default: false)
    #[serde(default)]
    pub include_confidence: Option<bool>,
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

/// F81: relationship expansion defaults ON (mirrors ExpandOptions::default).
fn default_true() -> bool {
    true
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
    /// Include caller chunks (functions that call the primary chunk).
    /// F81: defaults ON — omit the field to get relationship expansion.
    #[serde(default = "default_true")]
    pub callers: bool,
    /// Include callee chunks (functions called by the primary chunk).
    #[serde(default = "default_true")]
    pub callees: bool,
    /// Include test chunks.
    #[serde(default = "default_true")]
    pub tests: bool,
    /// Include documentation chunks
    #[serde(default)]
    pub docs: bool,
    /// Include import/export relationships (F82). Defaults ON.
    #[serde(default = "default_true")]
    pub imports: bool,
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
        // Must match the serde field defaults — this Default runs when the
        // whole `expand` object is ABSENT, serde field defaults when it is
        // present but partial. Divergence here would make `{}` and absent
        // behave differently (F81).
        Self {
            callers: true,
            callees: true,
            tests: true,
            docs: false,
            imports: true,
            config: false,
            max_depth: 2, // Match serde default
            routes: false,
            hooks: false,
            jsx_parents: false,
            jsx_children: false,
        }
    }
}

/// Parameters for the cache.warm JSON-RPC method (F69).
#[derive(Debug, Deserialize)]
pub struct CacheWarmParams {
    /// Queries to execute-and-cache (each runs through the SAME cached
    /// search path as a normal request).
    pub queries: Vec<String>,
    pub repo: String,
    #[serde(default)]
    pub worktree: Option<String>,
    /// Search mode for the warmed queries (defaults to the daemon default).
    #[serde(default)]
    pub mode: Option<String>,
    /// Result limit per query (defaults to the search default).
    #[serde(default)]
    pub k: Option<usize>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                                                // F81: relationship expansion defaults ON when absent
        assert!(params.expand.callers);
        assert!(params.expand.callees);
        assert!(params.expand.tests);
        assert!(params.expand.imports);
        // docs/config/React options remain opt-in
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

        // F81: Default MUST match the serde field defaults — `expand` absent
        // and `expand: {}` must behave identically (both expansion-on).
        assert!(config.callers);
        assert!(config.callees);
        assert!(config.tests);
        assert!(config.imports);
        assert!(!config.docs);
        assert!(!config.config);
        assert_eq!(config.max_depth, 2); // Serde default
        assert!(!config.routes);
        assert!(!config.hooks);
        assert!(!config.jsx_parents);
        assert!(!config.jsx_children);

        // The absent-vs-empty equivalence, asserted directly:
        let empty: ExpandConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.callers, config.callers);
        assert_eq!(empty.callees, config.callees);
        assert_eq!(empty.tests, config.tests);
        assert_eq!(empty.imports, config.imports);
    }

    #[test]
    fn test_context_params_partial_expand() {
        // Partial expand options - only some fields set
        // F81: explicit FALSE now carries the signal (defaults are on)
        let json = r#"{
            "chunk_id": "42",
            "expand": {
                "callers": false,
                "tests": false
            }
        }"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.chunk_id, "42");
        assert_eq!(params.budget_tokens, 6000); // Default
        assert!(!params.expand.callers); // explicit opt-out honored
        assert!(params.expand.callees); // untouched fields default ON
        assert!(!params.expand.tests);
        assert!(params.expand.imports);
        assert!(!params.expand.docs); // opt-in family unchanged
        assert_eq!(params.expand.max_depth, 2); // Default
    }

    #[test]
    fn test_search_params_with_confidence_true() {
        let json = r#"{"query": "test", "repo": "myrepo", "include_confidence": true}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.include_confidence, Some(true));
        // Old-shape compat: repo is still parsed
        assert_eq!(params.repo, Some("myrepo".to_string()));
    }

    #[test]
    fn test_search_params_with_confidence_false() {
        let json = r#"{"query": "test", "repo": "myrepo", "include_confidence": false}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.include_confidence, Some(false));
    }

    #[test]
    fn test_search_params_without_confidence_field() {
        // Backward compatibility: field omitted, defaults to None
        let json = r#"{"query": "test", "repo": "myrepo"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.include_confidence, None);
    }

    // D-8e: old-shape backward compatibility test
    #[test]
    fn test_search_params_old_shape_compat() {
        // Clients that always send `repo` should be byte-compatible
        let json = r#"{"query": "embedding pipeline", "repo": "crewchief", "limit": 5}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.query, "embedding pipeline");
        assert_eq!(params.repo, Some("crewchief".to_string()));
        assert!(params.repos.is_none());
        assert!(params.all_repos.is_none());
        assert_eq!(params.limit, Some(5));
    }

    // D-8a: multi-repo new shapes
    #[test]
    fn test_search_params_repos_list() {
        let json = r#"{"query": "auth", "repos": ["crewchief", "maproom"]}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert!(params.repo.is_none());
        assert_eq!(
            params.repos,
            Some(vec!["crewchief".to_string(), "maproom".to_string()])
        );
        assert!(params.all_repos.is_none());
    }

    #[test]
    fn test_search_params_all_repos() {
        let json = r#"{"query": "embedding pipeline", "all_repos": true}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert!(params.repo.is_none());
        assert!(params.repos.is_none());
        assert_eq!(params.all_repos, Some(true));
    }
}
