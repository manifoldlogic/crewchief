//! A/B Testing Framework for Hybrid Search
//!
//! This module provides comprehensive A/B testing infrastructure for validating
//! search quality improvements before full production rollout.
//!
//! # Features
//!
//! - **Shadow Mode**: Run new algorithms in parallel with production without user impact
//! - **Traffic Splitting**: Percentage-based experiment rollout (0-100%)
//! - **Event Logging**: Capture shadow results and user interactions
//! - **Statistical Analysis**: Chi-square, t-tests, confidence intervals
//! - **Quality Gates**: Automated validation before promotion
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use crewchief_maproom::ab_testing::*;
//! use uuid::Uuid;
//!
//! // Create experiment
//! let config = ExperimentConfig::new("hybrid-weights-v2".to_string(), 25);
//! let manager = ExperimentManager::new(db_pool.clone());
//! let experiment_id = manager.create_experiment(config).await?;
//!
//! // Run search in shadow mode
//! let shadow = ShadowMode::new();
//! let results = shadow.execute(
//!     query.clone(),
//!     user_id,
//!     |q| old_search(q),
//!     |q| new_search(q),
//! ).await?;
//!
//! // Log results
//! let logger = ABTestLogger::new(db_pool);
//! logger.log_shadow_results(experiment_id, &results).await?;
//!
//! // Analyze results
//! let analyzer = StatisticalAnalyzer::new();
//! let test_result = analyzer.chi_square_test(
//!     old_clicks, old_total,
//!     new_clicks, new_total,
//! )?;
//!
//! if test_result.is_significant {
//!     println!("Improvement is statistically significant!");
//! }
//! ```
//!
//! # Architecture
//!
//! The A/B testing framework consists of several components:
//!
//! - [`framework`]: Core experiment configuration and lifecycle management
//! - [`shadow_mode`]: Parallel execution of old and new implementations
//! - [`logger`]: Batch logging of results and interaction events
//! - [`analysis`]: Statistical tests and confidence intervals
//!
//! # Quality Gates
//!
//! Before promoting an experiment to full rollout, validate quality gates:
//!
//! - Recall >80% at k=10
//! - Precision >70% at k=10
//! - NDCG >0.75
//! - Statistical significance at p<0.05
//! - No latency degradation (p95 < baseline + 10ms)
//! - No error rate increase
//!
//! # Performance Considerations
//!
//! - Shadow mode adds <10ms latency to search requests
//! - Event logging uses batch writes for efficiency
//! - Start with low rollout percentages (5-25%)
//! - Use sampling for very high traffic scenarios
//! - Configure retention policies for log data (default 90 days)

pub mod analysis;
pub mod dashboard;
pub mod framework;
pub mod logger;
pub mod shadow_mode;

// Re-export commonly used types
pub use analysis::{ConfidenceInterval, StatisticalAnalyzer, StatisticalTestResult};
pub use dashboard::{Dashboard, DashboardMetrics, MetricComparison, QualityMetrics, SegmentMetrics, TimeSeriesPoint};
pub use framework::{
    ExperimentConfig, ExperimentManager, ExperimentStatus, QualityGates, TrafficSplitter,
};
pub use logger::{ABTestLogger, InteractionEvent, InteractionEventType, ShadowResultLog};
pub use shadow_mode::{RankingChange, ResultComparison, SearchResult, ShadowMode, ShadowModeResults};
