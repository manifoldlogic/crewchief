//! Integration tests for A/B testing framework
//!
//! These tests verify the complete A/B testing workflow including:
//! - Experiment lifecycle management
//! - Shadow mode execution
//! - Event logging
//! - Statistical analysis
//! - Quality gates validation
//!
//! Note: These tests require a PostgreSQL database and are marked with #[ignore]
//! to avoid running in CI without database setup.

use chrono::Utc;
use crewchief_maproom::ab_testing::*;
use std::time::Duration;
use tokio_postgres::NoTls;
use uuid::Uuid;

/// Test database configuration
struct TestDb {
    pool: deadpool_postgres::Pool,
}

impl TestDb {
    async fn new() -> anyhow::Result<Self> {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/maproom_test".to_string());

        let config: tokio_postgres::Config = db_url.parse()?;
        let manager = deadpool_postgres::Manager::new(config, NoTls);
        let pool = deadpool_postgres::Pool::builder(manager).build()?;

        Ok(Self { pool })
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        let client = self.pool.get().await?;

        // Clean up test data
        client
            .execute("DELETE FROM interaction_events WHERE 1=1", &[])
            .await?;
        client
            .execute("DELETE FROM shadow_results WHERE 1=1", &[])
            .await?;
        client
            .execute("DELETE FROM experiments WHERE 1=1", &[])
            .await?;

        Ok(())
    }
}

