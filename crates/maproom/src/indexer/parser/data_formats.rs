//! Data format parsers (JSON, YAML, TOML) - stub implementation (Phase 1)

use crate::indexer::SymbolChunk;

/// Extract chunks from JSON files
pub(super) fn extract_json_chunks(_source: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}

/// Extract chunks from YAML files
pub(super) fn extract_yaml_chunks(_source: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}

/// Extract chunks from TOML files
pub(super) fn extract_toml_chunks(_source: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}
