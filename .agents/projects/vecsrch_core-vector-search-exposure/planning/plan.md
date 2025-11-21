# Plan: Core Vector Search Exposure

## Phase 1: Analysis & Design (Completed)
- [x] Define requirements (Analysis)
- [x] Design CLI interface (Architecture)

## Phase 2: Implementation
- [ ] **Ticket 1**: Expose `VectorExecutor` types to CLI module.
    - Modify `crates/maproom/src/lib.rs` (or equivalent) to make necessary types public.
- [ ] **Ticket 2**: Implement CLI command definition.
    - Update `crates/maproom/src/cli/mod.rs` (or equivalent) with new clap struct/enum.
- [ ] **Ticket 3**: Implement command handler.
    - Write the logic to instantiate `VectorExecutor` and run the search.
    - Format output as JSON.

## Phase 3: Verification
- [ ] **Ticket 4**: Integration Testing.
    - Create a test script that seeds a DB, runs the CLI, and asserts on output.

## Resources
- **Agents**: Rust Developer (for implementation), QA Specialist (for testing).
