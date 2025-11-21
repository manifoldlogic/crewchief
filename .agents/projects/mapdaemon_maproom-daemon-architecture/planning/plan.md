# Plan: Maproom Daemon Architecture

## Phase 1: Analysis & Design (Completed)
- [x] Define requirements (Analysis)
- [x] Design protocol (Architecture)

## Phase 2: Implementation
- [ ] **Ticket 1**: Implement JSON-RPC Loop.
    - Create a basic loop that reads JSON lines from stdin and writes to stdout.
- [ ] **Ticket 2**: Implement `serve` command.
    - Wire up the loop to the CLI.
- [ ] **Ticket 3**: Integrate Vector Search.
    - Connect the RPC handlers to the `VectorExecutor` (reusing logic from VECSRCH).
    - Implement connection pooling.

## Phase 3: Verification
- [ ] **Ticket 4**: Performance Benchmarking.
    - Prove the performance gains.

## Dependencies
- Depends on **VECSRCH** for the underlying search logic.
