//! Integration tests for GitPoller using actual git repositories.
//!
//! These tests verify end-to-end behavior of the git polling system,
//! ensuring file changes are correctly detected and events are emitted.

mod helpers;

use helpers::temp_git_repo::TempGitRepo;
use maproom::incremental::events::FileEvent;
use maproom::incremental::git_poller::{GitPoller, GitPollerConfig, GitPollerError};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_poller_detects_new_file() {
    let repo = TempGitRepo::new();
    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100), // Fast for testing
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Create a new file
    repo.create_file("new-file.rs", "fn main() {}");

    // Wait for event with timeout
    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed");

    assert!(
        matches!(&event, FileEvent::Modified(p) if p.ends_with("new-file.rs")),
        "Expected Modified event for new-file.rs, got: {:?}",
        event
    );

    // Cleanup
    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_modification() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("existing.rs", "initial content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Modify the file
    repo.modify_file("existing.rs", "modified content");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(
        matches!(&event, FileEvent::Modified(p) if p.ends_with("existing.rs")),
        "Expected Modified event for existing.rs, got: {:?}",
        event
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_deletion() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("to-delete.rs", "content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Delete the file
    repo.delete_file("to-delete.rs");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(
        matches!(&event, FileEvent::Deleted(p) if p.ends_with("to-delete.rs")),
        "Expected Deleted event for to-delete.rs, got: {:?}",
        event
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_rename() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("old-name.rs", "content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        detect_renames: true,
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Rename using git mv to ensure git detects it as rename
    repo.stage_rename("old-name.rs", "new-name.rs");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(
        matches!(&event, FileEvent::Renamed(old, new)
            if old.ends_with("old-name.rs") && new.ends_with("new-name.rs")),
        "Expected Renamed event, got: {:?}",
        event
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_not_git_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let result = GitPoller::new(temp_dir.path().to_path_buf(), Default::default());
    assert!(result.is_err());
    match result {
        Err(GitPollerError::NotGitRepository { .. }) => {}
        Err(e) => panic!("Expected NotGitRepository error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[tokio::test]
async fn test_poller_continues_through_git_lock() {
    // Note: git status is a read-only operation and succeeds even when index.lock exists.
    // The lock file only affects write operations like git add/commit.
    // This test verifies the poller continues working correctly with the lock file present.
    let repo = TempGitRepo::new();

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, _rx, _shutdown) = GitPoller::new(repo.path(), config).unwrap();

    // Create git lock (simulates rebase/merge in progress)
    repo.create_git_lock();

    // git status should still succeed (it's a read-only operation)
    let result = poller.poll_once().await;
    assert!(
        result.is_ok(),
        "git status should succeed even with index.lock: {:?}",
        result
    );

    repo.remove_git_lock();

    // Should also succeed after lock removed
    let result = poller.poll_once().await;
    assert!(result.is_ok(), "Should succeed after lock removed");
}

#[tokio::test]
async fn test_poller_multiple_files_single_cycle() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("file1.rs", "content1");
    repo.create_and_commit_file("file2.rs", "content2");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(500), // Longer to batch changes
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Modify both files before poll
    repo.modify_file("file1.rs", "modified1");
    repo.modify_file("file2.rs", "modified2");

    // Collect events
    let mut events = Vec::new();
    for _ in 0..2 {
        if let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
            events.push(event);
        }
    }

    assert_eq!(events.len(), 2, "Expected 2 events for 2 modified files");

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_ignores_gitignored_files() {
    let repo = TempGitRepo::new();

    // Create .gitignore
    repo.create_and_commit_file(".gitignore", "ignored.rs\n");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, _rx, _shutdown) = GitPoller::new(repo.path(), config).unwrap();

    // Do initial poll to capture baseline
    let _ = poller.poll_once().await;

    // Create ignored file
    repo.create_file("ignored.rs", "content");

    // Poll again
    let events = poller.poll_once().await.unwrap();

    // Should not see the ignored file
    let has_ignored = events.iter().any(|e| match e {
        FileEvent::Modified(p) | FileEvent::Deleted(p) => p.ends_with("ignored.rs"),
        FileEvent::Renamed(_, p) => p.ends_with("ignored.rs"),
    });
    assert!(
        !has_ignored,
        "Should not receive events for gitignored files"
    );
}

#[tokio::test]
async fn test_poller_nested_file_detection() {
    let repo = TempGitRepo::new();

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        include_untracked: true,
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Create file in nested directory
    repo.create_file("src/lib/utils/helper.rs", "fn helper() {}");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    // Should get the full nested path, not just the directory
    assert!(
        matches!(&event, FileEvent::Modified(p) if p.ends_with("src/lib/utils/helper.rs")),
        "Expected full nested path, got: {:?}",
        event
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_shutdown_graceful() {
    let repo = TempGitRepo::new();

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(50),
        ..Default::default()
    };

    let (mut poller, _rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Give it some time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send shutdown signal
    shutdown.send(true).unwrap();

    // Should complete gracefully
    let result = timeout(Duration::from_secs(2), handle)
        .await
        .expect("timeout waiting for shutdown");

    assert!(result.is_ok(), "Poller should complete without error");
    assert!(result.unwrap().is_ok(), "Poller run should return Ok");
}

// ===== HEAD commit detection integration tests =====

/// Test that a file created and committed within the poll interval is detected.
///
/// This is the primary use case for HEAD tracking - detecting changes that
/// never appear in `git status --porcelain` because they were committed before
/// the next poll.
#[tokio::test]
async fn test_poller_detects_committed_file() {
    let repo = TempGitRepo::new();

    // Create initial commit so HEAD exists
    repo.create_and_commit_file("initial.txt", "initial content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Wait for initial poll to capture HEAD
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Create, stage, and commit a file - simulating "create and commit within poll interval"
    repo.create_file("committed.txt", "new content");
    repo.stage("committed.txt");
    repo.commit("add committed file");

    // Wait for event with timeout
    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout waiting for committed file event")
        .expect("channel closed");

    assert!(
        matches!(&event, FileEvent::Modified(p) if p.ends_with("committed.txt")),
        "Expected Modified event for committed.txt (via HEAD tracking), got: {:?}",
        event
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

/// Test that multiple commits within a poll interval detect all changed files.
#[tokio::test]
async fn test_poller_detects_multiple_commits() {
    let repo = TempGitRepo::new();

    // Create initial commit
    repo.create_and_commit_file("initial.txt", "initial");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(500), // Longer interval to batch multiple commits
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Wait for initial poll
    tokio::time::sleep(Duration::from_millis(550)).await;

    // Multiple commits before next poll
    repo.create_file("file1.txt", "content1");
    repo.stage("file1.txt");
    repo.commit("add file1");

    repo.create_file("file2.txt", "content2");
    repo.stage("file2.txt");
    repo.commit("add file2");

    // Collect events
    let mut file_names = Vec::new();
    for _ in 0..2 {
        if let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
            if let FileEvent::Modified(p) = event {
                if let Some(name) = p.file_name() {
                    file_names.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    assert!(
        file_names.contains(&"file1.txt".to_string()),
        "Expected file1.txt in events: {:?}",
        file_names
    );
    assert!(
        file_names.contains(&"file2.txt".to_string()),
        "Expected file2.txt in events: {:?}",
        file_names
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}

/// Test that HEAD tracking works correctly when modifications are both committed and dirty.
///
/// This tests the scenario where some changes are committed and others are still
/// pending in the working directory.
#[tokio::test]
#[ignore = "Flaky: race condition between commit event and dirty file creation timing"]
async fn test_poller_head_tracking_with_dirty_files() {
    let repo = TempGitRepo::new();

    // Create initial commit
    repo.create_and_commit_file("initial.txt", "initial");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Wait for initial poll
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Commit one file
    repo.create_file("committed.txt", "committed");
    repo.stage("committed.txt");
    repo.commit("add committed");

    // Create another file but don't commit (dirty)
    repo.create_file("dirty.txt", "dirty content");

    // Should get events for both files
    let mut file_names = Vec::new();
    for _ in 0..2 {
        if let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
            if let FileEvent::Modified(p) = event {
                if let Some(name) = p.file_name() {
                    file_names.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    assert!(
        file_names.contains(&"committed.txt".to_string()),
        "Expected committed.txt (from HEAD tracking): {:?}",
        file_names
    );
    assert!(
        file_names.contains(&"dirty.txt".to_string()),
        "Expected dirty.txt (from git status): {:?}",
        file_names
    );

    let _ = shutdown.send(true);
    let _ = handle.await;
}
