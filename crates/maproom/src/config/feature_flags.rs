//! Feature flag system for toggling search capabilities.

use serde::{Deserialize, Serialize};

/// Feature flags for controlling search behavior.
///
/// Feature flags allow enabling/disabling specific search capabilities without
/// code changes. This is useful for:
/// - Gradual rollout of new features
/// - A/B testing different search strategies
/// - Disabling resource-intensive features in low-resource environments
/// - Debugging and troubleshooting
///
/// # Examples
///
/// ```
/// use crewchief_maproom::config::FeatureFlags;
///
/// let flags = FeatureFlags::default();
/// assert!(flags.enable_vector_search);
/// assert!(flags.enable_hybrid_fusion);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable vector similarity search.
    ///
    /// When disabled, only FTS is used for retrieval.
    /// Disabling this can significantly reduce query latency and resource usage.
    pub enable_vector_search: bool,

    /// Enable hybrid fusion (combine multiple signals).
    ///
    /// When disabled, only FTS results are returned without fusion.
    /// This is useful for debugging or when you want pure keyword search.
    pub enable_hybrid_fusion: bool,

    /// Enable graph-based ranking signals.
    ///
    /// When disabled, graph importance is not used in scoring.
    /// Requires chunk_edges table to be populated.
    pub enable_graph_signals: bool,

    /// Enable temporal signals (recency and churn).
    ///
    /// When disabled, recency and churn are not used in scoring.
    /// Requires git metadata in chunks table.
    pub enable_temporal_signals: bool,

    /// Enable query result caching.
    ///
    /// When disabled, all queries are executed without caching.
    /// Useful for debugging or when cache invalidation is problematic.
    pub enable_query_cache: bool,

    /// Enable hot reload of fusion weights.
    ///
    /// When disabled, configuration changes require service restart.
    /// Disabling this can reduce resource usage from file watching.
    pub enable_hot_reload: bool,

    /// Enable quality-weighted graph scoring.
    ///
    /// When enabled, uses quality metrics (test coverage, PR review depth, CI success)
    /// in graph-based ranking. When disabled, uses legacy graph scoring.
    /// This is a gradual rollout flag - defaults to false for safety.
    #[serde(default)]
    pub enable_quality_weighted_graph: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_vector_search: true,
            enable_hybrid_fusion: true,
            enable_graph_signals: true,
            enable_temporal_signals: true,
            enable_query_cache: true,
            enable_hot_reload: true,
            enable_quality_weighted_graph: false, // Default: disabled for safe rollout
        }
    }
}

impl FeatureFlags {
    /// Create a new FeatureFlags with all features enabled.
    pub fn all_enabled() -> Self {
        Self::default()
    }

    /// Create a new FeatureFlags with all features disabled.
    pub fn all_disabled() -> Self {
        Self {
            enable_vector_search: false,
            enable_hybrid_fusion: false,
            enable_graph_signals: false,
            enable_temporal_signals: false,
            enable_query_cache: false,
            enable_hot_reload: false,
            enable_quality_weighted_graph: false,
        }
    }

    /// Create a minimal FeatureFlags with only FTS enabled.
    ///
    /// This is the fastest and most resource-efficient configuration,
    /// useful for high-load scenarios or limited resources.
    pub fn fts_only() -> Self {
        Self {
            enable_vector_search: false,
            enable_hybrid_fusion: false,
            enable_graph_signals: false,
            enable_temporal_signals: false,
            enable_query_cache: true,
            enable_hot_reload: false,
            enable_quality_weighted_graph: false,
        }
    }

    /// Check if any vector-based features are enabled.
    pub fn needs_embeddings(&self) -> bool {
        self.enable_vector_search
    }

    /// Check if any graph-based features are enabled.
    pub fn needs_graph_data(&self) -> bool {
        self.enable_graph_signals
    }

    /// Check if any temporal features are enabled.
    pub fn needs_temporal_data(&self) -> bool {
        self.enable_temporal_signals
    }

    /// Get a summary of enabled features.
    pub fn enabled_features(&self) -> Vec<&str> {
        let mut features = Vec::new();
        if self.enable_vector_search {
            features.push("vector_search");
        }
        if self.enable_hybrid_fusion {
            features.push("hybrid_fusion");
        }
        if self.enable_graph_signals {
            features.push("graph_signals");
        }
        if self.enable_temporal_signals {
            features.push("temporal_signals");
        }
        if self.enable_query_cache {
            features.push("query_cache");
        }
        if self.enable_hot_reload {
            features.push("hot_reload");
        }
        if self.enable_quality_weighted_graph {
            features.push("quality_weighted_graph");
        }
        features
    }

    /// Get a summary of disabled features.
    pub fn disabled_features(&self) -> Vec<&str> {
        let mut features = Vec::new();
        if !self.enable_vector_search {
            features.push("vector_search");
        }
        if !self.enable_hybrid_fusion {
            features.push("hybrid_fusion");
        }
        if !self.enable_graph_signals {
            features.push("graph_signals");
        }
        if !self.enable_temporal_signals {
            features.push("temporal_signals");
        }
        if !self.enable_query_cache {
            features.push("query_cache");
        }
        if !self.enable_hot_reload {
            features.push("hot_reload");
        }
        if !self.enable_quality_weighted_graph {
            features.push("quality_weighted_graph");
        }
        features
    }

    /// Get the count of enabled features.
    pub fn enabled_count(&self) -> usize {
        self.enabled_features().len()
    }

    /// Get the count of disabled features.
    pub fn disabled_count(&self) -> usize {
        self.disabled_features().len()
    }

