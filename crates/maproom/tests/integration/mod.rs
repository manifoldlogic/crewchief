//! Integration test modules for Maproom.
//!
//! This module registers all integration test submodules.

// Incremental indexing integration tests
pub mod incremental_scenarios;
pub mod concurrent_updates;
pub mod batch_processing;
pub mod failure_recovery;

// MCP and configuration tests
pub mod mcp_integration_test;
pub mod config_management_test;

// Monitoring and production readiness
pub mod monitoring_test;
pub mod production_readiness_test;
