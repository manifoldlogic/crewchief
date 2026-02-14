//! React hook detection.
//!
//! This module provides functionality to detect React hooks:
//! - Built-in hooks (useState, useEffect, useContext, etc.)
//! - Custom hooks (use* naming convention)
//! - Hook dependencies and relationships

use crate::db::traits::StoreChunks;
use crate::db::traits::StoreSearch;
use crate::db::SqliteStore;
use anyhow::Result;
use regex::Regex;

/// Built-in React hooks.
pub const BUILT_IN_HOOKS: &[&str] = &[
    "useState",
    "useEffect",
    "useContext",
    "useReducer",
    "useCallback",
    "useMemo",
    "useRef",
    "useImperativeHandle",
    "useLayoutEffect",
    "useDebugValue",
    "useDeferredValue",
    "useTransition",
    "useId",
    "useSyncExternalStore",
    "useInsertionEffect",
];

/// Hook metadata from database.
#[derive(Debug, Clone)]
pub struct HookInfo {
    pub id: i64,
    pub relpath: String,
    pub symbol_name: String,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub is_builtin: bool,
}

/// Detector for React hooks.
pub struct HookDetector {
    custom_hook_pattern: Regex,
}

impl HookDetector {
    /// Create a new hook detector.
    pub fn new() -> Self {
        Self {
            // Custom hooks must start with "use" followed by uppercase letter
            custom_hook_pattern: Regex::new(r"^use[A-Z][a-zA-Z0-9]*$").unwrap(),
        }
    }

    /// Check if a name matches the custom hook naming convention.
    ///
    /// Custom hooks must:
    /// - Start with "use"
    /// - Followed by an uppercase letter
    /// - Contain only alphanumeric characters
    ///
    /// Examples: useAuth, useLocalStorage, useFetch
    pub fn is_custom_hook(&self, name: &str) -> bool {
        self.custom_hook_pattern.is_match(name)
    }

    /// Check if a name is a built-in React hook.
    pub fn is_builtin_hook(&self, name: &str) -> bool {
        BUILT_IN_HOOKS.contains(&name)
    }

    /// Check if a name is any type of hook (built-in or custom).
    pub fn is_hook(&self, name: &str) -> bool {
        self.is_builtin_hook(name) || self.is_custom_hook(name)
    }

    /// Find hook calls in a code chunk's content.
    ///
    /// This searches for hook usage patterns in the code.
    ///
    /// # Arguments
    /// * `content` - Code content to search
    ///
    /// # Returns
    /// Vector of hook names found in the content
    pub fn find_hook_calls(&self, content: &str) -> Vec<String> {
        let mut hooks = Vec::new();

        // Pattern matches: hookName( or const [x, y] = hookName(
        let hook_call_pattern = Regex::new(r"\b(use[A-Z][a-zA-Z0-9]*)\s*\(").unwrap();

        for cap in hook_call_pattern.captures_iter(content) {
            if let Some(hook_name) = cap.get(1) {
                let name = hook_name.as_str().to_string();
                if self.is_hook(&name) && !hooks.contains(&name) {
                    hooks.push(name);
                }
            }
        }

        hooks
    }

    /// Find hook definitions that are called by the given component.
    ///
    /// This queries the database to find hook chunks that are imported
    /// or used by the target chunk.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `chunk_id` - Component chunk to find hooks for
    ///
    /// # Returns
    /// Vector of hook information ordered by relevance
    pub async fn find_used_hooks(
        &self,
        store: &SqliteStore,
        chunk_id: i64,
    ) -> Result<Vec<HookInfo>> {
        use crate::db::sqlite::graph::ImportDirection;

        let mut hooks = Vec::new();

        // Get the chunk to analyze its content for hook calls
        let chunk = store.get_chunk_by_id(chunk_id).await?;
        if chunk.is_none() {
            return Ok(vec![]);
        }
        let chunk = chunk.unwrap();

        // Find hook names used in this chunk
        let hook_names = self.find_hook_calls(&chunk.preview);

        // Find chunks that this component calls/imports
        let callees = store.find_callees(chunk_id, Some(1)).await?;
        let imports = store
            .find_imports(chunk_id, ImportDirection::Outgoing, Some(1))
            .await?;

        // Combine and deduplicate chunk IDs to check
        let mut chunk_ids_to_check: Vec<i64> = callees.iter().map(|c| c.chunk_id).collect();
        for import in imports {
            if !chunk_ids_to_check.contains(&import.chunk_id) {
                chunk_ids_to_check.push(import.chunk_id);
            }
        }

        // Check each related chunk for hook definitions
        for related_chunk_id in chunk_ids_to_check {
            if let Some(related_chunk) = store.get_chunk_by_id(related_chunk_id).await? {
                if let Some(ref symbol_name) = related_chunk.symbol_name {
                    // Check if this is a hook that's used by the target
                    if self.is_hook(symbol_name) && hook_names.contains(symbol_name) {
                        hooks.push(HookInfo {
                            id: related_chunk.id,
                            relpath: related_chunk.file_path,
                            symbol_name: symbol_name.clone(),
                            kind: related_chunk.kind,
                            start_line: related_chunk.start_line,
                            end_line: related_chunk.end_line,
                            is_builtin: self.is_builtin_hook(symbol_name),
                        });
                    }
                }
            }
        }

        Ok(hooks)
    }

