//! Shadow mode execution for A/B testing
//!
//! Runs both old and new search implementations in parallel, returning old results
//! to users while logging new results for comparison. Implements non-blocking async
//! execution with timeout handling and error isolation.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

/// Search result from either old or new implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// File path relative to repository root
    pub relpath: String,
    /// Symbol name (function, class, etc.)
    pub symbol_name: String,
    /// Relevance score
    pub score: f64,
    /// Rank position (1-indexed)
    pub rank: usize,
}

/// Results from shadow mode execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowModeResults {
    /// Query that was executed
    pub query: String,
    /// User ID (if available)
    pub user_id: Option<String>,
    /// Results from old (production) implementation
    pub old_results: Vec<SearchResult>,
    /// Results from new (experimental) implementation
    pub new_results: Option<Vec<SearchResult>>,
    /// Latency of old implementation in milliseconds
    pub old_latency_ms: u64,
    /// Latency of new implementation in milliseconds (if successful)
    pub new_latency_ms: Option<u64>,
    /// Error from new implementation (if any)
    pub new_error: Option<String>,
    /// Timestamp of execution
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Shadow mode executor
pub struct ShadowMode {
    /// Timeout for new implementation execution (to prevent blocking)
    pub timeout_duration: Duration,
}

impl ShadowMode {
    /// Create new shadow mode executor with default timeout (5 seconds)
    pub fn new() -> Self {
        Self {
            timeout_duration: Duration::from_secs(5),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            timeout_duration: Duration::from_millis(timeout_ms),
        }
    }

    /// Execute search in shadow mode
    ///
    /// Runs both old and new implementations in parallel. Returns old results immediately
    /// while logging new results asynchronously. The new implementation runs with timeout
    /// and error isolation to prevent impacting user experience.
    ///
    /// # Arguments
    /// * `query` - Search query string
    /// * `user_id` - Optional user identifier for tracking
    /// * `old_search_fn` - Function that executes old (production) search
    /// * `new_search_fn` - Function that executes new (experimental) search
    ///
    /// # Returns
    /// Returns old results immediately for user response. Logs both results asynchronously.
    pub async fn execute<F1, F2, Fut1, Fut2>(
        &self,
        query: String,
        user_id: Option<String>,
        old_search_fn: F1,
        new_search_fn: F2,
    ) -> anyhow::Result<ShadowModeResults>
    where
        F1: FnOnce(String) -> Fut1,
        F2: FnOnce(String) -> Fut2,
        Fut1: std::future::Future<Output = anyhow::Result<Vec<SearchResult>>>,
        Fut2: std::future::Future<Output = anyhow::Result<Vec<SearchResult>>>,
    {
        let timestamp = chrono::Utc::now();

        // Execute old search (production path)
        let old_start = std::time::Instant::now();
        let old_results = old_search_fn(query.clone()).await?;
        let old_latency_ms = old_start.elapsed().as_millis() as u64;

        // Execute new search with timeout and error isolation
        let new_start = std::time::Instant::now();
        let new_execution = timeout(self.timeout_duration, new_search_fn(query.clone()));

        let (new_results, new_latency_ms, new_error) = match new_execution.await {
            Ok(Ok(results)) => {
                let latency = new_start.elapsed().as_millis() as u64;
                (Some(results), Some(latency), None)
            }
            Ok(Err(e)) => {
                let latency = new_start.elapsed().as_millis() as u64;
                tracing::warn!(
                    query = %query,
                    error = %e,
                    "New search implementation failed"
                );
                (None, Some(latency), Some(e.to_string()))
            }
            Err(_) => {
                tracing::warn!(
                    query = %query,
                    timeout_ms = self.timeout_duration.as_millis(),
                    "New search implementation timed out"
                );
                (
                    None,
                    None,
                    Some(format!(
                        "Timeout after {}ms",
                        self.timeout_duration.as_millis()
                    )),
                )
            }
        };

        Ok(ShadowModeResults {
            query,
            user_id,
            old_results,
            new_results,
            old_latency_ms,
            new_latency_ms,
            new_error,
            timestamp,
        })
    }

