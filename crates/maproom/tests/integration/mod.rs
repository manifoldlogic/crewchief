//! Integration test modules for Maproom.
//!
//! This module registers all integration test submodules.

// MCP and configuration tests
pub mod mcp_integration_test;
pub mod config_management_test;

// Monitoring and production readiness
pub mod monitoring_test;
pub mod production_readiness_test;

// MD_ENHANCE quality and performance testing
pub mod quality_test;
pub mod performance_test;