    /// Find all custom hooks defined in the codebase.
    ///
    /// Searches for function chunks with names matching the custom hook pattern
    /// (use[A-Z]*) that are not built-in hooks.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `worktree_id` - Optional worktree to limit search
    ///
    /// # Returns
    /// Vector of hook information
    pub async fn find_all_custom_hooks(
        &self,
        store: &SqliteStore,
        _worktree_id: Option<i64>,
    ) -> Result<Vec<HookInfo>> {
        // Use FTS search to find functions with "use" prefix
        // Search for "use" in all repos, then filter to custom hooks
        let search_results = store
            .search_chunks_fts(
                "*",  // All repos
                None, // All worktrees initially
                "use", 100, // Get more results to filter
                false, None, None,
            )
            .await?;

        let mut hooks = Vec::new();

        for hit in search_results {
            // Check if this is a custom hook
            if let Some(ref symbol_name) = hit.symbol_name {
                if self.is_custom_hook(symbol_name) && !self.is_builtin_hook(symbol_name) {
                    // Filter by worktree if specified (can't do this in the query easily)
                    // For now, include all results (worktree filtering would require additional lookup)
                    hooks.push(HookInfo {
                        id: hit.chunk_id,
                        relpath: hit.file_relpath,
                        symbol_name: symbol_name.clone(),
                        kind: hit.kind,
                        start_line: hit.start_line,
                        end_line: hit.end_line,
                        is_builtin: false,
                    });
                }
            }
        }

        Ok(hooks)
    }

    /// Find hook by name.
    ///
    /// Searches for a specific hook definition in the codebase.
    ///
    /// # Arguments
    /// * `store` - SQLite store
    /// * `hook_name` - Name of the hook to find
    /// * `worktree_id` - Optional worktree to limit search
    ///
    /// # Returns
    /// Hook information if found
    pub async fn find_hook_by_name(
        &self,
        store: &SqliteStore,
        hook_name: &str,
        _worktree_id: Option<i64>,
    ) -> Result<Option<HookInfo>> {
        // Built-in hooks don't have definitions in the codebase
        if self.is_builtin_hook(hook_name) {
            return Ok(None);
        }

        // Search for the specific hook name
        let search_results = store
            .search_chunks_fts(
                "*",  // All repos
                None, // All worktrees
                hook_name, 10, // Just need to find one
                false, None, None,
            )
            .await?;

        // Find exact match
        for hit in search_results {
            if let Some(ref symbol_name) = hit.symbol_name {
                if symbol_name == hook_name && self.is_custom_hook(symbol_name) {
                    return Ok(Some(HookInfo {
                        id: hit.chunk_id,
                        relpath: hit.file_relpath,
                        symbol_name: symbol_name.clone(),
                        kind: hit.kind,
                        start_line: hit.start_line,
                        end_line: hit.end_line,
                        is_builtin: false,
                    }));
                }
            }
        }

        Ok(None)
    }
}

impl Default for HookDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_hook() {
        let detector = HookDetector::new();

        // Built-in hooks
        assert!(detector.is_builtin_hook("useState"));
        assert!(detector.is_builtin_hook("useEffect"));
        assert!(detector.is_builtin_hook("useContext"));
        assert!(detector.is_builtin_hook("useReducer"));
        assert!(detector.is_builtin_hook("useCallback"));
        assert!(detector.is_builtin_hook("useMemo"));
        assert!(detector.is_builtin_hook("useRef"));

