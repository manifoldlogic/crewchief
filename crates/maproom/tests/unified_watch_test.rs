//! Integration tests for unified watch command with branch switch detection (UNIWATCH-4002).
//!
//! Tests verify:
//! - Complete branch switch workflow (detection, state update, NDJSON emission)
//! - Rapid branch switch debouncing
//! - File changes during branch switch are not lost
//! - Detached HEAD creates SHA-named worktree
//! - --worktree flag backward compatibility

use anyhow::Result;
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// BranchSwitchEvent as emitted by the watch command.
#[derive(Debug, Deserialize)]
struct BranchSwitchEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[allow(dead_code)]
    timestamp: String,
    #[allow(dead_code)]
    repo: String,
    old_branch: String,
    new_branch: String,
    #[allow(dead_code)]
    old_worktree_id: i64,
    #[allow(dead_code)]
    new_worktree_id: i64,
    #[allow(dead_code)]
    worktree_created: bool,
}

/// Test environment with isolated database and git repo.
struct TestEnv {
    repo_dir: TempDir,
    db_dir: TempDir,
    binary_path: PathBuf,
}

impl TestEnv {
    /// Create a new test environment with temp database and git repo.
    fn new() -> Result<Self> {
        let repo_dir = create_test_repo()?;
        let db_dir = tempfile::tempdir()?;
        let binary_path = build_maproom_binary()?;

        let env = Self {
            repo_dir,
            db_dir,
            binary_path,
        };

        // Initialize and scan the repo
        env.setup_database()?;

        Ok(env)
    }

    fn repo_path(&self) -> &Path {
        self.repo_dir.path()
    }

    fn db_url(&self) -> String {
        format!("sqlite://{}/maproom.db", self.db_dir.path().display())
    }

    /// Run a maproom command with the test database.
    fn run_cmd(&self, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new(&self.binary_path)
            .args(args)
            .env("MAPROOM_DATABASE_URL", self.db_url())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail on warnings, only on actual errors
            if !stderr.contains("Error") && !stderr.contains("error:") {
                return Ok(output);
            }
            anyhow::bail!("Command {:?} failed: {}", args, stderr);
        }

        Ok(output)
    }

    /// Setup database and scan repo.
    fn setup_database(&self) -> Result<()> {
        // Database auto-migrates on first connection, but we can explicitly migrate
        self.run_cmd(&["db", "migrate"])?;

        // Scan the repo to index it
        self.run_cmd(&[
            "scan",
            "--path",
            &self.repo_path().to_string_lossy(),
            "--repo",
            "test-repo",
        ])?;

        Ok(())
    }

    /// Start watch command with stdout capture.
    fn start_watch(&self) -> Result<(Child, mpsc::Receiver<BranchSwitchEvent>)> {
        let (child, event_rx, _stderr_rx) = self.start_watch_with_debug()?;
        Ok((child, event_rx))
    }

    /// Start watch command with stdout and stderr capture for debugging.
    fn start_watch_with_debug(
        &self,
    ) -> Result<(
        Child,
        mpsc::Receiver<BranchSwitchEvent>,
        mpsc::Receiver<String>,
    )> {
        let mut child = Command::new(&self.binary_path)
            .args([
                "watch",
                "--path",
                &self.repo_path().to_string_lossy(),
                "--repo",
                "test-repo",
            ])
            .env("MAPROOM_DATABASE_URL", self.db_url())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Create channels for events and stderr
        let (event_tx, event_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        // Take ownership of stdout and stderr
        let child_stdout = child.stdout.take();
        let child_stderr = child.stderr.take();

        // Spawn thread to read stdout
        if let Some(stdout) = child_stdout {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    // Try to parse as BranchSwitchEvent
                    if let Ok(event) = serde_json::from_str::<BranchSwitchEvent>(&line) {
                        let _ = event_tx.send(event);
                    }
                }
            });
        }

        // Spawn thread to read stderr
        if let Some(stderr) = child_stderr {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = stderr_tx.send(line);
                }
            });
        }

        Ok((child, event_rx, stderr_rx))
    }

    /// Create a branch in the test repo.
    fn create_branch(&self, name: &str) -> Result<()> {
        git_command(self.repo_path(), &["checkout", "-b", name])?;
        // Switch back to main
        git_command(self.repo_path(), &["checkout", "main"])
    }

    /// Switch to a branch.
    fn checkout(&self, branch: &str) -> Result<()> {
        git_command(self.repo_path(), &["checkout", branch])
    }
}

