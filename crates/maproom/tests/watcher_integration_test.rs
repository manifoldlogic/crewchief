//! Integration tests for the file watcher.
//!
//! These tests verify real file system watching with actual file modifications.

use crewchief_maproom::incremental::{FileEvent, FileWatcher, WatcherConfig};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Helper to wait for an event with timeout
async fn wait_for_event(
    rx: &mut tokio::sync::mpsc::Receiver<FileEvent>,
    timeout_duration: Duration,
) -> Option<FileEvent> {
    timeout(timeout_duration, rx.recv()).await.ok().flatten()
}

#[tokio::test]
async fn test_detect_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = WatcherConfig {
        debounce_ms: 100, // Shorter debounce for tests
        channel_capacity: 100,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();

    // Start watching
    watcher.watch(temp_dir.path()).unwrap();

    // Wait a bit for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a new file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Wait for the event (within 2 seconds as per acceptance criteria)
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
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create file before starting watcher
    fs::write(&test_file, "initial content").unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create file before starting watcher
    fs::write(&test_file, "content to delete").unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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
    let temp_dir = TempDir::new().unwrap();
    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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

    // Wait a bit to ensure no events are generated
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that no events were received
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten();
    assert!(
        event.is_none(),
        "Should not receive events for ignored files, but got: {:?}",
        event
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_gitignore_patterns_respected() {
    use std::io::Write;

    let temp_dir = TempDir::new().unwrap();

    // Create .gitignore
    let gitignore_path = temp_dir.path().join(".gitignore");
    let mut gitignore = fs::File::create(&gitignore_path).unwrap();
    writeln!(gitignore, "*.tmp").unwrap();
    writeln!(gitignore, "**/cache").unwrap();
    writeln!(gitignore, "**/cache/**").unwrap();
    gitignore.flush().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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

    // Wait to ensure no events
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should not receive events for ignored files
    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten();
    assert!(
        event.is_none(),
        "Should not receive events for .gitignore patterns, but got: {:?}",
        event
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_debouncing_multiple_modifications() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create file before starting watcher
    fs::write(&test_file, "initial").unwrap();

    let config = WatcherConfig {
        debounce_ms: 300, // Longer debounce for this test
        channel_capacity: 100,
    };

    let (mut watcher, mut rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();
    watcher.watch(temp_dir.path()).unwrap();

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify file multiple times quickly
    for i in 0..5 {
        fs::write(&test_file, format!("content {}", i)).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Wait for debounce period plus buffer
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should receive only one event due to debouncing
    let first_event = wait_for_event(&mut rx, Duration::from_millis(100)).await;
    assert!(first_event.is_some(), "Should receive debounced event");

    // Check if there are additional events (there shouldn't be many)
    let mut event_count = 1;
    while timeout(Duration::from_millis(100), rx.recv())
        .await
        .ok()
        .flatten()
        .is_some()
    {
        event_count += 1;
    }

    // Due to debouncing, we should have significantly fewer events than modifications
    assert!(
        event_count < 5,
        "Debouncing should reduce event count (got {})",
        event_count
    );

    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_multiple_files_independent() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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

    assert_eq!(received_paths.len(), 2, "Should receive events for both files");
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
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("nested").join("deep");
    fs::create_dir_all(&nested_dir).unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    let config = WatcherConfig {
        debounce_ms: 100,
        channel_capacity: 100,
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
