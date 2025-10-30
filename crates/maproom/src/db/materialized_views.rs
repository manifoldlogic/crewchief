//! Materialized view management for Maproom.
//!
//! This module provides functions to refresh and manage materialized views
//! created by migration 0013_query_tuning.sql. These views pre-compute
//! expensive joins and aggregations to improve search query performance.
//!
//! # Materialized Views
//!
//! - `chunk_importance`: Pre-computed importance scores from graph edges
//! - `chunk_search_view`: Denormalized search view (chunks + files + worktrees)
//! - `file_metadata_view`: File metadata with pre-computed aggregations
//! - `chunk_edge_counts`: Edge count aggregations by chunk and type
//!
//! # Refresh Strategy
//!
//! Views should be refreshed:
//! - After bulk indexing operations
//! - After embedding updates
//! - After edge creation/deletion
//! - Periodically (hourly/daily based on update frequency)
//!
//! All refreshes use CONCURRENTLY to avoid blocking reads.

use anyhow::{Context, Result};
use tokio_postgres::Client;
use tracing::{debug, info, warn};

/// Statistics about a materialized view refresh operation.
#[derive(Debug, Clone)]
pub struct RefreshStats {
    /// Name of the materialized view
    pub view_name: String,
    /// Time taken to refresh the view
    pub refresh_time_ms: u64,
    /// Whether the refresh was successful
    pub success: bool,
    /// Error message if refresh failed
    pub error: Option<String>,
}

/// Refresh all materialized views concurrently.
///
/// This function calls the PostgreSQL function `maproom.refresh_all_views()`
/// which refreshes all views in the optimal order (respecting dependencies).
///
/// # Returns
///
/// A vector of refresh statistics for each view.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::db::{connect, materialized_views};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = connect().await?;
///     let stats = materialized_views::refresh_all(&client).await?;
///
///     for stat in stats {
///         println!("{}: {}ms", stat.view_name, stat.refresh_time_ms);
///     }
///     Ok(())
/// }
/// ```
pub async fn refresh_all(client: &Client) -> Result<Vec<RefreshStats>> {
    info!("Refreshing all materialized views concurrently");

    let rows = client
        .query(
            "SELECT view_name, refresh_time FROM maproom.refresh_all_views()",
            &[],
        )
        .await
        .context("Failed to refresh all views")?;

    let stats: Vec<RefreshStats> = rows
        .iter()
        .map(|row| {
            let view_name: String = row.get(0);
            // PostgreSQL returns interval as a string, parse it
            let interval_str: Option<String> = row.try_get(1).ok();
            let refresh_time_ms = interval_str
                .and_then(|s| parse_postgres_interval(&s))
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            RefreshStats {
                view_name: view_name.clone(),
                refresh_time_ms,
                success: true,
                error: None,
            }
        })
        .collect();

    let total_time: u64 = stats.iter().map(|s| s.refresh_time_ms).sum();
    info!(
        "Refreshed {} views in {}ms total",
        stats.len(),
        total_time
    );

    for stat in &stats {
        debug!(
            "  - {}: {}ms",
            stat.view_name, stat.refresh_time_ms
        );
    }

    Ok(stats)
}

