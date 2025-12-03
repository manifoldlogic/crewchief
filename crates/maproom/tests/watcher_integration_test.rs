//! Integration tests for the file watcher.
//!
//! These tests verify real file system watching with actual file modifications.
//!
//! Note: FileWatcher now uses git status polling internally, so all tests
//! require a git repository. The git polling approach eliminates "too many
//! open files" errors on large repositories.

use crewchief_maproom::incremental::{FileEvent, FileWatcher, WatcherConfig};
use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Helper to create a temporary git repository.
fn create_temp_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
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

/// Helper to wait for an event with timeout
async fn wait_for_event(
    rx: &mut tokio::sync::mpsc::Receiver<FileEvent>,
    timeout_duration: Duration,
) -> Option<FileEvent> {
    timeout(timeout_duration, rx.recv()).await.ok().flatten()
}

#[tokio::test]
async fn test_detect_file_creation() {
    let temp_dir = create_temp_git_repo();
    let config = WatcherConfig {
        debounce_ms: 100, // Shorter debounce for tests (unused with git polling)
        channel_capacity: 100,
        poll_interval_ms: 200, // Fast polling for tests
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();

    // Start watching (no-op with git polling, starts on creation)
    watcher.watch(temp_dir.path()).unwrap();

    // Wait a bit for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a new file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Wait for the event (git polling may take up to poll_interval)
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;

    assert!(event.is_some(), "Should receive file creation event");
    let event = event.unwrap();

    match event {
        FileEvent::Modified(path) => {
            assert_eq!(path, test_file, "Event path should match created file");
        }
        _ => panic!("Expected Modified event, got {:?}", event),
    }

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_detect_file_modification() {
    let temp_dir = create_temp_git_repo();
    let test_file = temp_dir.path().join("test.txt");

    // Create file and commit to track modifications
    fs::write(&test_file, "initial content").unwrap();
    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify the file
    fs::write(&test_file, "modified content").unwrap();

    // Wait for the event
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;

    assert!(event.is_some(), "Should receive file modification event");
    let event = event.unwrap();

    match event {
        FileEvent::Modified(path) => {
            assert_eq!(path, test_file, "Event path should match modified file");
        }
        _ => panic!("Expected Modified event, got {:?}", event),
    }

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_detect_file_deletion() {
    let temp_dir = create_temp_git_repo();
    let test_file = temp_dir.path().join("test.txt");

    // Create file and commit so deletion is trackable
    fs::write(&test_file, "content to delete").unwrap();
    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Delete the file
    fs::remove_file(&test_file).unwrap();

    // Wait for the event
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;

    assert!(event.is_some(), "Should receive file deletion event");
    let event = event.unwrap();

    match event {
        FileEvent::Deleted(path) => {
            assert_eq!(path, test_file, "Event path should match deleted file");
        }
        _ => panic!("Expected Deleted event, got {:?}", event),
    }

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_ignore_patterns_respected() {
    // Note: With git polling, git already ignores node_modules etc if .gitignore is present.
    // This test verifies that git's built-in ignore patterns work correctly.
    let temp_dir = create_temp_git_repo();

    // Create .gitignore with ignore patterns
    fs::write(temp_dir.path().join(".gitignore"), "*.log\nnode_modules/\n").unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create ignored files
    fs::write(temp_dir.path().join("debug.log"), "log content").unwrap();

    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    fs::write(node_modules.join("package.json"), "{}").unwrap();

    // Wait for poll cycle to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that no events were received for gitignored files
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten();

    // Filter out .gitignore itself if we see it
    let relevant_event = if let Some(ref e) = event {
        if e.path().to_string_lossy().contains(".gitignore") {
            // Drain any more events then check for ignored files
            tokio::time::sleep(Duration::from_millis(300)).await;
            timeout(Duration::from_millis(100), rx.recv())
                .await
                .ok()
                .flatten()
        } else {
            event
        }
    } else {
        event
    };

    assert!(
        relevant_event.is_none()
            || !relevant_event
                .as_ref()
                .unwrap()
                .path()
                .to_string_lossy()
                .contains("debug.log"),
        "Should not receive events for ignored files, but got: {:?}",
        relevant_event
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_gitignore_patterns_respected() {
    use std::io::Write;

    let temp_dir = create_temp_git_repo();

    // Create .gitignore
    let gitignore_path = temp_dir.path().join(".gitignore");
    let mut gitignore = fs::File::create(&gitignore_path).unwrap();
    writeln!(gitignore, "*.tmp").unwrap();
    writeln!(gitignore, "cache/").unwrap();
    gitignore.flush().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create ignored file
    fs::write(temp_dir.path().join("test.tmp"), "temp content").unwrap();

    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("data.json"), "{}").unwrap();

    // Wait for poll cycle
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Drain all events and check none are for gitignored files
    let mut received_ignored = false;
    let mut all_events = Vec::new();
    while let Ok(Some(event)) = timeout(Duration::from_millis(100), rx.recv()).await {
        let event_path = event.path();
        all_events.push(event_path.display().to_string());
        // Check if the filename ends with .tmp or if the path contains /cache/
        if let Some(file_name) = event_path.file_name() {
            let name = file_name.to_string_lossy();
            if name.ends_with(".tmp") || name == "data.json" {
                received_ignored = true;
            }
        }
        // Also check for cache directory in the path
        if event_path.to_string_lossy().contains("/cache/") {
            received_ignored = true;
        }
    }

    assert!(
        !received_ignored,
        "Should not receive events for .gitignore patterns, but got: {:?}",
        all_events
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_debouncing_multiple_modifications() {
    // Note: With git polling, debouncing is implicit in the poll interval.
    // This test verifies that multiple rapid modifications result in a single event.
    let temp_dir = create_temp_git_repo();
    let test_file = temp_dir.path().join("test.txt");

    // Create file and commit
    fs::write(&test_file, "initial").unwrap();
    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    let config = WatcherConfig {
        debounce_ms: 300, // Unused with git polling
        channel_capacity: 100,
        poll_interval_ms: 500, // Single poll covers all modifications
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify file multiple times quickly (within single poll interval)
    for i in 0..5 {
        fs::write(&test_file, format!("content {}", i)).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Wait for poll cycle
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Should receive at least one event
    let first_event = wait_for_event(&mut rx, Duration::from_millis(100)).await;
    assert!(first_event.is_some(), "Should receive event");

    // Check if there are additional events
    let mut event_count = 1;
    while timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten()
        .is_some()
    {
        event_count += 1;
    }

    // With git polling, multiple rapid modifications within a poll interval
    // result in a single Modified event (git sees the final state)
    assert!(
        event_count <= 2,
        "Git polling should consolidate rapid modifications (got {} events)",
        event_count
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_multiple_files_independent() {
    let temp_dir = create_temp_git_repo();
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create two files
    fs::write(&file1, "content 1").unwrap();
    fs::write(&file2, "content 2").unwrap();

    // Should receive events for both files
    let mut received_paths = Vec::new();

    for _ in 0..2 {
        if let Some(event) = wait_for_event(&mut rx, Duration::from_secs(2)).await {
            received_paths.push(event.path().clone());
        }
    }

    assert_eq!(
        received_paths.len(),
        2,
        "Should receive events for both files"
    );
    assert!(
        received_paths.contains(&file1),
        "Should receive event for file1"
    );
    assert!(
        received_paths.contains(&file2),
        "Should receive event for file2"
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_nested_directory_watching() {
    let temp_dir = create_temp_git_repo();
    let nested_dir = temp_dir.path().join("nested").join("deep");
    fs::create_dir_all(&nested_dir).unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create file in nested directory
    let nested_file = nested_dir.join("file.txt");
    fs::write(&nested_file, "nested content").unwrap();

    // Should receive event for nested file
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;
    assert!(
        event.is_some(),
        "Should receive event for file in nested directory"
    );

    let event = event.unwrap();
    match event {
        FileEvent::Modified(path) => {
            assert_eq!(
                path, nested_file,
                "Event path should match nested file location"
            );
        }
        _ => panic!("Expected Modified event for nested file"),
    }

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_watcher_stop_and_restart() {
    let temp_dir = create_temp_git_repo();
    let test_file = temp_dir.path().join("test.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 200,
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a file
    fs::write(&test_file, "content 1").unwrap();

    // Should receive event
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;
    assert!(event.is_some(), "Should receive event while watching");

    // Stop watching
    watcher.stop().unwrap();

    // Modify file while stopped
    fs::write(&test_file, "content 2").unwrap();

    // Wait to ensure no event is generated
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should not receive event while stopped
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten();
    assert!(
        event.is_none(),
        "Should not receive events when watcher is stopped"
    );
}

#[tokio::test]
async fn test_event_timing_within_2_seconds() {
    let temp_dir = create_temp_git_repo();
    let test_file = temp_dir.path().join("test.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
        poll_interval_ms: 500, // 0.5s polling for reasonable latency
        include_untracked: true,
        detect_renames: true,
        git_timeout_ms: 10000,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Record start time
    let start = std::time::Instant::now();

    // Create file
    fs::write(&test_file, "test content").unwrap();

    // Wait for event with 2 second timeout (per acceptance criteria)
    let event = wait_for_event(&mut rx, Duration::from_secs(2)).await;

    let elapsed = start.elapsed();

    assert!(event.is_some(), "Should receive event within 2 seconds");
    assert!(
        elapsed < Duration::from_secs(2),
        "Event should arrive within 2 seconds (took {:?})",
        elapsed
    );

    watcher.stop().unwrap();
}