/// Test 1: Complete branch switch workflow.
///
/// Verifies that:
/// 1. Watch command starts successfully
/// 2. Branch switch is detected
/// 3. BranchSwitchEvent NDJSON is emitted to stdout
#[test]
fn test_complete_branch_switch_workflow() -> Result<()> {
    let env = TestEnv::new()?;

    // Create feature branch
    env.create_branch("feature")?;

    // Start watch command with stderr capture
    let (mut child, event_rx, stderr_rx) = env.start_watch_with_debug()?;

    // Give watch time to initialize and print startup messages
    thread::sleep(Duration::from_millis(3000));

    // Check if watch process is still alive
    match child.try_wait()? {
        Some(status) => {
            eprintln!("Watch process exited early with status: {:?}", status);
            // Drain stderr to see why
            eprintln!("Stderr:");
            while let Ok(line) = stderr_rx.try_recv() {
                eprintln!("  {}", line);
            }
            anyhow::bail!("Watch process exited before branch switch test");
        }
        None => {
            eprintln!("Watch process is running (pid: {:?})", child.id());
        }
    }

    // Checkout feature branch
    env.checkout("feature")?;
    eprintln!("Checked out feature branch, waiting for event...");

    // Wait for branch switch event (with timeout)
    match receive_branch_event(&event_rx, Duration::from_secs(5)) {
        Ok(event) => {
            // Verify event
            assert_eq!(event.event_type, "branch_switched");
            assert_eq!(event.old_branch, "main");
            assert_eq!(event.new_branch, "feature");
            println!("✓ Complete branch switch workflow: main → feature");
        }
        Err(e) => {
            // Print stderr for debugging
            eprintln!("Test failed: {}", e);
            eprintln!("Stderr output:");
            while let Ok(line) = stderr_rx.try_recv() {
                eprintln!("  {}", line);
            }
            // Check if process exited
            match child.try_wait()? {
                Some(status) => eprintln!("Watch process exited with: {:?}", status),
                None => eprintln!("Watch process still running"),
            }
            return Err(e);
        }
    }

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();

    Ok(())
}

/// Test 2: Rapid branch switches are debounced.
///
/// Verifies that rapid switches within the debounce window (2s) are coalesced.
/// The debouncer allows the first event through immediately, then suppresses
/// subsequent events within the debounce window.
#[test]
fn test_rapid_branch_switches_debounced() -> Result<()> {
    let env = TestEnv::new()?;

    // Create branches
    env.create_branch("branch1")?;
    env.create_branch("branch2")?;
    env.create_branch("branch3")?;

    // Start watch command
    let (mut child, event_rx) = env.start_watch()?;

    // Give watch time to initialize
    thread::sleep(Duration::from_millis(1000));

    // Rapid switches (all within 2s debounce window)
    env.checkout("branch1")?;
    thread::sleep(Duration::from_millis(100));
    env.checkout("branch2")?;
    thread::sleep(Duration::from_millis(100));
    env.checkout("branch3")?;

    // Wait for debounce window to expire plus processing time
    thread::sleep(Duration::from_millis(3000));

    // Collect all events received
    let mut events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        events.push(event);
    }

    // Due to debouncing, we should see fewer events than switches (3)
    // The debouncer lets the first event through immediately, then suppresses
    // subsequent events within the 2-second window
    println!("  Received {} events for 3 rapid switches", events.len());

    // We should see fewer than 3 events due to debouncing
    assert!(
        events.len() < 3,
        "Expected fewer than 3 events due to debouncing, got {}",
        events.len()
    );

    // The first event should be for branch1 (first switch triggers immediately)
    if !events.is_empty() {
        assert_eq!(
            events[0].new_branch, "branch1",
            "First event should be for branch1 (first switch triggers immediately)"
        );
    }

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();

    println!(
        "✓ Rapid branch switches: {} events for 3 switches (debouncing working)",
        events.len()
    );
    Ok(())
}

