//! Unit tests for the file watcher component.
//!
//! These tests verify:
//! - Ignore pattern matching
//! - Event type conversions
//! - Watcher configuration
//! - Git polling integration

use maproom::incremental::{FileEvent, FileWatcher, IgnorePatternMatcher, WatcherConfig};
use std::path::PathBuf;
use tempfile::TempDir;

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

#[test]
fn test_file_event_path_extraction() {
    let path = PathBuf::from("/test/file.txt");
    let event = FileEvent::Modified(path.clone());
    assert_eq!(event.path(), &path);

    let old_path = PathBuf::from("/test/old.txt");
    let new_path = PathBuf::from("/test/new.txt");
    let rename_event = FileEvent::Renamed(old_path.clone(), new_path.clone());
    assert_eq!(rename_event.path(), &new_path);

    let delete_event = FileEvent::Deleted(path.clone());
    assert_eq!(delete_event.path(), &path);
}

#[test]
fn test_file_event_paths_extraction() {
    let path = PathBuf::from("/test/file.txt");
    let event = FileEvent::Modified(path.clone());
    assert_eq!(event.paths(), vec![&path]);

    let old_path = PathBuf::from("/test/old.txt");
    let new_path = PathBuf::from("/test/new.txt");
    let rename_event = FileEvent::Renamed(old_path.clone(), new_path.clone());
    assert_eq!(rename_event.paths(), vec![&old_path, &new_path]);
}

#[test]
fn test_ignore_pattern_default() {
    let matcher = IgnorePatternMatcher::new().unwrap();

    // Test default ignore patterns
    assert!(matcher.should_ignore(&PathBuf::from("debug.log")));
    assert!(matcher.should_ignore(&PathBuf::from(".git/HEAD")));
    assert!(matcher.should_ignore(&PathBuf::from("node_modules/lodash/index.js")));
    assert!(matcher.should_ignore(&PathBuf::from("dist/bundle.js")));
    assert!(matcher.should_ignore(&PathBuf::from("target/debug/binary")));
    assert!(matcher.should_ignore(&PathBuf::from(".crewchief/agent/state.json")));

    // Test files that should NOT be ignored
    assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
    assert!(!matcher.should_ignore(&PathBuf::from("package.json")));
    assert!(!matcher.should_ignore(&PathBuf::from("Cargo.toml")));
    assert!(!matcher.should_ignore(&PathBuf::from("README.md")));
}

#[test]
fn test_ignore_pattern_custom() {
    let patterns = vec!["*.tmp", "*.bak", "cache/**"];
    let matcher = IgnorePatternMatcher::with_patterns(patterns).unwrap();

    assert!(matcher.should_ignore(&PathBuf::from("test.tmp")));
    assert!(matcher.should_ignore(&PathBuf::from("backup.bak")));
    assert!(matcher.should_ignore(&PathBuf::from("cache/data.json")));

    assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
}

#[test]
fn test_ignore_pattern_should_watch() {
    let matcher = IgnorePatternMatcher::new().unwrap();

    assert!(matcher.should_watch(&PathBuf::from("src/lib.rs")));
    assert!(!matcher.should_watch(&PathBuf::from("node_modules/pkg/index.js")));
}

#[test]
fn test_ignore_pattern_from_gitignore() {
    use std::io::Write;

    let temp_dir = TempDir::new().unwrap();
    let gitignore_path = temp_dir.path().join(".gitignore");

    let mut file = std::fs::File::create(&gitignore_path).unwrap();
    writeln!(file, "# Comment line").unwrap();
    writeln!(file, "*.tmp").unwrap();
    writeln!(file, "").unwrap();
    writeln!(file, "build/**").unwrap();
    writeln!(file, "# Another comment").unwrap();
    writeln!(file, "*.cache").unwrap();
    file.flush().unwrap();

    let matcher = IgnorePatternMatcher::from_gitignore(&gitignore_path).unwrap();

    // Custom patterns from .gitignore
    assert!(matcher.should_ignore(&PathBuf::from("test.tmp")));
    assert!(matcher.should_ignore(&PathBuf::from("build/output.js")));
    assert!(matcher.should_ignore(&PathBuf::from("data.cache")));

    // Default patterns still work
    assert!(matcher.should_ignore(&PathBuf::from("node_modules/pkg/index.js")));
    assert!(matcher.should_ignore(&PathBuf::from(".git/config")));

    // Normal files not ignored
    assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
}

#[test]
fn test_ignore_pattern_gitignore_not_found() {
    // Should fall back to default patterns
    let matcher =
        IgnorePatternMatcher::from_gitignore(&PathBuf::from("/nonexistent/.gitignore")).unwrap();

    // Default patterns should still work
    assert!(matcher.should_ignore(&PathBuf::from("node_modules/pkg/index.js")));
    assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
}

#[test]
fn test_watcher_config_default() {
    let config = WatcherConfig::default();
    assert_eq!(config.debounce_ms, 500);
    assert_eq!(config.channel_capacity, 1000);
    assert_eq!(config.poll_interval_ms, 3000);
    assert!(config.include_untracked);
    assert!(config.detect_renames);
    assert_eq!(config.git_timeout_ms, 10000);
}

