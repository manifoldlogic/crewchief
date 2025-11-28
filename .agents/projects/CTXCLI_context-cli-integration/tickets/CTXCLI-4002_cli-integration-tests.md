# Ticket: CTXCLI-4002: Add CLI Context Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add integration tests for the CLI context command, testing full command execution against a test database.

## Background
This is Phase 4 (Testing & Polish). The CLI context command needs integration tests to verify argument parsing, command execution, and output format. These tests complement the unit tests in main.rs.

Reference: [planning/quality-strategy.md](../planning/quality-strategy.md) - CLI Integration section

## Acceptance Criteria
- [ ] Test: basic command `context --chunk-id 1 --json` returns valid JSON
- [ ] Test: all CLI arguments parsed correctly
- [ ] Test: `--json` flag produces valid JSON output
- [ ] Test: without `--json`, human-readable output is produced
- [ ] Test: chunk not found error message is helpful
- [ ] Test: database connection error handled gracefully
- [ ] All CLI options tested (callers, callees, tests, docs, config, max-depth)
- [ ] Tests pass in CI

## Technical Requirements
- Use `std::process::Command` to run CLI binary
- Set `MAPROOM_DATABASE_URL` env var to test database
- Parse stdout/stderr for assertions
- Reuse test fixture from CTXCLI-4001

## Implementation Notes

### Test Cases
```rust
// tests/context_cli_test.rs

#[tokio::test]
async fn test_context_cli_basic() {
    let db_path = setup_test_db().await;

    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "1", "--json"])
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let bundle: ContextBundle = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!bundle.items.is_empty());
    assert_eq!(bundle.items[0].role, "primary");
}

#[tokio::test]
async fn test_context_cli_with_callers() {
    let db_path = setup_test_db().await;

    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "1", "--callers", "--json"])
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let bundle: ContextBundle = serde_json::from_slice(&output.stdout).unwrap();

    // Should have primary + caller items
    assert!(bundle.items.iter().any(|i| i.role == "caller"));
}

#[tokio::test]
async fn test_context_cli_chunk_not_found() {
    let db_path = setup_test_db().await;

    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "999999", "--json"])
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("Chunk"));
}

#[tokio::test]
async fn test_context_cli_budget_truncation() {
    let db_path = setup_test_db().await;

    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "1", "--budget", "100", "--json"])
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let bundle: ContextBundle = serde_json::from_slice(&output.stdout).unwrap();
    assert!(bundle.total_tokens <= 100 || bundle.truncated);
}

#[tokio::test]
async fn test_context_cli_human_readable_output() {
    let db_path = setup_test_db().await;

    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "1"])  // No --json flag
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain human-readable elements
    assert!(stdout.contains("Context Bundle") || stdout.contains("PRIMARY"));
    assert!(stdout.contains("tokens"));
}
```

### Test Helper
```rust
async fn setup_test_db() -> String {
    // Create temp database file
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Run migrations
    // ...

    // Load fixture
    let fixture = include_str!("fixtures/context_test.sql");
    // Execute SQL...

    db_path.to_string_lossy().to_string()
}
```

## Dependencies
- CTXCLI-2003 (CLI human-readable output must be implemented)
- CTXCLI-4001 (Test fixture must exist)

## Risk Assessment
- **Risk**: Binary not built when running tests
  - **Mitigation**: Use `cargo run` which builds if needed, or ensure build step
- **Risk**: Temp database file cleanup issues
  - **Mitigation**: Use `tempfile` crate with auto-cleanup

## Files/Packages Affected
- `crates/maproom/tests/context_cli_test.rs` (create)
