//! Event logging infrastructure for A/B testing
//!
//! Provides efficient batch logging of shadow results and user interaction events
//! with structured metadata and PostgreSQL persistence.

use crate::ab_testing::shadow_mode::{SearchResult, ShadowModeResults};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// User interaction event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionEventType {
    /// User clicked on a result
    Click,
    /// User spent time viewing a result
    Dwell,
    /// User selected/opened a result
    Selection,
    /// User abandoned the search without interaction
    Abandon,
    /// User reformulated the query
    Reformulation,
}

impl std::fmt::Display for InteractionEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionEventType::Click => write!(f, "click"),
            InteractionEventType::Dwell => write!(f, "dwell"),
            InteractionEventType::Selection => write!(f, "selection"),
            InteractionEventType::Abandon => write!(f, "abandon"),
            InteractionEventType::Reformulation => write!(f, "reformulation"),
        }
    }
}

/// User interaction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionEvent {
    /// Unique event ID
    pub id: Uuid,
    /// Associated experiment ID
    pub experiment_id: Uuid,
    /// Search query
    pub query: String,
    /// Type of interaction
    pub event_type: InteractionEventType,
    /// Position of result in list (1-indexed, None for abandon/reformulation)
    pub result_position: Option<i32>,
    /// Time spent on result in milliseconds (for dwell events)
    pub dwell_time_ms: Option<i32>,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
    /// User ID (if available)
    pub user_id: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl InteractionEvent {
    /// Create a new interaction event
    pub fn new(
        experiment_id: Uuid,
        query: String,
        event_type: InteractionEventType,
        user_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            experiment_id,
            query,
            event_type,
            result_position: None,
            dwell_time_ms: None,
            timestamp: Utc::now(),
            user_id,
            metadata: None,
        }
    }

    /// Create a click event
    pub fn click(
        experiment_id: Uuid,
        query: String,
        position: i32,
        user_id: Option<String>,
    ) -> Self {
        let mut event = Self::new(experiment_id, query, InteractionEventType::Click, user_id);
        event.result_position = Some(position);
        event
    }

    /// Create a dwell event
    pub fn dwell(
        experiment_id: Uuid,
        query: String,
        position: i32,
        dwell_time_ms: i32,
        user_id: Option<String>,
    ) -> Self {
        let mut event = Self::new(experiment_id, query, InteractionEventType::Dwell, user_id);
        event.result_position = Some(position);
        event.dwell_time_ms = Some(dwell_time_ms);
        event
    }

    /// Create a selection event
    pub fn selection(
        experiment_id: Uuid,
        query: String,
        position: i32,
        user_id: Option<String>,
    ) -> Self {
        let mut event = Self::new(
            experiment_id,
            query,
            InteractionEventType::Selection,
            user_id,
        );
        event.result_position = Some(position);
        event
    }

    /// Create an abandon event
    pub fn abandon(experiment_id: Uuid, query: String, user_id: Option<String>) -> Self {
        Self::new(experiment_id, query, InteractionEventType::Abandon, user_id)
    }

    /// Create a reformulation event
    pub fn reformulation(experiment_id: Uuid, old_query: String, user_id: Option<String>) -> Self {
        Self::new(
            experiment_id,
            old_query,
            InteractionEventType::Reformulation,
            user_id,
        )
    }
}

/// Shadow result log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowResultLog {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub query: String,
    pub old_results: Vec<SearchResult>,
    pub new_results: Option<Vec<SearchResult>>,
    pub old_latency_ms: i32,
    pub new_latency_ms: Option<i32>,
    pub new_error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
}

impl From<(Uuid, &ShadowModeResults)> for ShadowResultLog {
    fn from((experiment_id, results): (Uuid, &ShadowModeResults)) -> Self {
        Self {
            id: Uuid::new_v4(),
            experiment_id,
            query: results.query.clone(),
            old_results: results.old_results.clone(),
            new_results: results.new_results.clone(),
            old_latency_ms: results.old_latency_ms as i32,
            new_latency_ms: results.new_latency_ms.map(|v| v as i32),
            new_error: results.new_error.clone(),
            timestamp: results.timestamp,
            user_id: results.user_id.clone(),
        }
    }
}

/// Batch logger for A/B testing events
pub struct ABTestLogger {
    // Stubbed: A/B testing not yet implemented for SQLite
    _phantom: std::marker::PhantomData<()>,
}

impl Default for ABTestLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl ABTestLogger {
    /// Create a new logger with default settings (batch_size=100, flush every 10s)
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create with custom batch size and flush interval
    pub fn with_config(_batch_size: usize, _flush_interval_secs: u64) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Log shadow mode results
    pub async fn log_shadow_results(
        &self,
        _experiment_id: Uuid,
        _results: &ShadowModeResults,
    ) -> anyhow::Result<()> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Log user interaction event
    pub async fn log_interaction(&self, _event: InteractionEvent) -> anyhow::Result<()> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Manually flush all buffers
    pub async fn flush_all(&self) -> anyhow::Result<()> {
        anyhow::bail!("A/B testing not implemented for SQLite backend")
    }

    /// Start background flusher task that periodically writes buffers to database
    pub fn start_background_flusher(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // Stub: do nothing
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_event_creation() {
        let experiment_id = Uuid::new_v4();

        let click = InteractionEvent::click(
            experiment_id,
            "test query".to_string(),
            3,
            Some("user123".to_string()),
        );
        assert_eq!(click.event_type, InteractionEventType::Click);
        assert_eq!(click.result_position, Some(3));
        assert_eq!(click.query, "test query");

        let dwell = InteractionEvent::dwell(experiment_id, "test query".to_string(), 1, 5000, None);
        assert_eq!(dwell.event_type, InteractionEventType::Dwell);
        assert_eq!(dwell.result_position, Some(1));
        assert_eq!(dwell.dwell_time_ms, Some(5000));

        let abandon = InteractionEvent::abandon(experiment_id, "test query".to_string(), None);
        assert_eq!(abandon.event_type, InteractionEventType::Abandon);
        assert_eq!(abandon.result_position, None);
    }

    #[test]
    fn test_shadow_result_log_conversion() {
        let experiment_id = Uuid::new_v4();
        let results = ShadowModeResults {
            query: "test".to_string(),
            user_id: Some("user1".to_string()),
            old_results: vec![],
            new_results: Some(vec![]),
            old_latency_ms: 100,
            new_latency_ms: Some(120),
            new_error: None,
            timestamp: Utc::now(),
        };

        let log = ShadowResultLog::from((experiment_id, &results));
        assert_eq!(log.experiment_id, experiment_id);
        assert_eq!(log.query, "test");
        assert_eq!(log.old_latency_ms, 100);
        assert_eq!(log.new_latency_ms, Some(120));
    }
}
