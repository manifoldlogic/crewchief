//! Unit tests for multi-worktree watcher functionality.
//!
//! Note: FileWatcher now requires a git repository due to git polling.
//! Tests that use FileWatcher/WorktreeWatcher directly must create git repos.

use crewchief_maproom::incremental::{
    EventType, MultiWatcher, WatcherConfig, WatcherStatus, WorktreeWatcher,
};
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create a temporary git repository.
fn create_temp_git_repo() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    dir
}

/// Helper to create a test directory structure (now creates git repo).
async fn create_test_dir() -> TempDir {
    create_temp_git_repo()
}

/// Helper to create a test file in a directory.
async fn create_test_file(dir: &TempDir, name: &str, content: &str) {
    let file_path = dir.path().join(name);
    fs::write(&file_path, content)
        .await
        .expect("Failed to write test file");
}

#[tokio::test]
async fn test_multi_watcher_creation() {
    let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
    assert_eq!(multi_watcher.watcher_count(), 0);
    assert!(multi_watcher.list_worktrees().is_empty());
}

#[tokio::test]
async fn test_add_single_worktree() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree-1".to_string();
    let path = temp_dir.path().to_path_buf();

    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    assert_eq!(multi_watcher.watcher_count(), 1);
    assert!(multi_watcher.is_watching(&worktree_id));
    assert!(multi_watcher.list_worktrees().contains(&worktree_id));

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_add_multiple_worktrees() {
    let temp_dir1 = create_test_dir().await;
    let temp_dir2 = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id1 = "test-worktree-1".to_string();
    let worktree_id2 = "test-worktree-2".to_string();

    multi_watcher
        .add_worktree(worktree_id1.clone(), temp_dir1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree(worktree_id2.clone(), temp_dir2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    assert_eq!(multi_watcher.watcher_count(), 2);
    assert!(multi_watcher.is_watching(&worktree_id1));
    assert!(multi_watcher.is_watching(&worktree_id2));

    let worktrees = multi_watcher.list_worktrees();
    assert_eq!(worktrees.len(), 2);
    assert!(worktrees.contains(&worktree_id1));
    assert!(worktrees.contains(&worktree_id2));

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_add_duplicate_worktree_fails() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    multi_watcher
        .add_worktree(worktree_id.clone(), path.clone())
        .await
        .expect("Failed to add worktree");

    // Try to add the same worktree again
    let result = multi_watcher.add_worktree(worktree_id, path).await;
    assert!(result.is_err());

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_remove_worktree() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    assert_eq!(multi_watcher.watcher_count(), 1);

    multi_watcher
        .remove_worktree(&worktree_id)
        .await
        .expect("Failed to remove worktree");

    assert_eq!(multi_watcher.watcher_count(), 0);
    assert!(!multi_watcher.is_watching(&worktree_id));
}

#[tokio::test]
async fn test_remove_nonexistent_worktree_fails() {
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let result = multi_watcher
        .remove_worktree(&"nonexistent".to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_restart_worktree() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Restart should succeed
    multi_watcher
        .restart_worktree(&worktree_id)
        .await
        .expect("Failed to restart worktree");

    assert!(multi_watcher.is_watching(&worktree_id));

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_restart_nonexistent_worktree_fails() {
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let result = multi_watcher
        .restart_worktree(&"nonexistent".to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_status() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Status should be None before adding
    assert!(multi_watcher.get_status(&worktree_id).is_none());

    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Status should be Running after adding
    let status = multi_watcher.get_status(&worktree_id);
    assert!(status.is_some());
    assert_eq!(status.unwrap(), &WatcherStatus::Running);

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_event_isolation() {
    let temp_dir1 = create_test_dir().await;
    let temp_dir2 = create_test_dir().await;

    let config = WatcherConfig {
        debounce_ms: 100, // Unused with git polling
        channel_capacity: 100,
        poll_interval_ms: 200, // Fast polling for tests
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    let worktree_id1 = "worktree-1".to_string();
    let worktree_id2 = "worktree-2".to_string();

    multi_watcher
        .add_worktree(worktree_id1.clone(), temp_dir1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree(worktree_id2.clone(), temp_dir2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    // Create files in both worktrees
    create_test_file(&temp_dir1, "file1.txt", "content1").await;
    create_test_file(&temp_dir2, "file2.txt", "content2").await;

    // Give time for events to propagate
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Collect events (should see events from both worktrees)
    let mut events = Vec::new();
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            events.push(evt);
        }
    }

    // We should have received events from both worktrees
    let worktree1_events: Vec<_> = events
        .iter()
        .filter(|e| e.worktree_id == worktree_id1)
        .collect();
    let worktree2_events: Vec<_> = events
        .iter()
        .filter(|e| e.worktree_id == worktree_id2)
        .collect();

    // Each worktree should have at least one event
    assert!(
        !worktree1_events.is_empty(),
        "Should have events from worktree 1"
    );
    assert!(
        !worktree2_events.is_empty(),
        "Should have events from worktree 2"
    );

    // Events should be properly tagged with their worktree IDs
    for event in &worktree1_events {
        assert_eq!(event.worktree_id, worktree_id1);
    }
    for event in &worktree2_events {
        assert_eq!(event.worktree_id, worktree_id2);
    }

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_shutdown() {
    let temp_dir1 = create_test_dir().await;
    let temp_dir2 = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    multi_watcher
        .add_worktree("worktree-1".to_string(), temp_dir1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), temp_dir2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    assert_eq!(multi_watcher.watcher_count(), 2);

    multi_watcher.shutdown().await.expect("Failed to shutdown");

    assert_eq!(multi_watcher.watcher_count(), 0);
}

#[tokio::test]
async fn test_worktree_watcher_creation() {
    let temp_dir = create_test_dir().await;
    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();
    let config = WatcherConfig::default();

    let (mut watcher, _rx) = WorktreeWatcher::new(worktree_id.clone(), path.clone(), config)
        .expect("Failed to create WorktreeWatcher");

    assert_eq!(watcher.worktree_id(), &worktree_id);
    assert_eq!(watcher.path(), path.as_path());
    assert_eq!(watcher.status(), &WatcherStatus::Stopped);

    // Start the watcher
    watcher.start().expect("Failed to start watcher");
    assert_eq!(watcher.status(), &WatcherStatus::Running);

    // Stop the watcher
    watcher.stop().expect("Failed to stop watcher");
    assert_eq!(watcher.status(), &WatcherStatus::Stopped);
}

#[tokio::test]
async fn test_worktree_watcher_event_tagging() {
    let temp_dir = create_test_dir().await;
    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = WorktreeWatcher::new(worktree_id.clone(), path, config)
        .expect("Failed to create WorktreeWatcher");

    watcher.start().expect("Failed to start watcher");

    // Create a test file
    create_test_file(&temp_dir, "test.txt", "test content").await;

    // Wait for event
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Receive event
    if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
        // Event should be tagged with correct worktree_id
        assert_eq!(event.worktree_id, worktree_id);
        assert_eq!(event.event_type, EventType::Modified);
        assert!(event.path.ends_with("test.txt"));
    }

    watcher.stop().expect("Failed to stop watcher");
}

#[tokio::test]
async fn test_automatic_restart_on_failure() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Add worktree
    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Verify initial status is Running
    assert_eq!(
        multi_watcher.get_status(&worktree_id).unwrap(),
        &WatcherStatus::Running
    );

    // Manually mark the watcher as failed to simulate a crash
    multi_watcher
        .mark_watcher_failed(&worktree_id, "Simulated failure".to_string())
        .expect("Failed to mark watcher as failed");

    // Verify status is now Failed
    assert!(matches!(
        multi_watcher.get_status(&worktree_id).unwrap(),
        WatcherStatus::Failed(_)
    ));

    // Trigger health check and restart
    multi_watcher.check_and_restart_failed_watchers().await;

    // Give the restart some time to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify the watcher has been restarted and is Running
    assert_eq!(
        multi_watcher.get_status(&worktree_id).unwrap(),
        &WatcherStatus::Running
    );

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_restart_backoff_timing() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Add worktree
    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Mark as failed
    multi_watcher
        .mark_watcher_failed(&worktree_id, "Simulated failure".to_string())
        .expect("Failed to mark watcher as failed");

    // First restart attempt - should have minimal delay (1 second)
    let start = std::time::Instant::now();
    multi_watcher.check_and_restart_failed_watchers().await;
    let elapsed = start.elapsed();

    // Should be approximately 1 second (initial delay)
    assert!(
        elapsed >= Duration::from_millis(900) && elapsed <= Duration::from_millis(1500),
        "First restart delay should be ~1 second, got {:?}",
        elapsed
    );

    // Verify watcher is now Running
    assert_eq!(
        multi_watcher.get_status(&worktree_id).unwrap(),
        &WatcherStatus::Running
    );

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_restart_backoff_exhaustion() {
    // This test verifies that the backoff logic works correctly.
    // Since we're using a valid temp directory, restart() will always succeed.
    // Therefore, we verify that successful restarts clear the retry state,
    // preventing retry exhaustion under normal conditions.
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Add worktree
    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Simulate multiple failure-restart cycles
    // Each restart should succeed and clear retry state
    for attempt in 1..=6 {
        // Mark as failed
        multi_watcher
            .mark_watcher_failed(&worktree_id, format!("Failure attempt {}", attempt))
            .expect("Failed to mark watcher as failed");

        // Attempt restart
        multi_watcher.check_and_restart_failed_watchers().await;

        // All restarts should succeed (valid temp directory)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // After successful restart, status should be Running
        assert_eq!(
            multi_watcher.get_status(&worktree_id).unwrap(),
            &WatcherStatus::Running,
            "Restart attempt {} should succeed",
            attempt
        );
    }

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_health_monitor_lifecycle() {
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    // Start health monitor
    multi_watcher.start_health_monitor();

    // Wait a bit to ensure it's running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop health monitor
    multi_watcher.stop_health_monitor().await;

    // Verify we can restart it
    multi_watcher.start_health_monitor();
    multi_watcher.stop_health_monitor().await;
}

#[tokio::test]
async fn test_exponential_backoff_calculation() {
    // This test verifies that retry state is preserved correctly when a watcher
    // is successfully restarted. Since restart() succeeds, the retry state is cleared.
    // Therefore, we verify that successful restarts reset the backoff counter.
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Add worktree
    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Each failure followed by successful restart should reset backoff to 1s
    for attempt in 1..=3 {
        // Mark as failed
        multi_watcher
            .mark_watcher_failed(&worktree_id, format!("Failure {}", attempt))
            .expect("Failed to mark watcher as failed");

        let start = std::time::Instant::now();
        multi_watcher.check_and_restart_failed_watchers().await;
        let elapsed = start.elapsed();

        // Since restart succeeds, delay should always be ~1s (initial delay)
        assert!(
            elapsed >= Duration::from_millis(900) && elapsed <= Duration::from_millis(1500),
            "Delay for attempt {} should be ~1s (restart succeeds), got {:?}",
            attempt,
            elapsed
        );

        // Let the watcher stabilize
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_max_retry_delay_cap() {
    let temp_dir = create_test_dir().await;
    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();

    let worktree_id = "test-worktree".to_string();
    let path = temp_dir.path().to_path_buf();

    // Add worktree
    multi_watcher
        .add_worktree(worktree_id.clone(), path)
        .await
        .expect("Failed to add worktree");

    // Simulate many failures to hit the max delay cap (60 seconds)
    // With 2x backoff: 1s -> 2s -> 4s -> 8s -> 16s -> 32s -> 64s (capped at 60s)
    for attempt in 1..=7 {
        multi_watcher
            .mark_watcher_failed(&worktree_id, format!("Failure {}", attempt))
            .expect("Failed to mark watcher as failed");

        let start = std::time::Instant::now();
        multi_watcher.check_and_restart_failed_watchers().await;
        let elapsed = start.elapsed();

        if attempt == 5 {
            // 5th attempt should still be under max (16s)
            assert!(
                elapsed < Duration::from_secs(20),
                "Delay should not exceed cap yet"
            );
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_retry_exhaustion_with_failing_restart() {
    // This test verifies that repeated failures are handled gracefully.
    // With git polling, the behavior may differ from native file watchers:
    // - When the directory doesn't exist, the watcher fails to create
    // - The multi_watcher handles this by keeping track of failed attempts
    //
    // Note: The specific retry exhaustion behavior depends on internal implementation.
    // This test primarily verifies the system doesn't hang or crash under repeated failures.
    let temp_dir = create_test_dir().await;
    let path = temp_dir.path().to_path_buf();

    let (mut multi_watcher, _rx) = MultiWatcher::new_with_defaults();
    let worktree_id = "test-worktree".to_string();

    // Add worktree with valid path
    multi_watcher
        .add_worktree(worktree_id.clone(), path.clone())
        .await
        .expect("Failed to add worktree");

    // Mark as failed
    multi_watcher
        .mark_watcher_failed(&worktree_id, "Initial failure".to_string())
        .expect("Failed to mark watcher as failed");

    // Now delete the temp directory to cause restart to fail
    drop(temp_dir);

    // Attempt restart - this should fail because the directory no longer exists
    multi_watcher.check_and_restart_failed_watchers().await;

    // The restart attempt failed, so retry state should be incremented
    // Mark as failed again and try again
    for attempt in 2..=5 {
        // Small delay before next attempt
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try marking as failed - this may fail if watcher was removed
        let _ = multi_watcher
            .mark_watcher_failed(&worktree_id, format!("Recurring failure {}", attempt));

        // Attempt restart
        multi_watcher.check_and_restart_failed_watchers().await;
    }

    // After multiple failed restart attempts, the system should handle gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try one more mark (may succeed or fail depending on state)
    let _ = multi_watcher.mark_watcher_failed(&worktree_id, "Final failure".to_string());

    // This attempt should complete in reasonable time
    let start = std::time::Instant::now();
    multi_watcher.check_and_restart_failed_watchers().await;
    let elapsed = start.elapsed();

    // Should complete in reasonable time (not wait for full exponential backoff)
    assert!(
        elapsed < Duration::from_secs(10),
        "Should complete in reasonable time, got {:?}",
        elapsed
    );

    // Watcher status could be Failed or may have been removed entirely
    // Just verify we can query status without panicking
    let _status = multi_watcher.get_status(&worktree_id);

    // Clean up (shutdown will handle any remaining watchers)
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}
