# Quality Strategy: Multi-Agent Concurrency

## Test Philosophy

Focus on testing what can break in multi-agent scenarios. The primary risks are:
1. Race conditions in daemon startup
2. Message framing corruption
3. Concurrent request handling
4. Graceful shutdown with in-flight requests

## Critical Test Paths

### 1. Connect-or-Spawn Race Condition

**Risk**: Two clients try to spawn daemon simultaneously

**Test approach**:
```typescript
test('concurrent connect-or-spawn only creates one daemon', async () => {
    // Launch 5 clients simultaneously
    const connections = await Promise.all([
        connectOrSpawn(config),
        connectOrSpawn(config),
        connectOrSpawn(config),
        connectOrSpawn(config),
        connectOrSpawn(config),
    ]);

    // Verify all connected
    expect(connections.every(c => c.isConnected())).toBe(true);

    // Verify only one daemon process
    const pids = await getDaemonPids();
    expect(pids.length).toBe(1);
});
```

### 2. Message Framing Correctness

**Risk**: Length-prefix misalignment corrupts messages

**Test approach**:
```rust
#[test]
fn length_prefixed_codec_handles_partial_reads() {
    let mut codec = LengthPrefixedCodec::new();
    let message = r#"{"jsonrpc":"2.0","method":"ping","id":1}"#;

    // Encode
    let mut buf = BytesMut::new();
    codec.encode(message.into(), &mut buf).unwrap();

    // Simulate partial reads
    let (first_half, second_half) = buf.split_at(10);

    let mut decoder_buf = BytesMut::new();
    decoder_buf.extend_from_slice(first_half);
    assert!(codec.decode(&mut decoder_buf).unwrap().is_none());

    decoder_buf.extend_from_slice(second_half);
    let decoded = codec.decode(&mut decoder_buf).unwrap().unwrap();
    assert_eq!(decoded, message);
}
```

### 3. Concurrent Request Handling

**Risk**: Responses routed to wrong clients

**Test approach**:
```typescript
test('concurrent requests from multiple clients get correct responses', async () => {
    const client1 = await connectOrSpawn(config);
    const client2 = await connectOrSpawn(config);

    // Send requests concurrently with client-specific data
    const [result1, result2] = await Promise.all([
        client1.search({ query: 'unique-query-1', repo: 'test' }),
        client2.search({ query: 'unique-query-2', repo: 'test' }),
    ]);

    // Verify responses match requests
    expect(result1.query).toBe('unique-query-1');
    expect(result2.query).toBe('unique-query-2');
});
```

### 4. Graceful Shutdown

**Risk**: In-flight requests lost on shutdown

**Test approach**:
```rust
#[tokio::test]
async fn graceful_shutdown_completes_inflight_requests() {
    let server = start_test_server().await;

    // Start a slow request
    let request_future = client.search_slow();

    // Send shutdown signal
    server.shutdown();

    // Verify request completes (not dropped)
    let result = request_future.await;
    assert!(result.is_ok());

    // Verify server is stopped
    assert!(server.is_stopped());
}
```

### 5. SQLite Retry Logic

**Risk**: SQLITE_BUSY not handled correctly

**Test approach**:
```rust
#[tokio::test]
async fn write_with_retry_handles_busy() {
    let store = test_store();

    // Hold a write lock in another connection
    let _lock = store.begin_exclusive_transaction().await;

    // Attempt write with retry
    let start = Instant::now();
    let result = store.write_with_retry(|conn| {
        conn.execute("INSERT INTO test VALUES (1)", [])?;
        Ok(())
    }).await;

    // Should fail after retries (since lock never released)
    assert!(result.is_err());
    assert!(start.elapsed() > Duration::from_millis(750)); // 50+100+200+400
}
```

## Integration Test Suite

### Multi-Process Test Harness

```rust
// tests/integration/multi_agent.rs

#[test]
fn test_multi_agent_concurrent_indexing() {
    let test_repo = setup_test_repo();

    // Start daemon
    let daemon = start_daemon_socket_mode();

    // Spawn 3 agent processes
    let agents: Vec<_> = (0..3)
        .map(|i| {
            spawn_agent_process(AgentConfig {
                worktree: format!("worktree-{}", i),
                daemon_socket: daemon.socket_path(),
            })
        })
        .collect();

    // Each agent indexes its worktree
    for agent in &agents {
        agent.send_command("index");
    }

    // Wait for all to complete
    for agent in &agents {
        let result = agent.wait_for_result();
        assert!(result.is_ok());
    }

    // Verify no errors in daemon logs
    assert!(!daemon.logs_contain("SQLITE_BUSY"));
    assert!(!daemon.logs_contain("error"));
}
```

## Risk-Based Test Priorities

| Test Area | Risk Level | Test Type | Priority |
|-----------|------------|-----------|----------|
| Connect-or-spawn race | High | Integration | P0 |
| Message framing | High | Unit | P0 |
| Request routing | High | Integration | P0 |
| Graceful shutdown | Medium | Integration | P1 |
| SQLite retry | Medium | Unit | P1 |
| Idle timeout | Low | Integration | P2 |
| Memory under load | Low | Stress | P2 |

## Test Environment

### Required Setup

1. **Test database**: Isolated SQLite file per test
2. **Unique socket paths**: `/tmp/maproom-test-{uuid}.sock`
3. **PID file cleanup**: Auto-delete on test completion
4. **Parallel test isolation**: Each test uses unique ports/paths

### CI Configuration

```yaml
# .github/workflows/test.yml
jobs:
  multi-agent-tests:
    runs-on: ubuntu-latest
    steps:
      - name: Run concurrent agent tests
        run: cargo test --test multi_agent -- --test-threads=1
        timeout-minutes: 10
```

## Confidence Criteria

### MVP Ready When

- [ ] Connect-or-spawn race condition test passes
- [ ] 3 concurrent clients can search without errors
- [ ] Graceful shutdown completes in-flight requests
- [ ] No SQLITE_BUSY errors in 10-minute stress test
- [ ] Memory stays under 150MB with 5 clients

### Regression Prevention

- Add multi-agent test to CI gate
- Stress test in nightly builds
- Memory profiling in release process
