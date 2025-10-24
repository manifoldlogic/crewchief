//! Monitoring and metrics infrastructure for hybrid search.
//!
//! This module provides comprehensive observability for the search system:
//! - Prometheus metrics for latency, throughput, and error tracking
//! - Structured logging with tracing
//! - Performance monitoring and alerting
//!
//! # Metrics Exposed
//!
//! - `maproom_search_query_latency_seconds` - End-to-end search latency histogram
//! - `maproom_search_fusion_time_seconds` - Score fusion computation time histogram
//! - `maproom_search_cache_hit_rate` - Cache effectiveness gauge (0.0-1.0)
//! - `maproom_search_result_count` - Number of results returned per query histogram
//! - `maproom_search_errors_total` - Counter for errors by type
//! - `maproom_search_queries_total` - Total number of search queries counter
//!
//! # Alert Thresholds
//!
//! - **p95 latency > 50ms**: Performance degradation warning
//! - **p95 latency > 100ms**: Critical performance issue
//! - **Error rate > 1%**: Reliability issue warning
//! - **Error rate > 5%**: Critical reliability issue
//! - **Cache hit rate < 50%**: Cache effectiveness warning
//!
//! # Usage
//!
//! ```no_run
//! use crewchief_maproom::metrics::{SearchMetrics, get_metrics};
//! use std::time::Instant;
//!
//! #[tokio::main]
//! async fn main() {
//!     let metrics = get_metrics();
//!
//!     let start = Instant::now();
//!     // ... perform search ...
//!     let duration = start.elapsed();
//!
//!     metrics.record_query_latency(duration.as_secs_f64(), "code", true);
//!     metrics.record_result_count(10, "code");
//!     metrics.increment_queries("code", true);
//! }
//! ```

pub mod prometheus;
pub mod search_metrics;

pub use prometheus::{init_metrics_server, metrics_handler};
pub use search_metrics::{get_metrics, SearchMetrics};