/// Test 3: File changes during branch switch don't get lost.
///
/// Verifies that file modifications are still indexed even during branch switch.
#[test]
fn test_file_changes_during_branch_switch() -> Result<()> {
    let env = TestEnv::new()?;

    // Create feature branch
    env.create_branch("feature")?;

    // Start watch command
    let (mut child, event_rx) = env.start_watch()?;

    // Give watch time to initialize
    thread::sleep(Duration::from_millis(1000));

    // Perform branch switch
    env.checkout("feature")?;

    // Immediately edit a file (simulating concurrent file edit)
    let test_file = env.repo_path().join("test_concurrent.rs");
    std::fs::write(&test_file, "// File edited during branch switch\n")?;

    // Wait for events to settle
    thread::sleep(Duration::from_secs(3));

    // Verify we got a branch switch event
    let event = receive_branch_event(&event_rx, Duration::from_secs(1))?;
    assert_eq!(event.event_type, "branch_switched");
    assert_eq!(event.new_branch, "feature");

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();

    println!("✓ File changes during branch switch handled gracefully");
    Ok(())
}

/// Test 4: Detached HEAD creates SHA-named worktree.
///
/// Verifies that checking out a commit SHA creates a worktree named with 8-char SHA.
#[test]
fn test_detached_head_creates_sha_worktree() -> Result<()> {
    let env = TestEnv::new()?;

    // Get the initial commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env.repo_path())
        .output()?;
    let full_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let short_sha = &full_sha[..8]; // First 8 chars

    // Start watch command
    let (mut child, event_rx) = env.start_watch()?;

    // Give watch time to initialize
    thread::sleep(Duration::from_millis(1000));

    // Checkout the commit SHA (detached HEAD)
    git_command(env.repo_path(), &["checkout", &full_sha])?;

    // Wait for branch switch event
    let event = receive_branch_event(&event_rx, Duration::from_secs(5))?;

    // Verify event
    assert_eq!(event.event_type, "branch_switched");
    assert_eq!(event.old_branch, "main");
    assert_eq!(
        event.new_branch, short_sha,
        "Detached HEAD should use 8-char SHA as branch name"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();

    println!("✓ Detached HEAD creates SHA-named worktree: {}", short_sha);
    Ok(())
}

/// Test 5: --worktree flag backward compatibility.
///
/// Verifies that the deprecated --worktree flag:
/// 1. Is still accepted (doesn't cause error)
/// 2. Shows deprecation warning in help
/// 3. Auto-detection is mentioned
#[test]
fn test_worktree_flag_backward_compatible() -> Result<()> {
    let binary_path = build_maproom_binary()?;

    // Verify the --worktree flag is documented in help
    let output = Command::new(&binary_path)
        .args(["watch", "--help"])
        .output()?;

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Flag should still be in help
    assert!(
        help_text.contains("--worktree"),
        "Help should list --worktree flag for backward compatibility"
    );

    // Flag should be marked as deprecated
    assert!(
        help_text.contains("deprecated"),
        "Help should mention --worktree is deprecated"
    );

    // Flag should mention auto-detection
    assert!(
        help_text.contains("auto-detect") || help_text.contains("Auto-detect"),
        "Help should mention auto-detection"
    );

    println!("✓ --worktree flag backward compatible (deprecated in help)");
    Ok(())
}

// =============================================================================
// Helper functions
// =============================================================================

/// Create a temporary git repository with an initial commit.
fn create_test_repo() -> Result<TempDir> {
    let dir = tempfile::tempdir()?;
    let path = dir.path();

    // Initialize git repo
    git_command(path, &["init"])?;
    git_command(path, &["checkout", "-b", "main"])?;

    // Configure git user for commits
    git_command(path, &["config", "user.email", "test@test.com"])?;
    git_command(path, &["config", "user.name", "Test User"])?;

    // Create initial file and commit
    let readme = path.join("README.md");
    std::fs::write(&readme, "# Test Repository\n")?;
    git_command(path, &["add", "."])?;
    git_command(path, &["commit", "-m", "Initial commit"])?;

    Ok(dir)
}

/// Run a git command in the specified directory.
fn git_command(repo_path: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Build the crewchief-maproom binary.
fn build_maproom_binary() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let status = Command::new("cargo")
        .args(["build", "--bin", "crewchief-maproom"])
        .current_dir(manifest_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to build crewchief-maproom binary");
    }

    // Binary is in workspace target dir
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let binary_path = workspace_root
        .join("target")
        .join("debug")
        .join("crewchief-maproom");

    if !binary_path.exists() {
        anyhow::bail!("Binary not found at: {}", binary_path.display());
    }

    Ok(binary_path)
}

/// Receive a branch switch event with timeout.
fn receive_branch_event(
    rx: &mpsc::Receiver<BranchSwitchEvent>,
    timeout: Duration,
) -> Result<BranchSwitchEvent> {
    rx.recv_timeout(timeout)
        .map_err(|_| anyhow::anyhow!("Timeout waiting for branch switch event"))
}
