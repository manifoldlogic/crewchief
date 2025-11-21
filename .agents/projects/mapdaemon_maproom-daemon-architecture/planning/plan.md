# Plan: Maproom Daemon Architecture

## Phase 1: Foundation & Scaffolding
**Goal:** Establish the project structure and type definitions.
- [ ] Create `crates/maproom/src/daemon/` module structure.
- [ ] Define JSON-RPC types (`Request`, `Response`, `Error`) using `serde`.
- [ ] Add `serve` subcommand to `clap` CLI definition in `main.rs`.

## Phase 2: The Event Loop
**Goal:** Implement the listening loop and basic communication.
- [ ] Implement `RpcLoop` struct with `run()` method.
- [ ] Implement `stdin` line reading and `stdout` writing using `tokio`.
- [ ] Implement `ping` method handler.
- [ ] Verify `crewchief-maproom serve` starts and responds to `ping`.

## Phase 3: Vector Search Integration
**Goal:** Connect the daemon to the existing search logic.
- [ ] Define `DaemonState` struct with `PgPool`.
- [ ] Initialize `VectorExecutor` within the daemon state.
- [ ] Implement `search` method handler:
    - Parse params.
    - Execute vector search.
    - Return results.
- [ ] Ensure connection pooling is functioning (single pool init).

## Phase 4: Verification & Polish
**Goal:** Ensure robustness and performance.
- [ ] Create integration test script (`scripts/test-daemon.py` or similar).
- [ ] Verify error handling (invalid JSON, etc.).
- [ ] Verify process exit on stdin close.
- [ ] Benchmark `ping` and `search` latency.

## Agent Assignments
*   **Antigravity:** Lead developer for all phases.
