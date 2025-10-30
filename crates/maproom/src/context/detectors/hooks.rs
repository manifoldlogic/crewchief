//! React hook detection.
//!
//! This module provides functionality to detect React hooks:
//! - Built-in hooks (useState, useEffect, useContext, etc.)
//! - Custom hooks (use* naming convention)
//! - Hook dependencies and relationships

use anyhow::{Context as AnyhowContext, Result};
use regex::Regex;
use tokio_postgres::Client;

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
        let hook_call_pattern =
            Regex::new(r"\b(use[A-Z][a-zA-Z0-9]*)\s*\(").unwrap();

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
    /// * `client` - PostgreSQL client
    /// * `chunk_id` - Component chunk to find hooks for
    ///
    /// # Returns
    /// Vector of hook information ordered by relevance
    pub async fn find_used_hooks(
        &self,
        client: &Client,
        chunk_id: i64,
    ) -> Result<Vec<HookInfo>> {
        // Query for chunks that:
        // 1. Are called/imported by the target chunk
        // 2. Have symbol names matching hook patterns
        let query = r#"
            WITH hook_calls AS (
                -- Find direct imports/calls from the target chunk
                SELECT DISTINCT
                    c.id,
                    f.relpath,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line
                FROM maproom.chunk_edges ce
                JOIN maproom.chunks c ON c.id = ce.dst_chunk_id
                JOIN maproom.files f ON f.id = c.file_id
                WHERE ce.src_chunk_id = $1
                  AND ce.relationship IN ('calls', 'imports')
                  AND c.symbol_name IS NOT NULL
            )
            SELECT
                id,
                relpath,
                symbol_name,
                kind,
                start_line,
                end_line,
                CASE
                    WHEN symbol_name = ANY($2::text[]) THEN true
                    ELSE false
                END as is_builtin
            FROM hook_calls
            WHERE symbol_name ~ '^use[A-Z][a-zA-Z0-9]*$'
            ORDER BY is_builtin DESC, symbol_name ASC;
        "#;

        let builtin_hooks: Vec<&str> = BUILT_IN_HOOKS.to_vec();
        let rows = client
            .query(query, &[&chunk_id, &builtin_hooks])
            .await
            .context("Failed to query hook usage")?;

        let hooks = rows
            .into_iter()
            .map(|row| HookInfo {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                is_builtin: row.get(6),
            })
            .collect();

        Ok(hooks)
    }

    /// Find all custom hooks defined in the codebase.
    ///
    /// # Arguments
    /// * `client` - PostgreSQL client
    /// * `worktree_id` - Optional worktree to limit search
    ///
    /// # Returns
    /// Vector of hook information
    pub async fn find_all_custom_hooks(
        &self,
        client: &Client,
        worktree_id: Option<i64>,
    ) -> Result<Vec<HookInfo>> {
        let query = if worktree_id.is_some() {
            r#"
                SELECT
                    c.id,
                    f.relpath,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line,
                    false as is_builtin
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE f.worktree_id = $1
                  AND c.symbol_name ~ '^use[A-Z][a-zA-Z0-9]*$'
                  AND c.kind IN ('func', 'arrow_func', 'function')
                ORDER BY c.symbol_name ASC;
            "#
        } else {
            r#"
                SELECT
                    c.id,
                    f.relpath,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line,
                    false as is_builtin
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE c.symbol_name ~ '^use[A-Z][a-zA-Z0-9]*$'
                  AND c.kind IN ('func', 'arrow_func', 'function')
                ORDER BY c.symbol_name ASC;
            "#
        };

        let rows = if let Some(wt_id) = worktree_id {
            client
                .query(query, &[&wt_id])
                .await
                .context("Failed to query custom hooks")?
        } else {
            client
                .query(query, &[])
                .await
                .context("Failed to query custom hooks")?
        };

        let hooks = rows
            .into_iter()
            .map(|row| HookInfo {
                id: row.get(0),
                relpath: row.get(1),
                symbol_name: row.get(2),
                kind: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                is_builtin: row.get(6),
            })
            .collect();

        Ok(hooks)
    }

    /// Find hook by name.
    ///
    /// # Arguments
    /// * `client` - PostgreSQL client
    /// * `hook_name` - Name of the hook to find
    /// * `worktree_id` - Optional worktree to limit search
    ///
    /// # Returns
    /// Hook information if found
    pub async fn find_hook_by_name(
        &self,
        client: &Client,
        hook_name: &str,
        worktree_id: Option<i64>,
    ) -> Result<Option<HookInfo>> {
        // Built-in hooks don't have definitions in the codebase
        if self.is_builtin_hook(hook_name) {
            return Ok(None);
        }

        let query = if worktree_id.is_some() {
            r#"
                SELECT
                    c.id,
                    f.relpath,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line,
                    false as is_builtin
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE f.worktree_id = $1
                  AND c.symbol_name = $2
                  AND c.kind IN ('func', 'arrow_func', 'function')
                LIMIT 1;
            "#
        } else {
            r#"
                SELECT
                    c.id,
                    f.relpath,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line,
                    false as is_builtin
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE c.symbol_name = $1
                  AND c.kind IN ('func', 'arrow_func', 'function')
                LIMIT 1;
            "#
        };

        let row = if let Some(wt_id) = worktree_id {
            client
                .query_opt(query, &[&wt_id, &hook_name])
                .await
                .context("Failed to query hook by name")?
        } else {
            client
                .query_opt(query, &[&hook_name])
                .await
                .context("Failed to query hook by name")?
        };

        Ok(row.map(|r| HookInfo {
            id: r.get(0),
            relpath: r.get(1),
            symbol_name: r.get(2),
            kind: r.get(3),
            start_line: r.get(4),
            end_line: r.get(5),
            is_builtin: r.get(6),
        }))
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