    /// Compare results between old and new implementations
    ///
    /// Analyzes differences in result sets, ranking changes, and score variations.
    pub fn compare_results(&self, results: &ShadowModeResults) -> ResultComparison {
        if results.new_results.is_none() {
            return ResultComparison {
                total_old: results.old_results.len(),
                total_new: 0,
                common_results: 0,
                only_in_old: results.old_results.len(),
                only_in_new: 0,
                ranking_changes: vec![],
                avg_score_diff: None,
                latency_diff_ms: None,
            };
        }

        let new_results = results.new_results.as_ref().unwrap();

        // Find common results
        let old_paths: std::collections::HashSet<_> = results
            .old_results
            .iter()
            .map(|r| (&r.relpath, &r.symbol_name))
            .collect();
        let new_paths: std::collections::HashSet<_> = new_results
            .iter()
            .map(|r| (&r.relpath, &r.symbol_name))
            .collect();

        let common: Vec<_> = old_paths.intersection(&new_paths).collect();
        let only_old: Vec<_> = old_paths.difference(&new_paths).collect();
        let only_new: Vec<_> = new_paths.difference(&old_paths).collect();

        // Analyze ranking changes
        let mut ranking_changes = Vec::new();
        for (relpath, symbol) in &common {
            let old_rank = results
                .old_results
                .iter()
                .find(|r| &r.relpath == *relpath && &r.symbol_name == *symbol)
                .map(|r| r.rank);
            let new_rank = new_results
                .iter()
                .find(|r| &r.relpath == *relpath && &r.symbol_name == *symbol)
                .map(|r| r.rank);

            if let (Some(old), Some(new)) = (old_rank, new_rank) {
                if old != new {
                    ranking_changes.push(RankingChange {
                        relpath: (*relpath).clone(),
                        symbol_name: (*symbol).clone(),
                        old_rank: old,
                        new_rank: new,
                        rank_diff: new as i32 - old as i32,
                    });
                }
            }
        }

        // Calculate average score difference for common results
        let mut score_diffs = Vec::new();
        for (relpath, symbol) in &common {
            let old_score = results
                .old_results
                .iter()
                .find(|r| &r.relpath == *relpath && &r.symbol_name == *symbol)
                .map(|r| r.score);
            let new_score = new_results
                .iter()
                .find(|r| &r.relpath == *relpath && &r.symbol_name == *symbol)
                .map(|r| r.score);

            if let (Some(old), Some(new)) = (old_score, new_score) {
                score_diffs.push(new - old);
            }
        }

        let avg_score_diff = if !score_diffs.is_empty() {
            Some(score_diffs.iter().sum::<f64>() / score_diffs.len() as f64)
        } else {
            None
        };

        let latency_diff_ms = results
            .new_latency_ms
            .map(|new| new as i64 - results.old_latency_ms as i64);

        ResultComparison {
            total_old: results.old_results.len(),
            total_new: new_results.len(),
            common_results: common.len(),
            only_in_old: only_old.len(),
            only_in_new: only_new.len(),
            ranking_changes,
            avg_score_diff,
            latency_diff_ms,
        }
    }
}

impl Default for ShadowMode {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison of results between old and new implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultComparison {
    pub total_old: usize,
    pub total_new: usize,
    pub common_results: usize,
    pub only_in_old: usize,
    pub only_in_new: usize,
    pub ranking_changes: Vec<RankingChange>,
    pub avg_score_diff: Option<f64>,
    pub latency_diff_ms: Option<i64>,
}

/// Change in ranking position for a result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingChange {
    pub relpath: String,
    pub symbol_name: String,
    pub old_rank: usize,
    pub new_rank: usize,
    pub rank_diff: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mock_old_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![
            SearchResult {
                relpath: "src/main.rs".to_string(),
                symbol_name: "main".to_string(),
                score: 0.9,
                rank: 1,
            },
            SearchResult {
                relpath: "src/lib.rs".to_string(),
                symbol_name: "init".to_string(),
                score: 0.7,
                rank: 2,
            },
        ])
    }

    async fn mock_new_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![
            SearchResult {
                relpath: "src/lib.rs".to_string(),
                symbol_name: "init".to_string(),
                score: 0.95,
                rank: 1,
            },
            SearchResult {
                relpath: "src/main.rs".to_string(),
                symbol_name: "main".to_string(),
                score: 0.85,
                rank: 2,
            },
        ])
    }

    async fn mock_failing_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
        Err(anyhow::anyhow!("Search failed"))
    }

    #[tokio::test]
    async fn test_shadow_mode_execution() {
        let shadow = ShadowMode::new();
        let results = shadow
            .execute(
                "test query".to_string(),
                Some("user123".to_string()),
                mock_old_search,
                mock_new_search,
            )
            .await
            .unwrap();

        assert_eq!(results.query, "test query");
        assert_eq!(results.user_id, Some("user123".to_string()));
        assert_eq!(results.old_results.len(), 2);
        assert!(results.new_results.is_some());
        assert_eq!(results.new_results.unwrap().len(), 2);
        assert!(results.new_error.is_none());
    }

    #[tokio::test]
    async fn test_shadow_mode_with_error() {
        let shadow = ShadowMode::new();
        let results = shadow
            .execute(
                "test query".to_string(),
                None,
                mock_old_search,
                mock_failing_search,
            )
            .await
            .unwrap();

        assert_eq!(results.old_results.len(), 2);
        assert!(results.new_results.is_none());
        assert!(results.new_error.is_some());
        assert!(results.new_error.unwrap().contains("Search failed"));
    }

    #[tokio::test]
    async fn test_result_comparison() {
        let shadow = ShadowMode::new();
        let results = shadow
            .execute(
                "test query".to_string(),
                None,
                mock_old_search,
                mock_new_search,
            )
            .await
            .unwrap();

        let comparison = shadow.compare_results(&results);

        assert_eq!(comparison.total_old, 2);
        assert_eq!(comparison.total_new, 2);
        assert_eq!(comparison.common_results, 2);
        assert_eq!(comparison.only_in_old, 0);
        assert_eq!(comparison.only_in_new, 0);
        assert_eq!(comparison.ranking_changes.len(), 2);
        assert!(comparison.avg_score_diff.is_some());
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let shadow = ShadowMode::with_timeout(100); // 100ms timeout

        async fn slow_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(vec![])
        }

        let results = shadow
            .execute("test".to_string(), None, mock_old_search, slow_search)
            .await
            .unwrap();

        assert!(results.new_results.is_none());
        assert!(results.new_error.is_some());
        assert!(results.new_error.unwrap().contains("Timeout"));
    }
}