#[tokio::test]
#[ignore] // Requires database
async fn test_experiment_lifecycle() -> anyhow::Result<()> {
    let test_db = TestDb::new().await?;
    test_db.cleanup().await?;

    let manager = ExperimentManager::new(test_db.pool.clone());

    // Create experiment
    let config = ExperimentConfig::new("test-experiment".to_string(), 25);
    let experiment_id = manager.create_experiment(config.clone()).await?;

    // Retrieve experiment
    let retrieved = manager.get_experiment(experiment_id).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "test-experiment");
    assert_eq!(retrieved.rollout_percentage, 25);
    assert_eq!(retrieved.status, ExperimentStatus::Running);

    // Update status
    manager.pause_experiment(experiment_id).await?;
    let paused = manager.get_experiment(experiment_id).await?.unwrap();
    assert_eq!(paused.status, ExperimentStatus::Paused);

    manager.resume_experiment(experiment_id).await?;
    let resumed = manager.get_experiment(experiment_id).await?.unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);

    // Update rollout percentage
    manager.update_rollout(experiment_id, 50).await?;
    let updated = manager.get_experiment(experiment_id).await?.unwrap();
    assert_eq!(updated.rollout_percentage, 50);

    // Complete experiment
    manager.complete_experiment(experiment_id).await?;
    let completed = manager.get_experiment(experiment_id).await?.unwrap();
    assert_eq!(completed.status, ExperimentStatus::Completed);

    test_db.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_shadow_mode_execution() -> anyhow::Result<()> {
    let shadow = ShadowMode::new();

    async fn old_search(query: String) -> anyhow::Result<Vec<SearchResult>> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(vec![
            SearchResult {
                relpath: "src/main.rs".to_string(),
                symbol_name: "main".to_string(),
                score: 0.9,
                rank: 1,
            },
            SearchResult {
                relpath: "src/lib.rs".to_string(),
                symbol_name: "init".to_string(),
                score: 0.7,
                rank: 2,
            },
        ])
    }

    async fn new_search(query: String) -> anyhow::Result<Vec<SearchResult>> {
        tokio::time::sleep(Duration::from_millis(60)).await;
        Ok(vec![
            SearchResult {
                relpath: "src/lib.rs".to_string(),
                symbol_name: "init".to_string(),
                score: 0.95,
                rank: 1,
            },
            SearchResult {
                relpath: "src/main.rs".to_string(),
                symbol_name: "main".to_string(),
                score: 0.85,
                rank: 2,
            },
            SearchResult {
                relpath: "src/utils.rs".to_string(),
                symbol_name: "helper".to_string(),
                score: 0.6,
                rank: 3,
            },
        ])
    }

    let results = shadow
        .execute(
            "test query".to_string(),
            Some("user123".to_string()),
            old_search,
            new_search,
        )
        .await?;

    // Verify results
    assert_eq!(results.query, "test query");
    assert_eq!(results.user_id, Some("user123".to_string()));
    assert_eq!(results.old_results.len(), 2);
    assert!(results.new_results.is_some());
    assert_eq!(results.new_results.as_ref().unwrap().len(), 3);
    assert!(results.old_latency_ms >= 50);
    assert!(results.new_latency_ms.unwrap() >= 60);
    assert!(results.new_error.is_none());

    // Compare results
    let comparison = shadow.compare_results(&results);
    assert_eq!(comparison.total_old, 2);
    assert_eq!(comparison.total_new, 3);
    assert_eq!(comparison.common_results, 2);
    assert_eq!(comparison.only_in_old, 0);
    assert_eq!(comparison.only_in_new, 1);
    assert!(comparison.latency_diff_ms.is_some());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_shadow_mode_with_timeout() -> anyhow::Result<()> {
    let shadow = ShadowMode::with_timeout(100); // 100ms timeout

    async fn old_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    async fn slow_new_search(_query: String) -> anyhow::Result<Vec<SearchResult>> {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(vec![])
    }

    let results = shadow
        .execute("test".to_string(), None, old_search, slow_new_search)
        .await?;

    // New implementation should timeout
    assert!(results.new_results.is_none());
    assert!(results.new_error.is_some());
    assert!(results.new_error.unwrap().contains("Timeout"));

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_event_logging() -> anyhow::Result<()> {
    let test_db = TestDb::new().await?;
    test_db.cleanup().await?;

    let manager = ExperimentManager::new(test_db.pool.clone());
    let logger = ABTestLogger::new(test_db.pool.clone());

    // Create experiment
    let config = ExperimentConfig::new("logging-test".to_string(), 50);
    let experiment_id = manager.create_experiment(config).await?;

    // Create mock shadow results
    let shadow_results = ShadowModeResults {
        query: "test query".to_string(),
        user_id: Some("user1".to_string()),
        old_results: vec![SearchResult {
            relpath: "src/main.rs".to_string(),
            symbol_name: "main".to_string(),
            score: 0.9,
            rank: 1,
        }],
        new_results: Some(vec![SearchResult {
            relpath: "src/lib.rs".to_string(),
            symbol_name: "init".to_string(),
            score: 0.95,
            rank: 1,
        }]),
        old_latency_ms: 100,
        new_latency_ms: Some(110),
        new_error: None,
        timestamp: Utc::now(),
    };

    // Log shadow results
    logger
        .log_shadow_results(experiment_id, &shadow_results)
        .await?;

    // Log interaction events
    let click_event = InteractionEvent::click(
        experiment_id,
        "test query".to_string(),
        1,
        Some("user1".to_string()),
    );
    logger.log_interaction(click_event).await?;

    let dwell_event = InteractionEvent::dwell(
        experiment_id,
        "test query".to_string(),
        1,
        5000,
        Some("user1".to_string()),
    );
    logger.log_interaction(dwell_event).await?;

    // Flush buffers
    logger.flush_all().await?;

    // Verify data was logged
    let client = test_db.pool.get().await?;

    let shadow_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM shadow_results WHERE experiment_id = $1",
            &[&experiment_id],
        )
        .await?
        .get(0);
    assert_eq!(shadow_count, 1);

    let event_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM interaction_events WHERE experiment_id = $1",
            &[&experiment_id],
        )
        .await?
        .get(0);
    assert_eq!(event_count, 2);

    test_db.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_traffic_splitting() -> anyhow::Result<()> {
    let test_db = TestDb::new().await?;
    test_db.cleanup().await?;

    let manager = ExperimentManager::new(test_db.pool.clone());
    let splitter = TrafficSplitter::new();

    // Test 0% rollout
    let config_0 = ExperimentConfig::new("test-0-percent".to_string(), 0);
    let id_0 = manager.create_experiment(config_0.clone()).await?;
    let exp_0 = manager.get_experiment(id_0).await?.unwrap();

    for i in 0..100 {
        let user_id = format!("user{}", i);
        assert!(!splitter.should_use_new_implementation(&exp_0, Some(&user_id), "query"));
    }

    // Test 100% rollout
    let config_100 = ExperimentConfig::new("test-100-percent".to_string(), 100);
    let id_100 = manager.create_experiment(config_100.clone()).await?;
    let exp_100 = manager.get_experiment(id_100).await?.unwrap();

    for i in 0..100 {
        let user_id = format!("user{}", i);
        assert!(splitter.should_use_new_implementation(&exp_100, Some(&user_id), "query"));
    }

    // Test 50% rollout - should split roughly 50/50
    let config_50 = ExperimentConfig::new("test-50-percent".to_string(), 50);
    let id_50 = manager.create_experiment(config_50.clone()).await?;
    let exp_50 = manager.get_experiment(id_50).await?.unwrap();

    let mut count_new = 0;
    for i in 0..1000 {
        let user_id = format!("user{}", i);
        if splitter.should_use_new_implementation(&exp_50, Some(&user_id), "query") {
            count_new += 1;
        }
    }

    // Should be close to 50% (allow some variance)
    assert!(
        count_new > 400 && count_new < 600,
        "Expected ~500, got {}",
        count_new
    );

    test_db.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_statistical_analysis() -> anyhow::Result<()> {
    let analyzer = StatisticalAnalyzer::new();

    // Test chi-square test
    let chi_result = analyzer.chi_square_test(100, 1000, 150, 1000)?;
    assert!(chi_result.statistic > 0.0);
    assert!(chi_result.p_value >= 0.0 && chi_result.p_value <= 1.0);

    // Test t-test
    let old_ndcg = vec![0.75, 0.76, 0.74, 0.77, 0.75, 0.76, 0.74, 0.75];
    let new_ndcg = vec![0.82, 0.83, 0.84, 0.82, 0.85, 0.83, 0.84, 0.82];
    let t_result = analyzer.t_test(&old_ndcg, &new_ndcg)?;
    assert!(t_result.statistic != 0.0);

    // Test confidence intervals
    let prop_ci = analyzer.proportion_confidence_interval(150, 1000, 0.95)?;
    assert_eq!(prop_ci.estimate, 0.15);
    assert!(prop_ci.lower_bound < 0.15);
    assert!(prop_ci.upper_bound > 0.15);

    let mean_ci = analyzer.mean_confidence_interval(&new_ndcg, 0.95)?;
    assert!(mean_ci.lower_bound < mean_ci.estimate);
    assert!(mean_ci.upper_bound > mean_ci.estimate);

    // Test sample size calculation
    let sample_size = analyzer.calculate_sample_size(0.10, 0.02, 0.80, 0.05)?;
    assert!(sample_size > 0);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_quality_gates_validation() -> anyhow::Result<()> {
    let test_db = TestDb::new().await?;
    test_db.cleanup().await?;

    let manager = ExperimentManager::new(test_db.pool.clone());

    // Create experiment with default quality gates
    let config = ExperimentConfig::new("quality-gates-test".to_string(), 25);
    let experiment_id = manager.create_experiment(config).await?;

    // Test passing quality gates
    let passes = manager
        .validate_quality_gates(
            experiment_id,
            0.85,  // recall (> 0.80 ✓)
            0.75,  // precision (> 0.70 ✓)
            0.80,  // ndcg (> 0.75 ✓)
            5,     // latency increase (< 10ms ✓)
            0.005, // error rate increase (< 0.01 ✓)
            0.03,  // p-value (< 0.05 ✓)
        )
        .await?;
    assert!(passes);

    // Test failing quality gates (low recall)
    let fails = manager
        .validate_quality_gates(
            experiment_id,
            0.75,  // recall (< 0.80 ✗)
            0.75,  // precision
            0.80,  // ndcg
            5,     // latency increase
            0.005, // error rate increase
            0.03,  // p-value
        )
        .await?;
    assert!(!fails);

    // Test failing quality gates (high p-value)
    let fails2 = manager
        .validate_quality_gates(
            experiment_id,
            0.85,  // recall
            0.75,  // precision
            0.80,  // ndcg
            5,     // latency increase
            0.005, // error rate increase
            0.10,  // p-value (> 0.05 ✗)
        )
        .await?;
    assert!(!fails2);

    test_db.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_list_active_experiments() -> anyhow::Result<()> {
    let test_db = TestDb::new().await?;
    test_db.cleanup().await?;

    let manager = ExperimentManager::new(test_db.pool.clone());

    // Create multiple experiments
    let config1 = ExperimentConfig::new("experiment-1".to_string(), 25);
    manager.create_experiment(config1).await?;

    let config2 = ExperimentConfig::new("experiment-2".to_string(), 50);
    let id2 = manager.create_experiment(config2).await?;

    let config3 = ExperimentConfig::new("experiment-3".to_string(), 75);
    manager.create_experiment(config3).await?;

    // Pause one experiment
    manager.pause_experiment(id2).await?;

    // List active experiments
    let active = manager.list_active_experiments().await?;
    assert_eq!(active.len(), 2); // Only 2 are running

    test_db.cleanup().await?;
    Ok(())
}