    /// Check if all features are enabled.
    pub fn is_all_enabled(&self) -> bool {
        self.enabled_count() == 7
    }

    /// Check if all features are disabled.
    pub fn is_all_disabled(&self) -> bool {
        self.disabled_count() == 7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_flags() {
        let flags = FeatureFlags::default();
        assert!(flags.enable_vector_search);
        assert!(flags.enable_hybrid_fusion);
        assert!(flags.enable_graph_signals);
        assert!(flags.enable_temporal_signals);
        assert!(flags.enable_query_cache);
        assert!(flags.enable_hot_reload);
        // Quality weighted graph defaults to false
        assert!(!flags.enable_quality_weighted_graph);
        assert!(!flags.is_all_enabled());
    }

    #[test]
    fn test_all_enabled() {
        let flags = FeatureFlags::all_enabled();
        // all_enabled() uses default(), which has quality_weighted_graph=false
        assert!(!flags.is_all_enabled());
        assert_eq!(flags.enabled_count(), 6);
        assert_eq!(flags.disabled_count(), 1);
    }

    #[test]
    fn test_all_disabled() {
        let flags = FeatureFlags::all_disabled();
        assert!(flags.is_all_disabled());
        assert_eq!(flags.enabled_count(), 0);
        assert_eq!(flags.disabled_count(), 7);
    }

    #[test]
    fn test_fts_only() {
        let flags = FeatureFlags::fts_only();
        assert!(!flags.enable_vector_search);
        assert!(!flags.enable_hybrid_fusion);
        assert!(!flags.enable_graph_signals);
        assert!(!flags.enable_temporal_signals);
        assert!(flags.enable_query_cache);
        assert!(!flags.enable_hot_reload);
    }

    #[test]
    fn test_needs_methods() {
        let flags = FeatureFlags::default();
        assert!(flags.needs_embeddings());
        assert!(flags.needs_graph_data());
        assert!(flags.needs_temporal_data());

        let fts_flags = FeatureFlags::fts_only();
        assert!(!fts_flags.needs_embeddings());
        assert!(!fts_flags.needs_graph_data());
        assert!(!fts_flags.needs_temporal_data());
    }

    #[test]
    fn test_enabled_features() {
        let flags = FeatureFlags::default();
        let enabled = flags.enabled_features();
        assert_eq!(enabled.len(), 6);
        assert!(enabled.contains(&"vector_search"));
        assert!(enabled.contains(&"hybrid_fusion"));
        assert!(enabled.contains(&"graph_signals"));
        assert!(enabled.contains(&"temporal_signals"));
        assert!(enabled.contains(&"query_cache"));
        assert!(enabled.contains(&"hot_reload"));
        // Quality weighted graph is disabled by default
        assert!(!enabled.contains(&"quality_weighted_graph"));
    }

    #[test]
    fn test_disabled_features() {
        let flags = FeatureFlags::all_disabled();
        let disabled = flags.disabled_features();
        assert_eq!(disabled.len(), 7);
        assert!(disabled.contains(&"vector_search"));
        assert!(disabled.contains(&"hybrid_fusion"));
        assert!(disabled.contains(&"quality_weighted_graph"));
    }

    #[test]
    fn test_partial_flags() {
        let mut flags = FeatureFlags::all_disabled();
        flags.enable_vector_search = true;
        flags.enable_query_cache = true;

        assert!(!flags.is_all_enabled());
        assert!(!flags.is_all_disabled());
        assert_eq!(flags.enabled_count(), 2);
        assert_eq!(flags.disabled_count(), 5);
    }

    #[test]
    fn test_quality_weighted_graph_defaults_to_false() {
        let flags = FeatureFlags::default();
        assert_eq!(flags.enable_quality_weighted_graph, false);
    }

    #[test]
    fn test_quality_weighted_graph_can_be_enabled() {
        let mut flags = FeatureFlags::default();
        flags.enable_quality_weighted_graph = true;
        assert!(flags.enable_quality_weighted_graph);
        let enabled = flags.enabled_features();
        assert!(enabled.contains(&"quality_weighted_graph"));
    }

    #[test]
    fn test_backward_compat_deserialize_without_quality_flag() {
        // Simulate old config YAML without quality_weighted_graph field
        let yaml = r#"
enable_vector_search: true
enable_hybrid_fusion: true
enable_graph_signals: true
enable_temporal_signals: true
enable_query_cache: true
enable_hot_reload: true
        "#;
        let flags: FeatureFlags = serde_yaml::from_str(yaml).unwrap();
        // Should default to false for backward compatibility
        assert_eq!(flags.enable_quality_weighted_graph, false);
        assert!(flags.enable_vector_search);
        assert!(flags.enable_hybrid_fusion);
    }

    #[test]
    fn test_deserialize_with_quality_flag_true() {
        let yaml = r#"
enable_vector_search: true
enable_hybrid_fusion: true
enable_graph_signals: true
enable_temporal_signals: true
enable_query_cache: true
enable_hot_reload: true
enable_quality_weighted_graph: true
        "#;
        let flags: FeatureFlags = serde_yaml::from_str(yaml).unwrap();
        assert!(flags.enable_quality_weighted_graph);
    }

    #[test]
    fn test_deserialize_with_quality_flag_false() {
        let yaml = r#"
enable_vector_search: true
enable_hybrid_fusion: true
enable_graph_signals: true
enable_temporal_signals: true
enable_query_cache: true
enable_hot_reload: true
enable_quality_weighted_graph: false
        "#;
        let flags: FeatureFlags = serde_yaml::from_str(yaml).unwrap();
        assert!(!flags.enable_quality_weighted_graph);
        assert!(flags.enable_vector_search);
    }
}
