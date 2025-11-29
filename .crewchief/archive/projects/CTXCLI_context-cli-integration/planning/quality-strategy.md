# Quality Strategy: Context CLI Integration

## Overview

This document outlines the testing strategy and quality assurance approach for integrating the context assembler with the CLI and MCP server.

## Testing Pyramid

```
                    ┌─────────────┐
                    │    E2E      │  MCP tool via daemon
                    │   Tests     │  (1-2 tests)
                    ├─────────────┤
                    │ Integration │  CLI + database
                    │   Tests     │  (5-10 tests)
                    ├─────────────┤
                    │   Unit      │  Functions, handlers
                    │   Tests     │  (10-15 tests)
                    └─────────────┘
```

## Unit Tests

### Daemon Types (`daemon/types.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_params_deserialization_minimal() {
        let json = r#"{"chunk_id": "12345"}"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.chunk_id, "12345");
        assert_eq!(params.budget_tokens, 6000); // default
    }

    #[test]
    fn test_context_params_deserialization_full() {
        let json = r#"{
            "chunk_id": "12345",
            "budget_tokens": 4000,
            "expand": {
                "callers": true,
                "callees": true,
                "tests": true,
                "max_depth": 3
            }
        }"#;
        let params: ContextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.budget_tokens, 4000);
        assert!(params.expand.callers);
        assert_eq!(params.expand.max_depth, 3);
    }

    #[test]
    fn test_expand_config_defaults() {
        let config = ExpandConfig::default();
        assert!(!config.callers);
        assert!(!config.callees);
        assert_eq!(config.max_depth, 2);
    }
}
```

### Daemon Handler (`daemon/mod.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_context_invalid_params() {
        // Missing chunk_id should return error
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "context".to_string(),
            params: Some(serde_json::json!({})),
            id: Some(serde_json::json!(1)),
        };
        // ... test error response
    }

    #[tokio::test]
    async fn test_handle_context_chunk_not_found() {
        // Non-existent chunk_id should return -32000 error
    }
}
```

### CLI Argument Parsing (`main.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_context_command_parsing_minimal() {
        let cli = Cli::parse_from(["maproom", "context", "--chunk-id", "12345"]);
        match cli.command {
            Commands::Context { chunk_id, budget, .. } => {
                assert_eq!(chunk_id, 12345);
                assert_eq!(budget, 6000); // default
            }
            _ => panic!("Expected Context command"),
        }
    }

    #[test]
    fn test_context_command_parsing_with_expands() {
        let cli = Cli::parse_from([
            "maproom", "context",
            "--chunk-id", "12345",
            "--budget", "4000",
            "--callers",
            "--callees",
            "--tests",
        ]);
        // ... verify options
    }
}
```

## Integration Tests

### CLI Integration

Test the full CLI command execution against a test database:

```rust
// tests/context_cli_test.rs

#[tokio::test]
async fn test_context_cli_basic() {
    // Setup: Create test database with known chunk
    let db_path = setup_test_db().await;

    // Execute CLI command
    let output = Command::new("cargo")
        .args(["run", "--bin", "crewchief-maproom", "--"])
        .args(["context", "--chunk-id", "1", "--json"])
        .env("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path))
        .output()
        .expect("Failed to execute command");

    // Verify output
    assert!(output.status.success());
    let bundle: ContextBundle = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!bundle.items.is_empty());
    assert_eq!(bundle.items[0].role, "primary");
}

#[tokio::test]
async fn test_context_cli_with_callers() {
    // Test that --callers flag includes caller functions
}

#[tokio::test]
async fn test_context_cli_chunk_not_found() {
    // Test error handling for non-existent chunk
}

#[tokio::test]
async fn test_context_cli_budget_truncation() {
    // Test that large chunks are truncated within budget
}
```

### Daemon Integration

Test JSON-RPC daemon with context method:

```rust
// tests/context_daemon_test.rs

#[tokio::test]
async fn test_daemon_context_method() {
    // Start daemon in subprocess
    let mut child = start_test_daemon().await;

    // Send JSON-RPC request
    let request = r#"{"jsonrpc":"2.0","method":"context","params":{"chunk_id":"1"},"id":1}"#;
    send_to_daemon(&mut child, request).await;

    // Read response
    let response = read_from_daemon(&mut child).await;
    let result: JsonRpcResponse = serde_json::from_str(&response).unwrap();

    assert!(result.error.is_none());
    let bundle: ContextBundle = serde_json::from_value(result.result.unwrap()).unwrap();
    assert!(!bundle.items.is_empty());
}

