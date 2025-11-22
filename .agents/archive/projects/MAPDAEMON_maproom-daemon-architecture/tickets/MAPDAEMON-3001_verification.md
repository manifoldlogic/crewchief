# MAPDAEMON-3001: Verification & Polish

**Status:** Open
**Phase:** 4 (Verification)
**Estimated Effort:** 60 minutes
**Priority:** Medium

---

## Summary
Verify the daemon implementation with integration tests and benchmarks. Ensure robustness against edge cases and proper resource cleanup.

---

## Background
We have a working daemon, but we need to prove it is robust enough for production use. This involves testing edge cases (bad input, broken pipes) and measuring the actual performance gain compared to the CLI.

---

## Acceptance Criteria
1.  ✅ Integration test script passes (happy path + error cases).
2.  ✅ Daemon exits cleanly when stdin is closed.
3.  ✅ No zombie processes left behind.
4.  ✅ Benchmark shows `ping` latency < 1ms.
5.  ✅ Benchmark shows `search` latency < 50ms (warm).

---

## Technical Requirements

### 1. Integration Test Script
Create `scripts/test-daemon.py` (or similar).
*   Start process.
*   Send `ping`. Assert `pong`.
*   Send `search`. Assert results.
*   Send malformed JSON. Assert error.
*   Close stdin. Wait for exit. Assert exit code 0.

### 2. Benchmarking
*   Measure time difference between write and read.
*   Compare against `time crewchief-maproom search ...`.

### 3. Cleanup
*   Ensure `Ctrl+C` works (SIGINT).
*   Ensure `kill` works (SIGTERM).

---

## Implementation Steps
1.  Write the python test script.
2.  Run the script and fix any bugs found in the daemon.
3.  Run manual benchmarks and record results in the ticket comments.
4.  Polish code (clippy, formatting).

---

## Verification
*   Run the test script.
*   Review benchmark results.
