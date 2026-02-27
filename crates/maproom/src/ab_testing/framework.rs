//! A/B testing framework core orchestration
//!
//! This module provides experiment configuration, traffic splitting, lifecycle management,
//! and quality gates validation for A/B testing hybrid search algorithms.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Experiment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExperimentStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

impl std::fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExperimentStatus::Running => write!(f, "running"),
            ExperimentStatus::Paused => write!(f, "paused"),
            ExperimentStatus::Completed => write!(f, "completed"),
            ExperimentStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for ExperimentStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(ExperimentStatus::Running),
            "paused" => Ok(ExperimentStatus::Paused),
            "completed" => Ok(ExperimentStatus::Completed),
            "failed" => Ok(ExperimentStatus::Failed),
            _ => Err(anyhow::anyhow!("Invalid experiment status: {}", s)),
        }
    }
}

/// Quality gates that must be met before full rollout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    /// Minimum recall at k=10 (e.g., 0.80 for 80%)
    pub min_recall: f64,
    /// Minimum precision at k=10 (e.g., 0.70 for 70%)
    pub min_precision: f64,
    /// Minimum NDCG score (e.g., 0.75)
    pub min_ndcg: f64,
    /// Maximum allowed p95 latency increase in milliseconds
    pub max_latency_increase_ms: i32,
    /// Maximum allowed error rate increase (e.g., 0.01 for 1%)
    pub max_error_rate_increase: f64,
    /// Required statistical significance p-value (e.g., 0.05)
    pub significance_threshold: f64,
}

impl Default for QualityGates {
    fn default() -> Self {
        Self {
            min_recall: 0.80,
            min_precision: 0.70,
            min_ndcg: 0.75,
            max_latency_increase_ms: 10,
            max_error_rate_increase: 0.01,
            significance_threshold: 0.05,
        }
    }
}

/// Experiment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// Unique experiment ID
    pub id: Uuid,
    /// Human-readable experiment name
    pub name: String,
    /// Detailed description of the experiment
    pub description: Option<String>,
    /// Percentage of traffic to route to new implementation (0-100)
    pub rollout_percentage: i32,
    /// Experiment start date
    pub start_date: DateTime<Utc>,
    /// Experiment end date (optional, for scheduled experiments)
    pub end_date: Option<DateTime<Utc>>,
    /// Current experiment status
    pub status: ExperimentStatus,
    /// Quality gates for promotion to full rollout
    pub quality_gates: QualityGates,
    /// Additional experiment-specific configuration
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExperimentConfig {
    /// Create a new experiment configuration
    pub fn new(name: String, rollout_percentage: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            rollout_percentage: rollout_percentage.clamp(0, 100),
            start_date: Utc::now(),
            end_date: None,
            status: ExperimentStatus::Running,
            quality_gates: QualityGates::default(),
            metadata: HashMap::new(),
        }
    }

    /// Check if experiment is currently active
    pub fn is_active(&self) -> bool {
        if self.status != ExperimentStatus::Running {
            return false;
        }

        let now = Utc::now();
        if now < self.start_date {
            return false;
        }

        if let Some(end_date) = self.end_date {
            if now > end_date {
                return false;
            }
        }

        true
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Experiment name cannot be empty"));
        }

        if self.rollout_percentage < 0 || self.rollout_percentage > 100 {
            return Err(anyhow::anyhow!(
                "Rollout percentage must be between 0 and 100, got {}",
                self.rollout_percentage
            ));
        }

        if let Some(end_date) = self.end_date {
            if end_date <= self.start_date {
                return Err(anyhow::anyhow!("End date must be after start date"));
            }
        }

        // Validate quality gates
        if self.quality_gates.min_recall < 0.0 || self.quality_gates.min_recall > 1.0 {
            return Err(anyhow::anyhow!("min_recall must be between 0.0 and 1.0"));
        }
        if self.quality_gates.min_precision < 0.0 || self.quality_gates.min_precision > 1.0 {
            return Err(anyhow::anyhow!("min_precision must be between 0.0 and 1.0"));
        }
        if self.quality_gates.min_ndcg < 0.0 || self.quality_gates.min_ndcg > 1.0 {
            return Err(anyhow::anyhow!("min_ndcg must be between 0.0 and 1.0"));
        }

        Ok(())
    }
}

/// Traffic splitter for routing queries to old vs new implementation
pub struct TrafficSplitter {
    /// Random number generator seed for reproducibility
    seed: u64,
}

