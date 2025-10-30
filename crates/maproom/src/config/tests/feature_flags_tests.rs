//! Tests for feature flags.

use crate::config::FeatureFlags;

#[test]
fn test_default_feature_flags() {
    let flags = FeatureFlags::default();
    assert!(flags.enable_vector_search);
    assert!(flags.enable_hybrid_fusion);
    assert!(flags.enable_graph_signals);
    assert!(flags.enable_temporal_signals);
    assert!(flags.enable_query_cache);
    assert!(flags.enable_hot_reload);
    assert!(flags.is_all_enabled());
}

#[test]
fn test_all_enabled() {
    let flags = FeatureFlags::all_enabled();
    assert!(flags.is_all_enabled());
    assert_eq!(flags.enabled_count(), 6);
    assert_eq!(flags.disabled_count(), 0);
}

#[test]
fn test_all_disabled() {
    let flags = FeatureFlags::all_disabled();
    assert!(flags.is_all_disabled());
    assert_eq!(flags.enabled_count(), 0);
    assert_eq!(flags.disabled_count(), 6);
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
    assert_eq!(flags.enabled_count(), 1);
}

#[test]
fn test_needs_embeddings() {
    let mut flags = FeatureFlags::all_disabled();
    assert!(!flags.needs_embeddings());

    flags.enable_vector_search = true;
    assert!(flags.needs_embeddings());
}

#[test]
fn test_needs_graph_data() {
    let mut flags = FeatureFlags::all_disabled();
    assert!(!flags.needs_graph_data());

    flags.enable_graph_signals = true;
    assert!(flags.needs_graph_data());
}

#[test]
fn test_needs_temporal_data() {
    let mut flags = FeatureFlags::all_disabled();
    assert!(!flags.needs_temporal_data());

    flags.enable_temporal_signals = true;
    assert!(flags.needs_temporal_data());
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
}

#[test]
fn test_disabled_features() {
    let flags = FeatureFlags::all_disabled();
    let disabled = flags.disabled_features();
    assert_eq!(disabled.len(), 6);
    assert!(disabled.contains(&"vector_search"));
    assert!(disabled.contains(&"hybrid_fusion"));
    assert!(disabled.contains(&"graph_signals"));
    assert!(disabled.contains(&"temporal_signals"));
    assert!(disabled.contains(&"query_cache"));
    assert!(disabled.contains(&"hot_reload"));
}

#[test]
fn test_partial_flags() {
    let mut flags = FeatureFlags::all_disabled();
    flags.enable_vector_search = true;
    flags.enable_query_cache = true;

    assert!(!flags.is_all_enabled());
    assert!(!flags.is_all_disabled());
    assert_eq!(flags.enabled_count(), 2);
    assert_eq!(flags.disabled_count(), 4);

    let enabled = flags.enabled_features();
    assert_eq!(enabled.len(), 2);
    assert!(enabled.contains(&"vector_search"));
    assert!(enabled.contains(&"query_cache"));

    let disabled = flags.disabled_features();
    assert_eq!(disabled.len(), 4);
    assert!(disabled.contains(&"hybrid_fusion"));
    assert!(disabled.contains(&"graph_signals"));
}

#[test]
fn test_feature_flag_combinations() {
    // Hybrid search requires vector + fusion
    let mut flags = FeatureFlags::all_disabled();
    flags.enable_vector_search = true;
    flags.enable_hybrid_fusion = true;
    assert!(flags.needs_embeddings());
    assert_eq!(flags.enabled_count(), 2);

    // Graph-enhanced search requires graph signals
    let mut flags = FeatureFlags::all_disabled();
    flags.enable_graph_signals = true;
    assert!(flags.needs_graph_data());
    assert!(!flags.needs_embeddings());

    // Temporal-enhanced search requires temporal signals
    let mut flags = FeatureFlags::all_disabled();
    flags.enable_temporal_signals = true;
    assert!(flags.needs_temporal_data());
    assert!(!flags.needs_embeddings());
}

#[test]
fn test_serialization() {
    use serde_json;

    let flags = FeatureFlags::default();
    let json = serde_json::to_string(&flags).unwrap();
    let deserialized: FeatureFlags = serde_json::from_str(&json).unwrap();

    assert_eq!(
        flags.enable_vector_search,
        deserialized.enable_vector_search
    );
    assert_eq!(
        flags.enable_hybrid_fusion,
        deserialized.enable_hybrid_fusion
    );
    assert_eq!(
        flags.enable_graph_signals,
        deserialized.enable_graph_signals
    );
    assert_eq!(
        flags.enable_temporal_signals,
        deserialized.enable_temporal_signals
    );
    assert_eq!(flags.enable_query_cache, deserialized.enable_query_cache);
    assert_eq!(flags.enable_hot_reload, deserialized.enable_hot_reload);
}
