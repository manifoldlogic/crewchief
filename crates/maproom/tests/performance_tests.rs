//! Performance test suite module.
//!
//! This file aggregates all performance-related tests into a single test target.
//! Individual test modules are in tests/performance/ directory.
//!
//! # Running
//!
//! ```bash
//! # Run all performance tests (requires DATABASE_URL)
//! cargo test --test performance_tests -- --ignored --nocapture
//!
//! # Run specific test module
//! cargo test --test performance_tests load_test -- --ignored --nocapture
//! ```

#[path = "performance/load_test.rs"]
mod load_test;

#[path = "performance/cache_effectiveness_test.rs"]
mod cache_effectiveness_test;

#[path = "performance/index_usage_test.rs"]
mod index_usage_test;
