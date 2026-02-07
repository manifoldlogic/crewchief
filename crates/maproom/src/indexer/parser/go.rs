//! Go parser (stub implementation - Phase 1)

use crate::indexer::SymbolChunk;

/// Extract chunks from Go source code
pub(super) fn extract_go_chunks(_source: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}

/// Extract chunks from go.mod files
pub(super) fn extract_gomod_chunks(_source: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}
