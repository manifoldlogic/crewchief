# Quality Strategy: Maproom Daemon

## Testing Approach
The primary goal is to ensure the daemon is robust, performant, and adheres strictly to the JSON-RPC protocol.

### 1. Unit Testing
*   **Protocol Parsing:** Test serialization and deserialization of JSON-RPC requests and responses.
*   **Handler Logic:** Test individual method handlers (`ping`, `search`) in isolation, mocking the database if necessary (or using a test DB).

### 2. Integration Testing (Black Box)
We will treat the binary as a black box and test its IO behavior.

*   **Test Harness:** A script (Python or Rust) that:
    1.  Spawns `crewchief-maproom serve`.
    2.  Sends a sequence of JSON-RPC messages to stdin.
    3.  Reads stdout and asserts on the responses.
    4.  Checks stderr for logs.
*   **Scenarios:**
    *   **Happy Path:** `ping` returns `pong`. `search` returns results.
    *   **Error Handling:** Send invalid JSON. Send unknown method. Verify standard JSON-RPC error codes.
    *   **Lifecycle:** Close stdin. Verify process exits with code 0.
    *   **Concurrency:** Send multiple requests rapidly. Verify all responses are received (order may vary if async, but `id` must match).

### 3. Performance Testing
*   **Latency:** Measure the time from writing a request to reading the response.
    *   Target: `ping` < 1ms (excluding process start).
    *   Target: `search` < 50ms (for cached/warm queries).
*   **Throughput:** Send a burst of requests. Ensure no memory leaks or crashes.

### 4. Risk Mitigation
*   **Stdout Pollution:** This is the biggest risk. Any `println!` or logging to stdout will break the client.
    *   **Test:** Configure logging to `info` or `debug` (via `RUST_LOG` env var) and run the integration test.
    *   **Assertion:** Parse every line of stdout as JSON. If parsing fails, the test fails. Assert that stdout *only* contains valid JSON-RPC responses.
*   **Orphaned Processes:**
    *   **Test:** Start the daemon, then close the stdin pipe from the test harness.
    *   **Assertion:** Wait 1 second. Check if the daemon process ID is still running. It MUST be gone.
    *   **Test:** Send `SIGPIPE` to the daemon's stdout (simulate reader death). Daemon should exit.

## MVP Testing Checklist
- [ ] Unit tests for JSON-RPC types.
- [ ] Integration test script (simple python or shell script).
- [ ] Verification that `ping` works.
- [ ] Verification that `search` connects to DB and returns vectors.
- [ ] Verification that process exits on stdin EOF.
