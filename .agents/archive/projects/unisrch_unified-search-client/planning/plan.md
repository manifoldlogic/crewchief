# Plan: Unified Search Client

## Phase 1: Analysis & Design (Completed)
- [x] Define requirements (Analysis)
- [x] Design delegation architecture (Architecture)

## Phase 2: Implementation
- [ ] **Ticket 1**: Clean up legacy code.
    - Remove old search implementation from `packages/maproom-mcp`.
- [ ] **Ticket 2**: Implement Rust CLI wrapper.
    - Create a service/utility in MCP to call the Rust binary safely.
- [ ] **Ticket 3**: Update MCP Tool definition.
    - Wire the `search` tool to use the new wrapper.

## Phase 3: Verification
- [ ] **Ticket 4**: End-to-End Testing.
    - Verify an agent can successfully search the codebase via MCP.

## Dependencies
- Depends on **VECSRCH** being completed (or at least the CLI interface defined).
