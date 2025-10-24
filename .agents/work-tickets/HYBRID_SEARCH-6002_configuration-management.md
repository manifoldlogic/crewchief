# Ticket: HYBRID_SEARCH-6002: Configuration Management

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive configuration management system for hybrid search with support for YAML configuration files, hot reload of fusion weights, feature flags, and environment variable overrides. This system will enable dynamic tuning of search behavior without requiring application restarts.

## Background
The hybrid search system has multiple tunable parameters across embedding, fusion, performance, and indexing components. A robust configuration system is essential for:
- Enabling experimentation with different fusion weights and search parameters
- Supporting feature rollouts via feature flags
- Allowing runtime adjustments without service restarts (hot reload)
- Providing clear documentation for all configuration options
- Supporting different deployment environments through env var overrides

This configuration system will be the foundation for production deployments and A/B testing different search strategies.

## Acceptance Criteria
- [ ] Configuration file schema created with all sections (embedding, fusion, performance, index)
- [ ] YAML configuration file loading and validation implemented
- [ ] Hot reload mechanism for fusion weights working without restart
- [ ] Feature flag system implemented (enable_vector_search, enable_hybrid_fusion, enable_graph_signals)
- [ ] Environment variable override mechanism implemented
- [ ] Configuration validation with clear error messages on startup
- [ ] Complete configuration documentation created
- [ ] Default configuration file provided with sensible defaults
- [ ] Tests verify hot reload doesn't disrupt ongoing queries
- [ ] Tests verify invalid configurations are rejected with helpful errors

## Technical Requirements

### Configuration Schema
- YAML configuration file: `maproom-search.yml`
- Four main sections: `embedding`, `fusion`, `performance`, `index`
- Schema defined in Rust with serde deserialization
- Strong typing for all configuration values
- Default values for all optional parameters

### Configuration Sections
1. **Embedding Configuration**:
   - Provider selection (openai, cohere, local)
   - Model name and dimension
   - Cache size and TTL

2. **Fusion Configuration**:
   - Fusion method (rrf, weighted, learned)
   - RRF k parameter
   - Individual signal weights (fts, vector, graph, recency, churn)

3. **Performance Configuration**:
   - Max candidates per search method
   - Final result limit
   - Query timeout in milliseconds
   - Parallel query execution flag

4. **Index Configuration**:
   - IVFFlat list count
   - IVFFlat probe count
   - Index refresh interval

### Hot Reload Implementation
- File watcher monitoring `maproom-search.yml`
- Safe update mechanism using Arc<RwLock<Config>>
- Only fusion weights are hot-reloadable
- Validation before applying updates
- Graceful handling of invalid updates (keep existing config)
- Logging of configuration changes

### Feature Flags
- `enable_vector_search`: Toggle vector search on/off
- `enable_hybrid_fusion`: Toggle fusion vs single-method search
- `enable_graph_signals`: Toggle graph-based ranking signals
- Feature flags queryable at runtime
- Flags affect query pipeline construction

### Environment Variable Overrides
- Pattern: `MAPROOM_SEARCH_<SECTION>_<KEY>`
- Examples:
  - `MAPROOM_SEARCH_FUSION_WEIGHTS_FTS=0.5`
  - `MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS=200`
- Environment variables take precedence over config file
- Document all override paths

## Implementation Notes

### File Structure
```
crates/maproom/
├── config/
│   └── maproom-search.yml          # Default configuration
├── src/
│   └── config/
│       ├── mod.rs                  # Public API
│       ├── search_config.rs        # Configuration structs and loading
│       ├── hot_reload.rs           # Hot reload implementation
│       └── feature_flags.rs        # Feature flag system
└── docs/
    └── configuration_guide.md      # Complete documentation
```

### Configuration Loading Flow
1. Load default configuration from YAML file
2. Parse and validate against schema
3. Apply environment variable overrides
4. Initialize hot reload watcher if enabled
5. Return Arc<RwLock<SearchConfig>> for shared access

### Hot Reload Mechanism
```rust
// Pseudo-code structure
pub struct ConfigReloader {
    config: Arc<RwLock<SearchConfig>>,
    watcher: RecommendedWatcher,
}

impl ConfigReloader {
    pub async fn watch(&mut self) -> Result<()> {
        // Watch for file changes
        // Validate new configuration
        // Update only if valid
        // Log changes
    }
}
```

### Architecture Reference
Per `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md` (lines 264-294):
- Configuration schema matches architecture specification
- YAML format for human readability
- Clear separation of concerns across sections
- Tunable parameters for production optimization

### Dependencies on Crates
- `serde` + `serde_yaml` for YAML parsing
- `notify` for file system watching (hot reload)
- `config` crate for environment variable handling
- `anyhow` for error handling
- `tokio::sync::RwLock` for thread-safe config access

### Testing Strategy
- Unit tests for configuration parsing
- Unit tests for environment variable overrides
- Integration tests for hot reload mechanism
- Tests for invalid configuration rejection
- Tests for feature flag behavior
- Load testing to ensure hot reload doesn't impact performance

## Dependencies
- **HYBRID_SEARCH-3002** (weighted fusion) - Configuration needs weighted fusion implementation to configure
- No other blocking dependencies - this is infrastructure work

## Risk Assessment

- **Risk**: Hot reload during active queries could cause inconsistent results
  - **Mitigation**: Use RwLock to ensure atomic config updates; read lock during query execution

- **Risk**: Invalid hot reload could break running service
  - **Mitigation**: Validate all updates before applying; keep existing config if validation fails; comprehensive error logging

- **Risk**: Environment variable overrides could be confusing to debug
  - **Mitigation**: Log all active overrides at startup; document override precedence clearly; consider adding config dump endpoint

- **Risk**: Feature flags could lead to untested code paths
  - **Mitigation**: Test all feature flag combinations; use flags only for well-tested features; document flag implications

- **Risk**: Configuration drift between environments
  - **Mitigation**: Version control default config; document all env var overrides per environment; consider config validation tooling

## Files/Packages Affected

### Files to Create
- `/workspace/crates/maproom/config/maproom-search.yml` - Default configuration file
- `/workspace/crates/maproom/src/config/mod.rs` - Module definition and public API
- `/workspace/crates/maproom/src/config/search_config.rs` - Configuration structs and loading
- `/workspace/crates/maproom/src/config/hot_reload.rs` - Hot reload implementation
- `/workspace/crates/maproom/src/config/feature_flags.rs` - Feature flag system
- `/workspace/crates/maproom/docs/configuration_guide.md` - Complete documentation

### Files to Modify
- `/workspace/crates/maproom/Cargo.toml` - Add config-related dependencies
- `/workspace/crates/maproom/src/lib.rs` - Export config module
- `/workspace/crates/maproom/src/search/mod.rs` - Integrate configuration
- `/workspace/crates/maproom/src/search/hybrid.rs` - Use configuration for fusion
- `/workspace/crates/maproom/src/embedding/mod.rs` - Use configuration for embedding settings

### Test Files to Create
- `/workspace/crates/maproom/src/config/tests/config_tests.rs` - Configuration loading tests
- `/workspace/crates/maproom/src/config/tests/hot_reload_tests.rs` - Hot reload tests
- `/workspace/crates/maproom/src/config/tests/feature_flags_tests.rs` - Feature flag tests

## Planning Document References
- Architecture: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md` (Configuration Schema section, lines 264-294)
- Phase 6 Planning: Configuration Management task (Week 6, Task 2)