impl TrafficSplitter {
    pub fn new() -> Self {
        Self {
            seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Determine if a query should use the new implementation
    ///
    /// Uses stable hashing to ensure consistent routing for the same user/query combination.
    /// This allows for reproducible experiments and prevents users from flipping between
    /// old and new implementations.
    pub fn should_use_new_implementation(
        &self,
        experiment: &ExperimentConfig,
        user_id: Option<&str>,
        query: &str,
    ) -> bool {
        if !experiment.is_active() {
            return false;
        }

        if experiment.rollout_percentage == 0 {
            return false;
        }

        if experiment.rollout_percentage == 100 {
            return true;
        }

        // Create stable hash from user_id and query
        let hash_input = format!(
            "{}:{}:{}",
            experiment.id,
            user_id.unwrap_or("anonymous"),
            query
        );
        let hash = self.hash_string(&hash_input);

        // Map hash to 0-100 range
        let bucket = hash % 100;

        bucket < experiment.rollout_percentage as u64
    }

    /// Simple hash function for stable traffic splitting
    fn hash_string(&self, s: &str) -> u64 {
        let mut hash = self.seed;
        for byte in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }
}

impl Default for TrafficSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Experiment lifecycle manager
pub struct ExperimentManager {
    // Stubbed: A/B testing not yet implemented for SQLite
    _phantom: std::marker::PhantomData<()>,
}

impl Default for ExperimentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExperimentManager {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new experiment
    pub async fn create_experiment(&self, _config: ExperimentConfig) -> anyhow::Result<Uuid> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Get experiment by ID
    pub async fn get_experiment(&self, _id: Uuid) -> anyhow::Result<Option<ExperimentConfig>> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// List all active experiments
    pub async fn list_active_experiments(&self) -> anyhow::Result<Vec<ExperimentConfig>> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Update experiment status
    pub async fn update_status(
        &self,
        _id: Uuid,
        _new_status: ExperimentStatus,
    ) -> anyhow::Result<()> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Pause an experiment
    pub async fn pause_experiment(&self, id: Uuid) -> anyhow::Result<()> {
        self.update_status(id, ExperimentStatus::Paused).await
    }

    /// Resume a paused experiment
    pub async fn resume_experiment(&self, id: Uuid) -> anyhow::Result<()> {
        self.update_status(id, ExperimentStatus::Running).await
    }

    /// Complete an experiment
    pub async fn complete_experiment(&self, id: Uuid) -> anyhow::Result<()> {
        self.update_status(id, ExperimentStatus::Completed).await
    }

    /// Update rollout percentage for gradual rollout
    pub async fn update_rollout(&self, _id: Uuid, _new_percentage: i32) -> anyhow::Result<()> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Validate quality gates for an experiment
    ///
    /// Returns true if the experiment meets all quality gates and can be promoted
    #[allow(clippy::too_many_arguments)] // Stubbed API; will be refactored when A/B testing is implemented
    pub async fn validate_quality_gates(
        &self,
        _experiment_id: Uuid,
        _recall: f64,
        _precision: f64,
        _ndcg: f64,
        _latency_increase_ms: i32,
        _error_rate_increase: f64,
        _p_value: f64,
    ) -> anyhow::Result<bool> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_config_validation() {
        let mut config = ExperimentConfig::new("test".to_string(), 50);
        assert!(config.validate().is_ok());

        config.rollout_percentage = 150;
        assert!(config.validate().is_err());

        config.rollout_percentage = -10;
        assert!(config.validate().is_err());

        config.rollout_percentage = 50;
        config.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_experiment_is_active() {
        let mut config = ExperimentConfig::new("test".to_string(), 50);
        assert!(config.is_active());

        config.status = ExperimentStatus::Paused;
        assert!(!config.is_active());

        config.status = ExperimentStatus::Running;
        config.start_date = Utc::now() + chrono::Duration::hours(1);
        assert!(!config.is_active());
    }

    #[test]
    fn test_traffic_splitter_deterministic() {
        let splitter = TrafficSplitter::new();
        let config = ExperimentConfig::new("test".to_string(), 50);

        let result1 =
            splitter.should_use_new_implementation(&config, Some("user123"), "test query");
        let result2 =
            splitter.should_use_new_implementation(&config, Some("user123"), "test query");

        assert_eq!(result1, result2, "Same user/query should get same result");
    }

    #[test]
    fn test_traffic_splitter_rollout_percentage() {
        let splitter = TrafficSplitter::new();
        let mut config = ExperimentConfig::new("test".to_string(), 0);

        assert!(!splitter.should_use_new_implementation(&config, Some("user1"), "query1"));

        config.rollout_percentage = 100;
        assert!(splitter.should_use_new_implementation(&config, Some("user1"), "query1"));
    }

    #[test]
    fn test_quality_gates_default() {
        let gates = QualityGates::default();
        assert_eq!(gates.min_recall, 0.80);
        assert_eq!(gates.min_precision, 0.70);
        assert_eq!(gates.min_ndcg, 0.75);
        assert_eq!(gates.max_latency_increase_ms, 10);
        assert_eq!(gates.significance_threshold, 0.05);
    }
}
