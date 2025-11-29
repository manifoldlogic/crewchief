//! Integration tests for multi-worktree watcher concurrent scenarios.
//!
//! Note: FileWatcher now requires a git repository due to git polling.
//! All tests create git repositories for worktrees.

use crewchief_maproom::incremental::{EventType, MultiWatcher, WatcherConfig};
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create a test directory structure (now as git repo).
fn create_test_worktree_sync() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    temp_dir
}

/// Helper to create a test directory structure.
async fn create_test_worktree() -> TempDir {
    create_test_worktree_sync()
}

/// Helper to create a test file.
async fn create_file(dir: &TempDir, name: &str, content: &str) {
    let file_path = dir.path().join(name);
    fs::write(&file_path, content)
        .await
        .expect("Failed to write test file");
}

/// Helper to modify a test file.
async fn modify_file(dir: &TempDir, name: &str, content: &str) {
    let file_path = dir.path().join(name);
    fs::write(&file_path, content)
        .await
        .expect("Failed to modify test file");
}

/// Helper to delete a test file.
async fn delete_file(dir: &TempDir, name: &str) {
    let file_path = dir.path().join(name);
    fs::remove_file(&file_path)
        .await
        .expect("Failed to delete test file");
}

#[tokio::test]
async fn test_concurrent_file_modifications_across_worktrees() {
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;
    let worktree3 = create_test_worktree().await;

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 1000,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    // Add three worktrees
    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    multi_watcher
        .add_worktree("worktree-3".to_string(), worktree3.path().to_path_buf())
        .await
        .expect("Failed to add worktree 3");

    // Create files concurrently in all worktrees
    let file_ops = async {
        tokio::join!(
            create_file(&worktree1, "file1.txt", "content1"),
            create_file(&worktree2, "file2.txt", "content2"),
            create_file(&worktree3, "file3.txt", "content3"),
        )
    };

    file_ops.await;

    // Wait for events to propagate
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Collect events
    let mut events_by_worktree = HashMap::new();
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            events_by_worktree
                .entry(evt.worktree_id.clone())
                .or_insert_with(Vec::new)
                .push(evt);
        }
    }

    // Verify events from all three worktrees
    assert!(events_by_worktree.contains_key("worktree-1"));
    assert!(events_by_worktree.contains_key("worktree-2"));
    assert!(events_by_worktree.contains_key("worktree-3"));

    // Verify event isolation
    for (worktree_id, events) in &events_by_worktree {
        for event in events {
            assert_eq!(&event.worktree_id, worktree_id);
        }
    }

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_high_frequency_events_multiple_worktrees() {
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;

    let config = WatcherConfig {
        debounce_ms: 200,
        channel_capacity: 2000,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    // Create multiple files rapidly in both worktrees
    for i in 0..10 {
        create_file(&worktree1, &format!("file1_{}.txt", i), "content").await;
        create_file(&worktree2, &format!("file2_{}.txt", i), "content").await;
    }

    // Wait for events
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Collect events
    let mut event_count = 0;
    let mut worktree1_count = 0;
    let mut worktree2_count = 0;

    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            event_count += 1;
            match evt.worktree_id.as_str() {
                "worktree-1" => worktree1_count += 1,
                "worktree-2" => worktree2_count += 1,
                _ => {}
            }
        }
    }

    // We should have received events from both worktrees
    assert!(worktree1_count > 0, "Should have events from worktree 1");
    assert!(worktree2_count > 0, "Should have events from worktree 2");
    assert_eq!(event_count, worktree1_count + worktree2_count);

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_worktree_lifecycle_during_active_monitoring() {
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;
    let worktree3 = create_test_worktree().await;

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 500,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    // Add initial worktrees
    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    assert_eq!(multi_watcher.watcher_count(), 2);

    // Create file in worktree 1
    create_file(&worktree1, "file1.txt", "content1").await;

    // Add worktree 3 while monitoring
    multi_watcher
        .add_worktree("worktree-3".to_string(), worktree3.path().to_path_buf())
        .await
        .expect("Failed to add worktree 3");

    assert_eq!(multi_watcher.watcher_count(), 3);

    // Create file in worktree 3
    create_file(&worktree3, "file3.txt", "content3").await;

    // Remove worktree 2
    multi_watcher
        .remove_worktree(&"worktree-2".to_string())
        .await
        .expect("Failed to remove worktree 2");

    assert_eq!(multi_watcher.watcher_count(), 2);

    // Create file in worktree 1 after removal
    modify_file(&worktree1, "file1.txt", "modified content").await;

    // Wait for events
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Collect events
    let mut events_by_worktree = HashMap::new();
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            events_by_worktree
                .entry(evt.worktree_id.clone())
                .or_insert_with(Vec::new)
                .push(evt);
        }
    }

    // Should have events from worktree 1 and 3, but not 2 after removal
    assert!(events_by_worktree.contains_key("worktree-1"));
    assert!(events_by_worktree.contains_key("worktree-3"));
    // Note: worktree-2 might have events from before removal, that's OK

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_event_type_isolation() {
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 500,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    // Perform different operations in each worktree
    // Worktree 1: create and modify
    create_file(&worktree1, "file1.txt", "initial").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    modify_file(&worktree1, "file1.txt", "modified").await;

    // Worktree 2: create and delete
    create_file(&worktree2, "file2.txt", "content").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    delete_file(&worktree2, "file2.txt").await;

    // Wait for all events
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Collect events
    let mut worktree1_events = Vec::new();
    let mut worktree2_events = Vec::new();

    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            match evt.worktree_id.as_str() {
                "worktree-1" => worktree1_events.push(evt),
                "worktree-2" => worktree2_events.push(evt),
                _ => {}
            }
        }
    }

    // Verify worktree 1 has modify events
    assert!(worktree1_events
        .iter()
        .any(|e| e.event_type == EventType::Modified));

    // Verify worktree 2 has both modify and delete events
    assert!(worktree2_events
        .iter()
        .any(|e| e.event_type == EventType::Modified));
    assert!(worktree2_events
        .iter()
        .any(|e| e.event_type == EventType::Deleted));

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_watcher_restart_isolation() {
    // This test verifies that restarting one worktree doesn't affect others.
    // With git polling, restart behavior may differ from native file watchers.
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 500,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    // Create file in worktree 1 (before restart)
    create_file(&worktree1, "before_restart.txt", "content").await;

    // Wait for initial event
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Restart worktree 1
    multi_watcher
        .restart_worktree(&"worktree-1".to_string())
        .await
        .expect("Failed to restart worktree 1");

    // Wait for restart to fully initialize (git poller needs to start polling)
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Create file in worktree 2 (which should not be affected by restart)
    create_file(&worktree2, "normal_operation.txt", "content").await;

    // Wait for events (need longer wait with git polling)
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Collect events
    let mut events_by_worktree = HashMap::new();
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(evt) = event {
            events_by_worktree
                .entry(evt.worktree_id.clone())
                .or_insert_with(Vec::new)
                .push(evt);
        }
    }

    // Worktree 2 should still be receiving events (not affected by restart)
    assert!(
        events_by_worktree.contains_key("worktree-2"),
        "Worktree 2 should receive events after worktree 1 restart"
    );

    // Verify worktree 2 received the file event
    let worktree2_events = &events_by_worktree["worktree-2"];
    assert!(worktree2_events
        .iter()
        .any(|e| e.path.ends_with("normal_operation.txt")));

    // Note: worktree 1 may or may not have events depending on restart timing
    // The important thing is worktree 2 isolation was maintained

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_channel_capacity_with_multiple_worktrees() {
    let worktree1 = create_test_worktree().await;
    let worktree2 = create_test_worktree().await;

    // Use small capacity to test buffering
    let config = WatcherConfig {
        debounce_ms: 50,
        channel_capacity: 50,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut multi_watcher, mut rx) = MultiWatcher::new(config);

    multi_watcher
        .add_worktree("worktree-1".to_string(), worktree1.path().to_path_buf())
        .await
        .expect("Failed to add worktree 1");

    multi_watcher
        .add_worktree("worktree-2".to_string(), worktree2.path().to_path_buf())
        .await
        .expect("Failed to add worktree 2");

    // Create many files to test channel capacity
    for i in 0..20 {
        create_file(&worktree1, &format!("file_{}.txt", i), "content").await;
        create_file(&worktree2, &format!("file_{}.txt", i), "content").await;
    }

    // Wait for debouncing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Drain events
    let mut total_events = 0;
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if event.is_some() {
            total_events += 1;
        }
    }

    // We should have received events (exact count depends on debouncing)
    assert!(total_events > 0, "Should have received some events");

    // Clean up
    multi_watcher.shutdown().await.expect("Failed to shutdown");
}