/// Refresh a specific materialized view by name.
///
/// # Arguments
///
/// * `client` - Database client
/// * `view_name` - Name of the view to refresh (without schema prefix)
///
/// # Returns
///
/// Refresh statistics for the view.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::db::{connect, materialized_views};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = connect().await?;
///
///     // Refresh only the search view after embedding updates
///     let stat = materialized_views::refresh_one(&client, "chunk_search_view").await?;
///     println!("Refreshed {} in {}ms", stat.view_name, stat.refresh_time_ms);
///
///     Ok(())
/// }
/// ```
pub async fn refresh_one(client: &Client, view_name: &str) -> Result<RefreshStats> {
    let start = std::time::Instant::now();
    info!("Refreshing materialized view: {}", view_name);

    let sql = format!(
        "REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.{}",
        view_name
    );

    let result = client.execute(&sql, &[]).await;

    let elapsed = start.elapsed();
    let refresh_time_ms = elapsed.as_millis() as u64;

    match result {
        Ok(_) => {
            info!(
                "Refreshed {} in {}ms",
                view_name, refresh_time_ms
            );

            // Update statistics after refresh
            let analyze_sql = format!("ANALYZE maproom.{}", view_name);
            if let Err(e) = client.execute(&analyze_sql, &[]).await {
                warn!("Failed to analyze {}: {}", view_name, e);
            }

            Ok(RefreshStats {
                view_name: view_name.to_string(),
                refresh_time_ms,
                success: true,
                error: None,
            })
        }
        Err(e) => {
            warn!(
                "Failed to refresh {} after {}ms: {}",
                view_name, refresh_time_ms, e
            );
            Ok(RefreshStats {
                view_name: view_name.to_string(),
                refresh_time_ms,
                success: false,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Refresh views that depend on chunk data.
///
/// This refreshes:
/// - chunk_importance (depends on chunks + chunk_edges)
/// - chunk_search_view (depends on chunk_importance)
/// - chunk_edge_counts (depends on chunk_edges)
///
/// Call this after bulk chunk indexing or edge creation.
pub async fn refresh_chunk_views(client: &Client) -> Result<Vec<RefreshStats>> {
    info!("Refreshing chunk-related materialized views");
    let mut stats = Vec::new();

    // Refresh in dependency order
    stats.push(refresh_one(client, "chunk_importance").await?);
    stats.push(refresh_one(client, "chunk_edge_counts").await?);
    stats.push(refresh_one(client, "chunk_search_view").await?);

    let total_time: u64 = stats.iter().map(|s| s.refresh_time_ms).sum();
    info!(
        "Refreshed {} chunk views in {}ms total",
        stats.len(),
        total_time
    );

    Ok(stats)
}

/// Refresh views that depend on file data.
///
/// This refreshes:
/// - file_metadata_view (depends on files + chunks)
/// - chunk_search_view (depends on files)
///
/// Call this after file indexing or deletion.
pub async fn refresh_file_views(client: &Client) -> Result<Vec<RefreshStats>> {
    info!("Refreshing file-related materialized views");
    let mut stats = Vec::new();

    stats.push(refresh_one(client, "file_metadata_view").await?);
    stats.push(refresh_one(client, "chunk_search_view").await?);

    let total_time: u64 = stats.iter().map(|s| s.refresh_time_ms).sum();
    info!(
        "Refreshed {} file views in {}ms total",
        stats.len(),
        total_time
    );

    Ok(stats)
}

/// Refresh only the search view (fastest option).
///
/// Call this after embedding updates when you need to update the search
/// index quickly without refreshing all views.
pub async fn refresh_search_view(client: &Client) -> Result<RefreshStats> {
    refresh_one(client, "chunk_search_view").await
}

/// Information about view staleness.
#[derive(Debug, Clone)]
pub struct ViewStaleness {
    /// Name of the materialized view
    pub view_name: String,
    /// Last refresh timestamp
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    /// Age since last refresh
    pub age: Option<std::time::Duration>,
    /// Whether the view is considered stale (> 1 hour old)
    pub is_stale: bool,
}

/// Check staleness of all materialized views.
///
/// # Returns
///
/// Vector of staleness information for each view, ordered by age (oldest first).
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::db::{connect, materialized_views};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = connect().await?;
///     let staleness = materialized_views::check_staleness(&client).await?;
///
///     for view in staleness {
///         if view.is_stale {
///             println!("⚠️  {} is stale (age: {:?})", view.view_name, view.age);
///         }
///     }
///     Ok(())
/// }
/// ```
pub async fn check_staleness(client: &Client) -> Result<Vec<ViewStaleness>> {
    debug!("Checking materialized view staleness");

    let rows = client
        .query("SELECT * FROM maproom.view_staleness()", &[])
        .await
        .context("Failed to check view staleness")?;

    let views: Vec<ViewStaleness> = rows
        .iter()
        .map(|row| {
            let view_name: String = row.get(0);
            let last_refresh: Option<chrono::DateTime<chrono::Utc>> = row.get(1);
            let age_interval: Option<String> = row.try_get(2).ok();
            let is_stale: bool = row.get(3);

            // Parse PostgreSQL interval to Duration (rough approximation)
            let age = age_interval.and_then(|interval_str| {
                // Simple parsing for common interval formats
                // PostgreSQL returns intervals like "01:30:15" or "2 days 01:30:15"
                parse_postgres_interval(&interval_str)
            });

            ViewStaleness {
                view_name,
                last_refresh,
                age,
                is_stale,
            }
        })
        .collect();

    Ok(views)
}

/// Parse a PostgreSQL interval string to a Duration.
///
/// This is a simple parser for common interval formats. PostgreSQL intervals
/// can be complex, but we only need basic support for monitoring.
fn parse_postgres_interval(interval: &str) -> Option<std::time::Duration> {
    // Remove leading/trailing whitespace
    let interval = interval.trim();

    // Handle simple time formats: "HH:MM:SS" or "HH:MM:SS.microseconds"
    if let Some((hours, rest)) = interval.split_once(':') {
        if let (Ok(h), Some((minutes, seconds))) =
            (hours.parse::<u64>(), rest.split_once(':'))
        {
            if let (Ok(m), Ok(s)) = (minutes.parse::<u64>(), seconds.parse::<f64>()) {
                let total_secs = h * 3600 + m * 60 + s as u64;
                return Some(std::time::Duration::from_secs(total_secs));
            }
        }
    }

    // Handle formats with days: "X days HH:MM:SS"
    if let Some((days_part, time_part)) = interval.split_once("days") {
        if let Ok(days) = days_part.trim().parse::<u64>() {
            let time_part = time_part.trim();
            if let Some(time_duration) = parse_postgres_interval(time_part) {
                let total_duration = std::time::Duration::from_secs(days * 86400)
                    + time_duration;
                return Some(total_duration);
            }
        }
    }

    // If we can't parse it, return None
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_postgres_interval() {
        // Simple time format
        assert_eq!(
            parse_postgres_interval("01:30:15"),
            Some(std::time::Duration::from_secs(5415))
        );

        // With days
        assert_eq!(
            parse_postgres_interval("2 days 01:30:15"),
            Some(std::time::Duration::from_secs(178215))
        );

        // Invalid format
        assert_eq!(parse_postgres_interval("invalid"), None);
    }
}