#[test]
fn test_watcher_config_custom() {
    let config = WatcherConfig {
        debounce_ms: 1000,
        channel_capacity: 500,
        poll_interval_ms: 5000,
        include_untracked: false,
        detect_renames: false,
        git_timeout_ms: 20000,
    };
    assert_eq!(config.debounce_ms, 1000);
    assert_eq!(config.channel_capacity, 500);
    assert_eq!(config.poll_interval_ms, 5000);
    assert!(!config.include_untracked);
    assert!(!config.detect_renames);
    assert_eq!(config.git_timeout_ms, 20000);
}

#[tokio::test]
async fn test_file_watcher_creation() {
    // FileWatcher now requires a git repository
    let temp_dir = create_temp_git_repo();
    let config = WatcherConfig::default();

    let result = FileWatcher::new(temp_dir.path().to_path_buf(), config);
    assert!(result.is_ok());

    let (mut watcher, _rx) = result.unwrap();
    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_file_watcher_rejects_non_git_dir() {
    // Non-git directories should be rejected
    let temp_dir = TempDir::new().unwrap();
    let config = WatcherConfig::default();

    let result = FileWatcher::new(temp_dir.path().to_path_buf(), config);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_watcher_with_gitignore() {
    use std::io::Write;

    let temp_dir = create_temp_git_repo();
    let gitignore_path = temp_dir.path().join(".gitignore");

    let mut file = std::fs::File::create(&gitignore_path).unwrap();
    writeln!(file, "*.tmp").unwrap();
    file.flush().unwrap();

    let config = WatcherConfig::default();
    let result = FileWatcher::new(temp_dir.path().to_path_buf(), config);
    assert!(result.is_ok());

    let (mut watcher, _rx) = result.unwrap();
    watcher.stop().unwrap();
}

#[tokio::test]
async fn test_file_watcher_start_stop() {
    let temp_dir = create_temp_git_repo();
    let config = WatcherConfig::default();

    let (mut watcher, _rx) = FileWatcher::new(temp_dir.path().to_path_buf(), config).unwrap();

    // watch() is now a no-op with git polling (polling starts on creation)
    let result = watcher.watch(temp_dir.path());
    assert!(result.is_ok());

    // Stop watching
    let result = watcher.stop();
    assert!(result.is_ok());
}

#[test]
fn test_event_equality() {
    let path1 = PathBuf::from("/test/file.txt");
    let path2 = PathBuf::from("/test/file.txt");
    let path3 = PathBuf::from("/test/other.txt");

    let event1 = FileEvent::Modified(path1.clone());
    let event2 = FileEvent::Modified(path2.clone());
    let event3 = FileEvent::Modified(path3.clone());

    assert_eq!(event1, event2);
    assert_ne!(event1, event3);
}

#[test]
fn test_event_types() {
    let path = PathBuf::from("/test/file.txt");

    let modified = FileEvent::Modified(path.clone());
    let deleted = FileEvent::Deleted(path.clone());
    let renamed = FileEvent::Renamed(path.clone(), PathBuf::from("/test/new.txt"));

    assert_ne!(modified, deleted);
    assert_ne!(modified, renamed);
    assert_ne!(deleted, renamed);
}

#[test]
fn test_ignore_pattern_edge_cases() {
    let matcher = IgnorePatternMatcher::new().unwrap();

    // Test nested paths
    assert!(matcher.should_ignore(&PathBuf::from("deep/nested/node_modules/pkg/index.js")));
    assert!(matcher.should_ignore(&PathBuf::from("a/b/c/target/release/binary")));

    // Test file extensions
    assert!(matcher.should_ignore(&PathBuf::from("debug.log")));
    assert!(matcher.should_ignore(&PathBuf::from("path/to/error.log")));

    // Test hidden files
    assert!(matcher.should_ignore(&PathBuf::from(".DS_Store")));
    assert!(matcher.should_ignore(&PathBuf::from("folder/.DS_Store")));
}

#[test]
fn test_multiple_ignore_patterns() {
    let patterns = vec!["*.log", "*.tmp", "build/**", "cache/**", "*.bak", ".env*"];
    let matcher = IgnorePatternMatcher::with_patterns(patterns).unwrap();

    assert!(matcher.should_ignore(&PathBuf::from("debug.log")));
    assert!(matcher.should_ignore(&PathBuf::from("temp.tmp")));
    assert!(matcher.should_ignore(&PathBuf::from("build/output.js")));
    assert!(matcher.should_ignore(&PathBuf::from("cache/data.json")));
    assert!(matcher.should_ignore(&PathBuf::from("config.bak")));
    assert!(matcher.should_ignore(&PathBuf::from(".env")));
    assert!(matcher.should_ignore(&PathBuf::from(".env.local")));

    assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
    assert!(!matcher.should_ignore(&PathBuf::from("README.md")));
}