#[tokio::test]
async fn test_daemon_context_expand_options() {
    // Test expand options are passed through correctly
}
```

## E2E Tests

### MCP Context Tool

Test the full MCP integration:

```typescript
// packages/maproom-mcp/tests/context.e2e.test.ts

describe('MCP Context Tool E2E', () => {
  let daemonProcess: ChildProcess
  let mcpServer: McpServer

  beforeAll(async () => {
    // Start daemon and MCP server
    daemonProcess = await startDaemon()
    mcpServer = await startMcpServer()
  })

  afterAll(async () => {
    await mcpServer.close()
    daemonProcess.kill()
  })

  it('should retrieve context bundle via MCP', async () => {
    const result = await mcpServer.callTool('context', {
      chunk_id: '1',
      budget_tokens: 6000,
      expand: { callers: true },
    })

    expect(result.items).not.toHaveLength(0)
    expect(result.items[0].role).toBe('primary')
    expect(result.total_tokens).toBeLessThanOrEqual(6000)
  })

  it('should handle chunk not found error', async () => {
    const result = await mcpServer.callTool('context', {
      chunk_id: '999999',
    })

    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('CHUNK_NOT_FOUND')
  })
})
```

## Test Data Setup

### Test Database Fixture

> **Owner:** CTXCLI-4001 is responsible for creating this fixture file.

```sql
-- tests/fixtures/context_test.sql
-- Created by: CTXCLI-4001
-- Purpose: Test fixture for daemon context integration tests

-- Insert test repository
INSERT INTO repos (id, name, remote_url) VALUES (1, 'test-repo', 'git@github.com:test/repo.git');

-- Insert test worktree
INSERT INTO worktrees (id, repo_id, name, abs_path) VALUES (1, 1, 'main', '/tmp/test-repo');

-- Insert test file
INSERT INTO files (id, worktree_id, relpath, lang, blob_sha)
VALUES (1, 1, 'src/auth.ts', 'typescript', 'abc123');

-- Insert test chunks (primary function)
INSERT INTO chunks (id, file_id, kind, symbol_name, start_line, end_line, preview, blob_sha)
VALUES (1, 1, 'function', 'authenticate', 10, 30, 'async function authenticate() {...}', 'def456');

-- Insert caller chunk
INSERT INTO chunks (id, file_id, kind, symbol_name, start_line, end_line, preview, blob_sha)
VALUES (2, 1, 'function', 'login', 40, 60, 'function login() { authenticate(); }', 'ghi789');

-- Insert edge (login calls authenticate)
INSERT INTO chunk_edges (from_chunk_id, to_chunk_id, edge_type)
VALUES (2, 1, 'calls');
```

## Performance Requirements

| Operation | Target | Maximum |
|-----------|--------|---------|
| Context assembly (cold) | < 100ms | 500ms |
| Context assembly (cached) | < 10ms | 50ms |
| Daemon startup | < 500ms | 2s |
| MCP tool round-trip | < 200ms | 1s |

## Code Coverage Targets

| Component | Target | Minimum |
|-----------|--------|---------|
| daemon/types.rs | 90% | 80% |
| daemon/mod.rs (context handler) | 85% | 75% |
| main.rs (context command) | 80% | 70% |
| context.ts (MCP tool) | 80% | 70% |

## Acceptance Criteria

### Functional
- [ ] CLI `context` command returns valid JSON bundle
- [ ] CLI `context` command handles missing chunks gracefully
- [ ] Daemon `context` method returns JSON-RPC response
- [ ] MCP `context` tool uses daemon instead of PostgreSQL
- [ ] All expand options work (callers, callees, tests, hooks, etc.)
- [ ] Budget truncation works correctly

### Non-Functional
- [ ] Context assembly < 100ms for typical chunks
- [ ] No memory leaks in daemon with repeated requests
- [ ] Graceful degradation if file not found on disk
- [ ] All tests pass in CI

## CI Integration

Add to GitHub Actions workflow:

```yaml
- name: Test Context CLI
  run: |
    cargo test -p crewchief-maproom context

- name: Test Context Integration
  run: |
    cargo test -p crewchief-maproom --test context_cli_test
    cargo test -p crewchief-maproom --test context_daemon_test

- name: Test MCP Context E2E
  run: |
    cd packages/maproom-mcp
    pnpm test:e2e -- --grep "context"
```