        // Not built-in
        assert!(!detector.is_builtin_hook("useAuth"));
        assert!(!detector.is_builtin_hook("useLocalStorage"));
        assert!(!detector.is_builtin_hook("useFetch"));
    }

    #[test]
    fn test_is_custom_hook() {
        let detector = HookDetector::new();

        // Valid custom hooks
        assert!(detector.is_custom_hook("useAuth"));
        assert!(detector.is_custom_hook("useLocalStorage"));
        assert!(detector.is_custom_hook("useFetch"));
        assert!(detector.is_custom_hook("useToggle"));
        assert!(detector.is_custom_hook("useDebounce"));

        // Invalid - doesn't follow convention
        // Note: useState DOES match the custom hook naming pattern (use + Uppercase)
        // but it's also a built-in hook. Use is_builtin_hook() to distinguish.
        assert!(detector.is_custom_hook("useState")); // Matches pattern (even though it's also built-in)
        assert!(!detector.is_custom_hook("use")); // No uppercase after "use"
        assert!(!detector.is_custom_hook("useauth")); // No uppercase after "use"
        assert!(!detector.is_custom_hook("Use")); // Starts uppercase
        assert!(!detector.is_custom_hook("myHook")); // Doesn't start with "use"
        assert!(!detector.is_custom_hook("use_auth")); // Contains underscore
    }

    #[test]
    fn test_is_hook() {
        let detector = HookDetector::new();

        // Built-in hooks
        assert!(detector.is_hook("useState"));
        assert!(detector.is_hook("useEffect"));

        // Custom hooks
        assert!(detector.is_hook("useAuth"));
        assert!(detector.is_hook("useFetch"));

        // Not hooks
        assert!(!detector.is_hook("myFunction"));
        assert!(!detector.is_hook("Component"));
        assert!(!detector.is_hook("use"));
    }

    #[test]
    fn test_find_hook_calls_builtin() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const [count, setCount] = useState(0);
                useEffect(() => {
                    console.log(count);
                }, [count]);
                return <div>{count}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        assert_eq!(hooks.len(), 2);
        assert!(hooks.contains(&"useState".to_string()));
        assert!(hooks.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_find_hook_calls_custom() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const user = useAuth();
                const data = useFetch('/api/data');
                const [isOpen, toggle] = useToggle(false);
                return <div>{user.name}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        assert_eq!(hooks.len(), 3);
        assert!(hooks.contains(&"useAuth".to_string()));
        assert!(hooks.contains(&"useFetch".to_string()));
        assert!(hooks.contains(&"useToggle".to_string()));
    }

    #[test]
    fn test_find_hook_calls_mixed() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const [user, setUser] = useState(null);
                const auth = useAuth();
                useEffect(() => {
                    if (auth.isLoggedIn) {
                        setUser(auth.user);
                    }
                }, [auth]);
                return <div>{user?.name}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        assert_eq!(hooks.len(), 3);
        assert!(hooks.contains(&"useState".to_string()));
        assert!(hooks.contains(&"useAuth".to_string()));
        assert!(hooks.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_find_hook_calls_no_duplicates() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const [count1, setCount1] = useState(0);
                const [count2, setCount2] = useState(1);
                const [count3, setCount3] = useState(2);
                return <div>{count1 + count2 + count3}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        // Should only have one entry for useState
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0], "useState");
    }

    #[test]
    fn test_find_hook_calls_no_false_positives() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                // This is a comment about useState
                const message = "useState is useful";
                return <div>use Effects carefully</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        // Should find no hooks - no hook calls in this code
        assert_eq!(hooks.len(), 0);
    }

    #[test]
    fn test_find_hook_calls_complex_destructuring() {
        let detector = HookDetector::new();

        let code = r#"
            function Component() {
                const { user, loading, error } = useAuth();
                const [{ data, status }, dispatch] = useReducer(reducer, initialState);
                return <div>{data}</div>;
            }
        "#;

        let hooks = detector.find_hook_calls(code);

        assert_eq!(hooks.len(), 2);
        assert!(hooks.contains(&"useAuth".to_string()));
        assert!(hooks.contains(&"useReducer".to_string()));
    }

    // Database tests are in integration tests
    #[tokio::test]
    #[ignore]
    async fn test_find_used_hooks() {
        // Integration test - requires database
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_all_custom_hooks() {
        // Integration test - requires database
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_hook_by_name() {
        // Integration test - requires database
    }
}
